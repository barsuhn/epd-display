#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

use core::str::from_utf8;
use defmt::{info, warn};
use embassy_executor::{Executor, Spawner};
use embassy_net::Config;
use embassy_rp::bind_interrupts;
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_net::tcp::TcpSocket;
use embassy_rp::peripherals::DMA_CH0;
use embassy_sync::channel::{Channel, Sender, Receiver};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Timer, Duration};
use embedded_io_async::Write;
use static_cell::StaticCell;

use dev_tools::stack_paint::{paint_stack, paint_stack_mem, measure_stack_usage, measure_stack_mem_usage};
use epd_display::epd2in66b::{EpdType, create_epd, draw_demo, EpdPeripherals};
use pico_wifi::{WifiPeripherals,WifiDriver};
use pico_wifi::init::init_wifi;

static mut CORE1_STACK: Stack<32768> = Stack::new();

bind_interrupts!(
    struct Irqs {}
);

const WIFI_NETWORK: &str = dotenvy_macro::dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenvy_macro::dotenv!("WIFI_PASSWORD");

#[cortex_m_rt::entry]
fn main() -> ! {
    static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
    static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
    static DISPLAY: StaticCell<EpdType> = StaticCell::new();

    static INIT_CHANNEL: StaticCell<Channel<NoopRawMutex, WifiDriver, 1>> = StaticCell::new();
    let p = embassy_rp::init(Default::default());

    let epd_peripherals = EpdPeripherals {
        spi: p.SPI1, dma: p.DMA_CH1, cs_pin: p.PIN_9, clk_pin: p.PIN_10, mosi_pin: p.PIN_11,
        dc_pin: p.PIN_8, rst_pin: p.PIN_12, busy_pin: p.PIN_13,
    };

    let wifi_peripherals = WifiPeripherals {
        pwr_pin: p.PIN_23,
        cs_pin: p.PIN_25,
        dio_pin: p.PIN_24,
        clk_pin: p.PIN_29,
        pio: p.PIO0,
        dma: p.DMA_CH0,
    };

    spawn_core1(p.CORE1, unsafe { &mut *(&raw mut CORE1_STACK)  }, move || {
        let init_channel = INIT_CHANNEL.init(Channel::new());
        let executor1 = EXECUTOR1.init(Executor::new());

        executor1.run(|spawner| {
            info!("wifi task spawning");

            match spawner.spawn(run_init_wifi(spawner, wifi_peripherals, init_channel.sender())) {
                Ok(_) => info!("core 1 wifi init task started"),
                Err(_) => info!("core 1 wifi init task failed"),
            }

            match spawner.spawn(run_wifi(init_channel.receiver())) {
                Ok(_) => info!("core 1 connect task started"),
                Err(_) => info!("core 1 connect task failed"),
            }
        });
    });

    info!("display task spawning");

    let epd = create_epd(epd_peripherals);
    let display = DISPLAY.init(epd);

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner|
        match spawner.spawn(run_display(display)) {
            Ok(_) => info!("core 0 display task started"),
            Err(e) => info!("core 0 display task failed: {:?}", e),
        }
    );
}

#[embassy_executor::task]
async fn run_display(display: &'static mut EpdType) {
    unsafe { paint_stack("display"); }

    info!("initializing display");
    display.init().await;

    info!("drawing");
    draw_demo(display);

    info!("updating display");
    display.refresh().await;

    info!("going to sleep state");
    display.sleep().await;

    unsafe { measure_stack_usage("display"); }

    loop {
        info!("tick display");
        Timer::after_secs(240).await;
    }
}

#[embassy_executor::task]
async fn run_init_wifi(spawner: Spawner, peripherals: WifiPeripherals<DMA_CH0>, sender: Sender<'static, NoopRawMutex, WifiDriver, 1>) {
    unsafe { paint_stack_mem("wifi init", &raw mut CORE1_STACK.mem); }

    let config = Config::dhcpv4(Default::default());
    let driver = init_wifi(&spawner, peripherals, config).await;

    unsafe { measure_stack_mem_usage("wifi init", &raw const CORE1_STACK.mem); }

    sender.send(driver).await;
}

#[embassy_executor::task]
async fn run_wifi(receiver: Receiver<'static, NoopRawMutex, WifiDriver, 1>) {
    let mut driver = receiver.receive().await;

    unsafe { paint_stack_mem("wifi", &raw mut CORE1_STACK.mem); }

    if let Err(err) =  driver.connect(WIFI_NETWORK, WIFI_PASSWORD).await {
        panic!("join failed with status={}", err.status);
    }

    info!("Connected");

    unsafe { measure_stack_mem_usage("wifi", &raw const CORE1_STACK.mem); }

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
