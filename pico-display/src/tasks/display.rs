use defmt::info;
use embassy_rp::peripherals::{PIN_8, PIN_9, PIN_10, PIN_11, PIN_12, PIN_13, DMA_CH1};
use embassy_time::Timer;
use static_cell::StaticCell;
use epd_display::epd2in66b::{create_epd, draw_demo, EpdPeripherals, EpdType};

pub type DisplayPeripherals = EpdPeripherals<PIN_9, PIN_10, PIN_11, PIN_8, PIN_12, PIN_13, DMA_CH1>;

#[embassy_executor::task]
pub async fn run_display(peripherals: DisplayPeripherals) {
    static DISPLAY: StaticCell<EpdType> = StaticCell::new();
    let epd = create_epd(peripherals);
    let display = DISPLAY.init(epd);

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
