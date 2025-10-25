#![no_std]
#![no_main]

mod display_demo;

use defmt::info;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::SPI1;
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_time::Timer;

use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use epd_display::epd::display_spi::DisplaySpi;
use epd_display::epd::epd_2in66b::Epd2in66b;
use display_demo::draw_demo;

static mut CORE1_STACK: Stack<40960> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static DISPLAY: StaticCell<Epd2in66b<SPI1>> = StaticCell::new();

bind_interrupts!(
    struct Irqs {}
);

#[embassy_executor::task]
async fn run_display(display: &'static mut Epd2in66b<SPI1>) {
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
        Timer::after_secs(5).await;
    }
}

#[embassy_executor::task]
async fn run_wifi() {
    loop {
        info!("tick wifi");
        Timer::after_secs(5).await;
    }
}



#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    spawn_core1(p.CORE1, unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK)  }, move || {
        info!("display task spawning");

        let spi_ifc = DisplaySpi::new(p.SPI1, p.PIN_9, p.PIN_10, p.PIN_11, p.DMA_CH0);
        let display = DISPLAY.init(Epd2in66b::new(spi_ifc, p.PIN_13, p.PIN_8, p.PIN_12));

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
