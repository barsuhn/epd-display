#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

use defmt::{info,warn};
use core::str::from_utf8;
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::Config;
use embassy_sync::channel::{Channel, Sender, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Duration;
use embedded_io_async::Write;
use static_cell::StaticCell;

use wifi::{WifiPeripherals,WifiDriver};
use wifi::init::init_wifi;
use dev_tools::stack_paint::{measure_stack_usage, paint_stack};

const WIFI_NETWORK: &str = dotenvy_macro::dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenvy_macro::dotenv!("WIFI_PASSWORD");

#[cortex_m_rt::entry]
fn main() -> ! {
    static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
    static INIT_CHANNEL: StaticCell<Channel<NoopRawMutex, WifiDriver, 1>> = StaticCell::new();
    let executor0 = EXECUTOR0.init(Executor::new());
    let init_channel = INIT_CHANNEL.init(Channel::new());
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

        match spawner.spawn(run_init_wifi(spawner, wifi_peripherals, init_channel.sender())) {
            Ok(_) => info!("wifi init task started"),
            Err(_) => info!("wifi init task failed"),
        }

        match spawner.spawn(run_wifi(init_channel.receiver())) {
            Ok(_) => info!("connect task started"),
            Err(_) => info!("connect task failed"),
        }
    });
}

#[embassy_executor::task]
async fn run_init_wifi(spawner: Spawner, peripherals: WifiPeripherals, sender: Sender<'static, NoopRawMutex, WifiDriver, 1>) {
    unsafe { paint_stack(); }

    let config = Config::dhcpv4(Default::default());
    let driver = init_wifi(&spawner, peripherals, config).await;

    unsafe { measure_stack_usage(); }

    sender.send(driver).await;
}

#[embassy_executor::task]
async fn run_wifi(receiver: Receiver<'static, NoopRawMutex, WifiDriver, 1>) {
    let mut driver = receiver.receive().await;

    unsafe { paint_stack(); }

    if let Err(err) =  driver.connect(WIFI_NETWORK, WIFI_PASSWORD).await {
        panic!("join failed with status={}", err.status);
    }

    unsafe { measure_stack_usage(); }

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