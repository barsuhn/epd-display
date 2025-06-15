#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::SPI1;
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_time::Timer;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};

use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use display::color::ThreeColor;
use display::display_spi::DisplaySpi;
use display::epd_2in66b::Epd2in66b;

static mut CORE1_STACK: Stack<40960> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static DISPLAY: StaticCell<Epd2in66b<SPI1>> = StaticCell::new();

bind_interrupts!(
    struct Irqs {}
);

fn draw(display: &mut Epd2in66b<SPI1>) {
    let w = display.width() as u32;
    let h = display.height() as u32;

    let text_r = "I see a red door and";
    let text_b = "I want it painted black!";

    let _ = Text::with_alignment(
        text_r,
        Point::new(2, 20),
        MonoTextStyle::new(&FONT_10X20, ThreeColor::Chromatic),
        Alignment::Left,
    )
    .draw(display);

    let _ = Rectangle::new(Point::new(0, 30), Size::new(w, 30))
        .into_styled(PrimitiveStyle::with_fill(ThreeColor::Black))
        .draw(display);

    let _ = Text::with_alignment(
        text_b,
        Point::new(2, 50),
        MonoTextStyle::new(&FONT_10X20, ThreeColor::White),
        Alignment::Left,
    )
    .draw(display);

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

#[embassy_executor::task]
async fn run_display(display: &'static mut Epd2in66b<SPI1>) {
    info!("initializing display");
    display.init().await;

    info!("drawing");
    draw(display);

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
