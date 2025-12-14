#![no_std]
#![no_main]

mod tasks;
mod data;

use {defmt_rtt as _, panic_probe as _};
use defmt::{debug,warn};
use embassy_executor::Executor;
use embassy_rp::multicore::{spawn_core1, Stack};
use static_cell::StaticCell;

use crate::tasks::wifi::{WifiPeripherals, run_wifi};
use crate::tasks::display::{DisplayPeripherals, run_display};

#[cortex_m_rt::entry]
fn main() -> ! {
    static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
    static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
    static CORE1_STACK: StaticCell<Stack<40960>> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    let epd_peripherals = DisplayPeripherals {
        spi: p.SPI1,
        dma: p.DMA_CH1,
        cs_pin: p.PIN_9,
        clk_pin: p.PIN_10,
        mosi_pin: p.PIN_11,
        dc_pin: p.PIN_8,
        rst_pin: p.PIN_12,
        busy_pin: p.PIN_13,
    };

    let wifi_peripherals = WifiPeripherals {
        pwr_pin: p.PIN_23,
        cs_pin: p.PIN_25,
        dio_pin: p.PIN_24,
        clk_pin: p.PIN_29,
        pio: p.PIO0,
        dma: p.DMA_CH0,
    };

    spawn_core1(p.CORE1, CORE1_STACK.init(Stack::new()), move || {
        let executor1 = EXECUTOR1.init(Executor::new());

        executor1.run(|spawner| {
            match spawner.spawn(run_wifi(spawner, wifi_peripherals)) {
                Ok(_) => debug!("Core 1 wifi task started"),
                Err(_) => warn!("Core 1 wifi task failed"),
            }
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner|
        match spawner.spawn(run_display(epd_peripherals)) {
            Ok(_) => debug!("Core 0 display task started"),
            Err(e) => warn!("Core 0 display task failed: {:?}", e),
        }
    );
}
