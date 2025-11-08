#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

use defmt::*;
use core::str::from_utf8;
use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, StackResources};
use embassy_rp::{bind_interrupts, Peri};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::channel::{Channel, Sender, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Duration;
use embedded_io_async::Write;
use static_cell::StaticCell;

use dev_tools::stack_paint::{measure_stack_usage, paint_stack};

bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => InterruptHandler<PIO0>;
    }
);

const WIFI_NETWORK: &str = dotenvy_macro::dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenvy_macro::dotenv!("WIFI_PASSWORD");

struct WifiPeripherals {
    pwr_pin: Peri<'static, PIN_23>,
    cs_pin: Peri<'static, PIN_25>,
    dio_pin: Peri<'static, PIN_24>,
    clk_pin: Peri<'static, PIN_29>,
    pio: Peri<'static, PIO0>,
    dma: Peri<'static, DMA_CH0>,
}

struct WifiHardware {
    control: cyw43::Control<'static>,
    net_device: cyw43::NetDriver<'static>,
}

struct WifiResources {
    stack: embassy_net::Stack<'static>,
    control: cyw43::Control<'static>,
}

// This allocates memory for 3 sockets. DHCP and DNS each require one socket.
static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();
static INIT_CHANNEL: StaticCell<Channel<NoopRawMutex, WifiHardware, 1>> = StaticCell::new();
static CONNECT_CHANNEL: StaticCell<Channel<NoopRawMutex, WifiResources, 1>> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let init_channel = INIT_CHANNEL.init(Channel::new());
    let connect_channel = CONNECT_CHANNEL.init(Channel::new());
    let executor0 = EXECUTOR0.init(Executor::new());
    let p = embassy_rp::init(Default::default());
    let wifi_peripherals = WifiPeripherals {
        pwr_pin: p.PIN_23,
        cs_pin: p.PIN_25,
        dio_pin: p.PIN_24,
        clk_pin: p.PIN_29,
        pio: p.PIO0,
        dma: p.DMA_CH0,
    };

    executor0.run(|spawner| {
        info!("wifi task spawning");

        match spawner.spawn(run_wifi_init(spawner, wifi_peripherals, init_channel.sender())) {
            Ok(_) => info!("wifi init task started"),
            Err(_) => info!("wifi init task failed"),
        }

        match spawner.spawn(run_wifi_connect(spawner, init_channel.receiver(), connect_channel.sender())) {
            Ok(_) => info!("connect task started"),
            Err(_) => info!("connect task failed"),
        }

        match spawner.spawn(run_server(connect_channel.receiver())) {
            Ok(_) => info!("server task started"),
            Err(_) => info!("server task failed")
        }
    });
}

#[embassy_executor::task]
async fn run_wifi_init(spawner: Spawner, peripherals: WifiPeripherals, sender: Sender<'static, NoopRawMutex, WifiHardware, 1>) {
    unsafe { paint_stack(); }

    let mut wifi_hardware = init_wifi_hardware(&spawner, peripherals).await;

    unsafe { measure_stack_usage(); }

    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    wifi_hardware.control.init(clm).await;
    wifi_hardware.control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    sender.send(wifi_hardware).await;
}

#[embassy_executor::task]
async fn run_wifi_connect(spawner: Spawner, receiver: Receiver<'static, NoopRawMutex, WifiHardware, 1>, sender: Sender<'static, NoopRawMutex, WifiResources, 1>) {
    let WifiHardware{ mut control, net_device} = receiver.receive().await;

    unsafe { paint_stack(); }

    let stack = connect_to_network(&spawner, &mut control, net_device).await;
    let resources = WifiResources{ stack, control };

    unsafe { measure_stack_usage(); }

    sender.send(resources).await;
}

#[embassy_executor::task]
async fn run_server(receiver: Receiver<'static, NoopRawMutex, WifiResources, 1>) {
    let WifiResources{ stack, mut control} = receiver.receive().await;

    run_tcp_server(stack, &mut control).await
}

async fn init_wifi_hardware(
    spawner: &Spawner,
    peripherals: WifiPeripherals,
) -> WifiHardware {
    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");

    let pwr = Output::new(peripherals.pwr_pin, Level::Low);
    let cs = Output::new(peripherals.cs_pin, Level::High);
    let mut pio = Pio::new(peripherals.pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        peripherals.dio_pin,
        peripherals.clk_pin,
        peripherals.dma,
    );

    let state = CYW43_STATE.init(cyw43::State::new());
    let (net_device, control, runner) =cyw43::new(state, pwr, spi, fw).await;

    match spawner.spawn(cyw43_task(runner)) {
        Ok(_) => info!("Cyw43 runner task spawned"),
        Err(_) => info!("Cyw43 runner task failed")
    };

    WifiHardware {
        control,
        net_device,
    }
}

async fn connect_to_network(
    spawner: &Spawner,
    control: &mut cyw43::Control<'static>,
    net_device: cyw43::NetDriver<'static>,
) -> embassy_net::Stack<'static> {
    let mut rng = RoscRng;
    let seed = rng.next_u64();
    let config = Config::dhcpv4(Default::default());
    let (stack, runner) = embassy_net::new(net_device, config, STACK_RESOURCES.init(StackResources::new()), seed);

    match spawner.spawn(net_task(runner)) {
        Ok(_) => info!("Net runner task spawned"),
        Err(_) => info!("Net runner task failed")
    }

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

    info!("Stack is up!");

    if let Some(config) = stack.config_v4() {
        info!("IP address: {}", config.address);
        info!("Gateway: {:?}", config.gateway);
        info!("DNS servers: {:?}", config.dns_servers);
    } else {
        warn!("No IPv4 configuration available");
    }

    stack
}

async fn run_tcp_server(
    stack: embassy_net::Stack<'static>,
    control: &mut cyw43::Control<'static>,
) -> ! {
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

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}