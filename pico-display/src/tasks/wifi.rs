use defmt::{info, warn};
use core::str::from_utf8;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_net::Config;
use embassy_net::tcp::TcpSocket;
use embassy_time::Duration;
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use embedded_io_async::Write;
use pico_wifi::{WifiDriver,WifiPio};
pub use pico_wifi::WifiPeripherals;
use pico_wifi::init::init_wifi;

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


bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    }
);

const WIFI_NETWORK: &str = dotenvy_macro::dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenvy_macro::dotenv!("WIFI_PASSWORD");

#[embassy_executor::task]
pub async fn run_wifi(spawner: Spawner, peripherals: WifiPeripherals<DMA_CH0>) {
    info!("Wifi initialization");

    let mut wifi_pio = WifiPio::new(peripherals.cs_pin, peripherals.pio, Irqs);
    let spi = wifi_spi!(wifi_pio, peripherals.dio_pin, peripherals.clk_pin, peripherals.dma);
    let pwr = Output::new(peripherals.pwr_pin, Level::Low);

    let config = Config::dhcpv4(Default::default());
    let mut driver = init_wifi(&spawner, pwr, spi, config).await;

    if let Err(err) =  driver.connect(WIFI_NETWORK, WIFI_PASSWORD).await {
        panic!("Connection failed with status={}", err.status);
    }

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
