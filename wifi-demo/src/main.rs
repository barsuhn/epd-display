#![no_std]
#![no_main]


use defmt::info;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use epd_display::epd2in66b::{EpdType, create_epd, draw_demo, EpdPeripherals};

static mut CORE1_STACK: Stack<65536> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static DISPLAY: StaticCell<EpdType> = StaticCell::new();

bind_interrupts!(
    struct Irqs {}
);

#[embassy_executor::task]
async fn run_display(display: &'static mut EpdType) {
    info!("initializing display");
    display.init().await;

    info!("drawing");
    draw_demo(display);

    info!("updating display");
    display.refresh().await;

    info!("going to sleep state");
    display.sleep().await;

    loop {
        info!("tick display");
        Timer::after_secs(240).await;
    }
}

#[embassy_executor::task]
async fn run_wifi() {
    loop {
        info!("tick wifi");
        Timer::after_secs(240).await;
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    spawn_core1(p.CORE1, unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK)  }, move || {
        info!("display task spawning");

        let epd_peripherals = EpdPeripherals {
            spi: p.SPI1, dma: p.DMA_CH0, cs_pin: p.PIN_9, clk_pin: p.PIN_10, mosi_pin: p.PIN_11,
            dc_pin: p.PIN_8, rst_pin: p.PIN_12, busy_pin: p.PIN_13,
        };

        let epd = create_epd(epd_peripherals);
        let display = DISPLAY.init(epd);

        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner|
            match spawner.spawn(run_display(display)) {
                Ok(_) => info!("core 1 display task started"),
                Err(e) => info!("core 1 display task failed: {:?}", e),
            }
        );
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        info!("wifi task spawning");

        match spawner.spawn(run_wifi()) {
            Ok(_) => info!("core 0 wifi task started"),
            Err(_) => info!("core 0 wifi task failed"),
        }
    });
}
