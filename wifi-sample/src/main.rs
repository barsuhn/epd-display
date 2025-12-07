//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to specified Wifi network and creates a TCP endpoint on port 1234.

#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use core::str::from_utf8;

use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, StackResources};
use embassy_rp::{bind_interrupts, Peri};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::dma::Channel;
use embassy_rp::pio::{InterruptHandler, Pio, PioPin, Instance as PioInstance};
use embassy_rp::clocks::RoscRng;
use embassy_time::Duration;
use cyw43::{Control, NetDriver};
use embedded_io_async::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = dotenvy_macro::dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenvy_macro::dotenv!("WIFI_PASSWORD");

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

struct WifiPio<PIO: PioInstance + 'static> {
    pub cs: Output<'static>,
    pub pio: Pio<'static, PIO>,
}

impl<PIO: PioInstance + 'static> WifiPio<PIO> {
    fn new(cs_pin: Peri<'static, impl Pin>, pio_instance: Peri<'static, PIO>, irq: impl Binding<PIO::Interrupt, InterruptHandler<PIO>>) -> Self {
        WifiPio {
            cs: Output::new(cs_pin, Level::High),
            pio: Pio::new(pio_instance, irq)
        }
    }

    #[allow(unused)]
    fn spi<DMA: Channel>(mut self: Self, dio_pin: Peri<'static, impl PioPin>, clk_pin: Peri<'static, impl PioPin>, dma_channel: Peri<'static, DMA>) -> PioSpi<'static, PIO, 0, DMA> {
        PioSpi::new(
            &mut self.pio.common,
            self.pio.sm0,
            DEFAULT_CLOCK_DIVIDER,
            self.pio.irq0,
            self.cs,
            dio_pin,
            clk_pin,
            dma_channel,
        )
    }
}

macro_rules! wifi_spi {
    ($wifi_pio:expr, $dio_pin:expr, $clk_pin:expr, $dma_channel:expr) => {
        {
            PioSpi::new(
                &mut $wifi_pio.pio.common,
                $wifi_pio.pio.sm0,
                DEFAULT_CLOCK_DIVIDER,
                $wifi_pio.pio.irq0,
                $wifi_pio.cs,
                $dio_pin,
                $clk_pin,
                $dma_channel,
            )
        }
    }
}

async fn init_cyw43(spawner: Spawner, spi: PioSpi<'static, PIO0, 0, DMA_CH0>, pwr: Output<'static>) -> (Control<'static>, NetDriver<'static>) {
    info!("Init Cyw43");

    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    match spawner.spawn(cyw43_task(runner)) {
        Ok(_) => info!("cyw43 task running"),
        Err(_) => info!("cyw43 task failed"),
    }

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    (control, net_device)
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let pwr = Output::new(p.PIN_23, Level::Low);
    let mut wio = WifiPio::new(p.PIN_25, p.PIO0, Irqs);
    // let spi = wio.spi(p.PIN_24, p.PIN_29, p.DMA_CH0);
    let spi = wifi_spi!(wio, p.PIN_24, p.PIN_29, p.DMA_CH0);

    let (mut control, net_device) = init_cyw43(spawner, spi, pwr).await;

    let config = Config::dhcpv4(Default::default());
    let mut rng = RoscRng;
    let seed = rng.next_u64();

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    match spawner.spawn(net_task(runner)) {
        Ok(_) => info!("net task running"),
        Err(_) => info!("net task failed"),
    }

    info!("joining...");

    while let Err(err) = control
        .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await
    {
        info!("join failed with status={}", err.status);
    }

    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for DHCP...");
    stack.wait_config_up().await;

    // And now we can use it!
    info!("Stack is up!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;
        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };

            info!("rxd {}", from_utf8(&buf[..n]).unwrap());

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("write error: {:?}", e);
                    break;
                }
            };
        }
    }
}
