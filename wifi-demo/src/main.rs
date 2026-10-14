#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

use defmt::{info,warn};
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::Config;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Output, Level};
use embassy_rp::dma;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio;
use embassy_time::Duration;
use heapless::{String, Vec};
use serde::Deserialize;
use static_cell::StaticCell;

use pico_wifi::{WifiPeripherals,WifiPio,WifiDriver};
use pico_wifi::init::init_wifi;
use dev_tools::stack_paint::{measure_stack_usage, paint_stack};

const WIFI_NETWORK: &str = dotenvy_macro::dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenvy_macro::dotenv!("WIFI_PASSWORD");

bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
        DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    }
);

#[cortex_m_rt::entry]
fn main() -> ! {
    static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
    let executor0 = EXECUTOR0.init(Executor::new());
    let p = embassy_rp::init(Default::default());
    let peripherals = WifiPeripherals {
        pwr_pin: p.PIN_23,
        cs_pin: p.PIN_25,
        dio_pin: p.PIN_24,
        clk_pin: p.PIN_29,
        pio: p.PIO0,
        dma: p.DMA_CH0,
        irqs: Irqs,
    };

    executor0.run(|spawner| {
        info!("wifi task spawning");

        match run_wifi(spawner, peripherals).map(|token| spawner.spawn(token)) {
            Ok(_) => info!("wifi init task started"),
            Err(_) => info!("wifi init task failed"),
        }
    });
}

#[embassy_executor::task]
async fn run_wifi(spawner: Spawner, peripherals: WifiPeripherals<DMA_CH0, Irqs>) {
    unsafe { paint_stack("wifi"); }

    let wifi_pio = WifiPio::new(
        peripherals.cs_pin,
        peripherals.dio_pin,
        peripherals.clk_pin,
        peripherals.pio,
        peripherals.dma,
        peripherals.irqs);
    let spi = wifi_pio.pio_spi();
    let pwr = Output::new(peripherals.pwr_pin, Level::Low);

    let mut driver = {
        let config = Config::dhcpv4(Default::default());
        init_wifi(&spawner, pwr, spi, config).await
    };

    if let Err(err) =  driver.connect(WIFI_NETWORK, WIFI_PASSWORD).await {
        panic!("join failed: {:?}", err);
    }

    unsafe { measure_stack_usage("wifi"); }

    run_tcp_server(&mut driver).await;
}

async fn run_tcp_server(driver: &mut WifiDriver) -> ! {
    let WifiDriver{ control, stack} = driver;
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
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

            match parse_message(&buf[..n]) {
                Some(TextMessage { title, body}) => {
                    info!("Received text message");
                    info!("Title: {}", title.as_str());
                    for i in 0..body.len() {
                        info!("Line {}: {}", i, body[i].as_str())
                    }
                },
                _ => info!("Failed to parse message")
            }

            match socket.write(&buf[..n]).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("write error: {:?}", e);
                    break;
                }
            };
        }
    }
}

#[derive(Deserialize, Debug)]
struct TextMessage {
    pub title: String<80>,
    pub body: Vec<String<80>,10>
}

fn parse_message(buf: &[u8]) -> Option<TextMessage> {
    match serde_json_core::from_slice::<TextMessage>(buf) {
        Ok((message,_)) => Some(message),
        _ => None
    }
}

