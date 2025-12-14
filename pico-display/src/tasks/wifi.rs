use defmt::{trace, debug, info, warn};
use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_net::Config;
use embassy_net::tcp::TcpSocket;
use embassy_time::Duration;
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use heapless::{String, Vec};
use serde::Deserialize;

use epd_display::epd::three_color::ThreeColor;
use pico_wifi::{WifiDriver, WifiPio};
pub use pico_wifi::WifiPeripherals;
use pico_wifi::init::init_wifi;

use crate::data::display_cmd::{DisplayCmd, TextLine, TextPanelContent, DISPLAY_CMD_READY, SHARED_DISPLAY_CMD};

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
    trace!("Wifi initialization");

    let mut wifi_pio = WifiPio::new(peripherals.cs_pin, peripherals.pio, Irqs);
    let spi = wifi_spi!(wifi_pio, peripherals.dio_pin, peripherals.clk_pin, peripherals.dma);
    let pwr = Output::new(peripherals.pwr_pin, Level::Low);

    let config = Config::dhcpv4(Default::default());
    let mut driver = init_wifi(&spawner, pwr, spi, config).await;

    if let Err(err) =  driver.connect(WIFI_NETWORK, WIFI_PASSWORD).await {
        panic!("Connection failed with status={}", err.status);
    }

    if let Some(config) = driver.stack.config_v4() {
        let title = TextLine::new("Display server connected", ThreeColor::Black);
        let mut content = TextPanelContent::new(title);
        let mut body_text: String<80> = String::new();

        if write!(body_text, "IP address: {}", config.address.address()).is_ok() {
            let body_line = TextLine::new(&body_text, ThreeColor::Black);
            let _ = content.add_body_line(body_line);
        }

        send_text_panel(content);
    } else {
        let title = TextLine::new("Wifi connection failed", ThreeColor::Chromatic);
        let content = TextPanelContent::new(title);

        send_text_panel(content);
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
        debug!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        trace!("Received connection from {:?}", socket.remote_endpoint());
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

            if let Some(content) = parse_message(&buf[..n]) {
                info!("Parsed message.");
                send_text_panel(content);
            } else {
                info!("Couldn't parse message.")
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

#[derive(Deserialize)]
struct TextMessage {
    pub title: String<80>,
    pub body: Vec<String<80>,10>
}

fn parse_message(buf: &[u8]) -> Option<TextPanelContent> {
    match serde_json_core::from_slice::<TextMessage>(buf) {
        Ok((TextMessage {title, body}, _)) => {
            let title = TextLine::new(&title, ThreeColor::Black);
            let mut content = TextPanelContent::new(title);

            for body_text in body {
                let body_line = TextLine::new(&body_text, ThreeColor::Black);
                content.add_body_line(body_line).ok()?;
            }

            Some(content)
        },
        _ => None
    }
}

fn send_text_panel(content: TextPanelContent) {
    SHARED_DISPLAY_CMD.lock(|cmd| {
        *cmd.borrow_mut() = DisplayCmd::TextPanel(content);
    });

    let _ = DISPLAY_CMD_READY.try_send(());
}
