use embassy_rp::peripherals::{PIN_8, PIN_9, PIN_10, PIN_11, PIN_12, PIN_13, DMA_CH1, SPI1};
use static_cell::StaticCell;
use epd_display::{EpdPeripherals, EpdType};

use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use epd_display::epd::three_color::ThreeColor;
use crate::data::display_cmd::{DisplayCmd, TextPanelContent, DISPLAY_CMD_READY, SHARED_DISPLAY_CMD};

pub type DisplayPeripherals = EpdPeripherals<PIN_9, PIN_10, PIN_11, PIN_8, PIN_12, PIN_13, SPI1, DMA_CH1>;

#[embassy_executor::task]
pub async fn run_display(peripherals: DisplayPeripherals) {
    static DISPLAY: StaticCell<EpdType<SPI1>> = StaticCell::new();
    let epd = EpdType::from_peripherals(peripherals);
    let display = DISPLAY.init(epd);

    loop {
        DISPLAY_CMD_READY.receive().await;

        display.init().await;
        display.clear();

        SHARED_DISPLAY_CMD.lock(|cmd| {
            let cmd = cmd.borrow();

            match *cmd {
                DisplayCmd::TextPanel(ref content) => draw_text_panel(display, content),
                _ => ()
            }
        });

        display.refresh().await;
        display.sleep().await;
    }
}

fn draw_text_panel(display: &mut EpdType<SPI1>, content: &TextPanelContent) {
    let title = content.title();

    let _ = Text::with_alignment(
        title.text(),
        Point::new(2, 20),
        MonoTextStyle::new(&FONT_10X20, title.color()),
        Alignment::Left
    ).draw(display);

    if let Ok(w) = i32::try_from(display.width()) {
        let _ = Line::new(Point::new(0, 32), Point::new(w, 32))
            .into_styled(PrimitiveStyle::with_stroke(ThreeColor::Chromatic, 3))
            .draw(display);
    }

    for i in 0 .. content.body_len() {
        let Some(body_line) = content.body_line(i)  else { continue };
        let Ok(i) = i32::try_from(i) else { continue };

        let _ = Text::with_alignment(
            body_line.text(),
            Point::new(2, 55 + i * 22),
            MonoTextStyle::new(&FONT_10X20, body_line.color()),
            Alignment::Left,
        ).draw(display);
    }
}
