#![no_std]
#![no_main]

pub mod bitmap_buffer;
pub mod three_color_epd;

use defmt::info;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::SPI1;
use embassy_executor::Executor;
use embassy_time::Timer;
use {defmt_rtt as _,panic_probe as _};
use static_cell::StaticCell;

use three_color_epd::{SpiInterface, ThreeColorEpd};

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static DISPLAY: StaticCell<ThreeColorEpd<SPI1>> = StaticCell::new();

bind_interrupts!(struct Irqs {});

fn create_fbb() -> BitmapBufferType!(152, 296) {
    let mut fbb = bitmap_buffer!(152, 296);

    fbb.set_pixel(0,0);
    fbb.set_pixel(152,0);
    fbb.set_pixel(304,0);
    fbb.set_pixel(456,0);
    fbb.set_byte(1, 0b10101000);
    fbb.set_byte(20, 0b10101000);
    fbb.set_byte(39, 0b10101000);
    fbb.set_byte(58, 0b10101000);

    fbb
}

fn create_fbc() -> BitmapBufferType!(152, 296) {
    let mut fbc = bitmap_buffer!(152, 296);

    fbc.set_pixel(0,0);
    fbc.set_pixel(152,0);
    fbc.set_pixel(304,0);
    fbc.set_pixel(456,0);
    fbc.set_byte(2, 0b10101000);
    fbc.set_byte(21, 0b10101000);
    fbc.set_byte(40, 0b10101000);
    fbc.set_byte(59, 0b10101000);

    fbc
}

#[embassy_executor::task]
async fn run(display: &'static mut ThreeColorEpd<SPI1>) {

    loop {
        let _fbb = create_fbb();
        let _fbc = create_fbc();

        display.reset().await;

        info!("tick");
        Timer::after_secs(1).await;
    }
}

#[cortex_m_rt::entry]
fn main() -> ! { 
    let p = embassy_rp::init(Default::default());
    let executor0 =  EXECUTOR0.init(Executor::new());
    let spi_ifc = SpiInterface::new(p.SPI1, p.PIN_9, p.PIN_10, p.PIN_11, p.DMA_CH0);
    let display = DISPLAY.init(ThreeColorEpd::new(spi_ifc, p.PIN_13, p.PIN_8, p.PIN_12));

    executor0.run(|spawner| {
        info!("display task spawning");

        match spawner.spawn(run(display)) {
            Ok(_) => info!("display task started"),
            Err(_) => info!("display task failed"),
        }
    });
}
