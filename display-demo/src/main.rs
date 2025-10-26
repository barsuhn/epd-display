#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_time::Timer;

use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use epd_display::epd2in66b::{EpdType, create_epd, draw_demo, EpdPeripherals};
use dev_tools::stack_paint::{paint_stack, measure_stack_usage};

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static DISPLAY: StaticCell<EpdType> = StaticCell::new();

bind_interrupts!(
    struct Irqs {}
);

#[cortex_m_rt::entry]
fn main() -> ! {
    unsafe { paint_stack(); }

    let p = embassy_rp::init(Default::default());

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        info!("display task spawning");

        let epd_peripherals = EpdPeripherals {
            spi: p.SPI1, dma: p.DMA_CH0, cs_pin: p.PIN_9, clk_pin: p.PIN_10, mosi_pin: p.PIN_11,
            dc_pin: p.PIN_8, rst_pin: p.PIN_12, busy_pin: p.PIN_13,
        };

        let epd = create_epd(epd_peripherals);
        let display = DISPLAY.init(epd);

        match spawner.spawn(run_display(display)) {
            Ok(_) => info!("core 1 display task started"),
            Err(e) => info!("core 1 display task failed: {:?}", e),
        }
    });
}

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

    unsafe { measure_stack_usage(); }

    loop {
        info!("tick display");
        Timer::after_secs(240).await;
    }
}
