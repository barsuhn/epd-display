#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::SPI1;
use embassy_time::Timer;

use embedded_hal_async::spi::SpiDevice;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_graphics::{
    prelude::*,
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    primitives::{Circle, Line, Rectangle, PrimitiveStyle},
    text::{Alignment, Text},
};

use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use dev_tools::stack_paint::{paint_stack, measure_stack_usage};
use epd_display::{EpdType, EpdPeripherals};
use epd_display::epd::epd_2in66b::Epd2in66b;
use epd_display::epd::three_color::ThreeColor;

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static DISPLAY: StaticCell<EpdType<SPI1>> = StaticCell::new();

bind_interrupts!(
    struct Irqs {}
);

#[cortex_m_rt::entry]
fn main() -> ! {
    unsafe { paint_stack("display"); }

    let p = embassy_rp::init(Default::default());

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        info!("display task spawning");

        let epd_peripherals = EpdPeripherals {
            spi: p.SPI1, dma: p.DMA_CH0, cs_pin: p.PIN_9, clk_pin: p.PIN_10, mosi_pin: p.PIN_11,
            dc_pin: p.PIN_8, rst_pin: p.PIN_12, busy_pin: p.PIN_13,
        };

        let epd = EpdType::from_peripherals(epd_peripherals);
        let display = DISPLAY.init(epd);

        match spawner.spawn(run_display(display)) {
            Ok(_) => info!("display task started"),
            Err(e) => info!("display task failed: {:?}", e),
        }
    });
}

#[embassy_executor::task]
async fn run_display(display: &'static mut EpdType<SPI1>) {
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

pub fn draw_demo<SPI, DC, RST, BUSY>(display: &mut Epd2in66b<SPI, DC, RST, BUSY>)
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
{
    let w = display.width() as u32;
    let h = display.height() as u32;

    let text_r = "I see a red door and";
    let text_b = "I want it painted black!";

    let _ = Text::with_alignment(
        text_r,
        Point::new(2, 20),
        MonoTextStyle::new(&FONT_10X20, ThreeColor::Chromatic),
        Alignment::Left
    ).draw(display);

    let _ = Rectangle::new(Point::new(0, 30), Size::new(w, 30))
        .into_styled(PrimitiveStyle::with_fill(ThreeColor::Black))
        .draw(display);

    let _ = Text::with_alignment(
        text_b,
        Point::new(2, 50),
        MonoTextStyle::new(&FONT_10X20, ThreeColor::White),
        Alignment::Left,
    ).draw(display);

    let ly = 65;
    let _ = Line::new(Point::new(0, ly), Point::new(w as i32, ly))
        .into_styled(PrimitiveStyle::with_stroke(ThreeColor::Chromatic, 3))
       .draw(display);

    let cy = (0.75 * h as f64) as i32;
    let or = (0.20 * h as f64) as u32;
    let ir = (0.15 * h as f64) as u32;

    for i in 0..4 {
        let xl = i as f64 * 0.25;
        let xh = (i + 1) as f64 * 0.25;
        let xm = 0.5 * (xl + xh);
        let cx = (xm * w as f64) as i32;
        let oc = if i % 2 == 0 {
            ThreeColor::Black
        } else {
            ThreeColor::Chromatic
        };
        let ic = if i % 2 == 0 {
            ThreeColor::Chromatic
        } else {
            ThreeColor::Black
        };

        let _ = Circle::with_center(Point::new(cx, cy), 2 * or)
            .into_styled(PrimitiveStyle::with_stroke(oc, 5))
            .draw(display);

        let _ = Circle::with_center(Point::new(cx, cy), 2 * ir)
            .into_styled(PrimitiveStyle::with_fill(ic))
            .draw(display);
    }
}
