use embassy_rp::dma::Channel;
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Level, Output, Pin, Pull};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Async, Spi, CsPin, ClkPin, MosiPin, Config as SpiConfig};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use embedded_hal_async::spi::SpiDevice;
use embedded_hal::digital::{InputPin, OutputPin};

use embedded_graphics::{
    prelude::*,
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    primitives::{Circle, Line, Rectangle, PrimitiveStyle},
    text::{Alignment, Text},
};

use crate::epd::three_color::ThreeColor;
use crate::epd::epd_2in66b::Epd2in66b;

type SpiDeviceType = ExclusiveDevice<Spi<'static, SPI1, Async>, Output<'static>, NoDelay>;
pub type EpdType = Epd2in66b<SpiDeviceType, Output<'static>, Output<'static>, Input<'static>>;

pub struct EpdPeripherals<CS,CLK,MOSI,DC,RST,BUSY,DMA> where
    CS: CsPin<SPI1>,
    CLK: ClkPin<SPI1>,
    MOSI: MosiPin<SPI1>,
    DC: Pin,
    RST: Pin,
    BUSY: Pin,
    DMA: Channel,
{
    pub spi: Peri<'static, SPI1>,
    pub dma: Peri<'static, DMA>,
    pub cs_pin: Peri<'static, CS>,
    pub clk_pin: Peri<'static, CLK>,
    pub mosi_pin: Peri<'static, MOSI>,
    pub dc_pin: Peri<'static, DC>,
    pub rst_pin: Peri<'static, RST>,
    pub busy_pin: Peri<'static, BUSY>,
}

pub fn create_epd<CS,CLK,MOSI,DC,RST,BUSY,DMA>(p: EpdPeripherals<CS,CLK,MOSI,DC,RST,BUSY,DMA>) -> EpdType
where
    CS: CsPin<SPI1>,
    CLK: ClkPin<SPI1>,
    MOSI: MosiPin<SPI1>,
    DC: Pin,
    RST: Pin,
    BUSY: Pin,
    DMA: Channel,
{
    let spi_bus = Spi::new_txonly(
        p.spi,
        p.clk_pin,
        p.mosi_pin,
        p.dma,
        SpiConfig::default(),
    );
    let cs = Output::new(p.cs_pin, Level::High);
    let Ok(spi_device) = ExclusiveDevice::new(spi_bus, cs, NoDelay);

    let dc = Output::new(p.dc_pin, Level::High);
    let rst = Output::new(p.rst_pin, Level::High);
    let busy = Input::new(p.busy_pin, Pull::None);
    Epd2in66b::new(spi_device, dc, rst, busy)
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
