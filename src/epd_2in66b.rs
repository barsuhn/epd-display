use embassy_rp::Peri;
use embassy_rp::spi::Instance as SpiInstance;
use embassy_rp::gpio::Pin;
use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::raw::RawU2;
use embedded_graphics::Pixel;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::geometry::Point;
use embedded_graphics::prelude::*;

use crate::display_orientation::DisplayOrientation;
use crate::display_spi::DisplaySpi;
use crate::color::ThreeColor;
use crate::epd::Epd;
use crate::{BitmapBufferType, bitmap_buffer};


const WIDTH: usize = 152;
const HEIGHT: usize = 296;

pub struct Epd2in66b<SPI>
where
    SPI: SpiInstance + 'static,
{
    epd: Epd<SPI>,
    orientation: DisplayOrientation,
    bw_buffer: BitmapBufferType!(WIDTH, HEIGHT),
    chromatic_buffer: BitmapBufferType!(WIDTH, HEIGHT),
}

// public API

impl<SPI> Epd2in66b<SPI>
where
    SPI: SpiInstance + 'static,
{
    pub fn new(spi: DisplaySpi<SPI>,
               busy_pin: Peri<'static, impl Pin>,
               dc_pin: Peri<'static, impl Pin>,
               rst_pin: Peri<'static, impl Pin>) -> Self {
        let epd = Epd::new(spi, busy_pin, dc_pin, rst_pin);
        let bw_buffer = bitmap_buffer!(WIDTH, HEIGHT);
        let chromatic_buffer = bitmap_buffer!(WIDTH, HEIGHT);
        let orientation = DisplayOrientation::Landscape;

        let mut epd = Epd2in66b { epd, orientation, bw_buffer, chromatic_buffer };

        epd.clear();

        epd
    }

    pub fn width(&self) -> usize {
        match self.orientation {
            DisplayOrientation::Portrait => WIDTH,
            DisplayOrientation::Landscape => HEIGHT,
            DisplayOrientation::PortraitFlipped => WIDTH,
            DisplayOrientation::LandscapeFlipped => HEIGHT,
        }
    }

    pub fn height(&self) -> usize {
        match self.orientation {
            DisplayOrientation::Portrait => HEIGHT,
            DisplayOrientation::Landscape => WIDTH,
            DisplayOrientation::PortraitFlipped => HEIGHT,
            DisplayOrientation::LandscapeFlipped => WIDTH,
        }
    }

    pub fn clear(&mut self) {
        self.bw_buffer.fill(0xff);
        self.chromatic_buffer.fill(0x0);
    }

    pub async fn init(&mut self) {
        self.epd.hw_reset().await;
        self.sw_reset().await;

        self.set_data_entry_mode(DataEntryRow::XMinor, DataEntrySign::IncYIncX).await;
        self.set_display_update(WriteMode::Normal, WriteMode::Normal, OutputSource::S8ToS167).await;
        self.set_window(0, WIDTH - 1, 0, HEIGHT - 1).await;
    }

    pub async fn refresh(&mut self) {
        self.set_cursor(0, 0).await;
        self.epd.cmd_data(ThreeColorEpdCommand::WriteBlackWhiteRAM as u8, &self.bw_buffer.buffer).await;
        self.set_cursor(0, 0).await;
        self.epd.cmd_data(ThreeColorEpdCommand::WriteChromaticRAM as u8, &self.chromatic_buffer.buffer).await;
        self.activate().await;
    }

    pub async fn sleep(&mut self) {
        self.cmd_data(ThreeColorEpdCommand::DeepSleepMode, &[DeepSleep::SleepLosingRAM as u8]).await;
    }
}

impl PixelColor for ThreeColor {
    type Raw = RawU2;
}

impl<SPI> Dimensions for Epd2in66b<SPI>
where
    SPI: SpiInstance + 'static,
{
    fn bounding_box(&self) -> Rectangle {
       Rectangle::new(Point::zero(), Size::new(WIDTH as u32, HEIGHT as u32))
    }
}

impl<SPI> DrawTarget for Epd2in66b<SPI>
where
    SPI: SpiInstance + 'static,
{
    type Color = ThreeColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord,color) in pixels {
            let coord = self.convert_point(coord);
            
            if coord.x < 0 || coord.x >= WIDTH as i32 || coord.y < 0 || coord.y >= HEIGHT as i32 {
                continue;
            }
            
            let (x, y) = (coord.x as usize, coord.y as usize);

            match color {
                ThreeColor::Black => {
                    self.bw_buffer.clear_pixel(x, y);
                    self.chromatic_buffer.clear_pixel(x, y);
                },
                ThreeColor::White => {
                    self.bw_buffer.set_pixel(x, y);
                    self.chromatic_buffer.clear_pixel(x, y);
                },
                ThreeColor::Chromatic => {
                    self.bw_buffer.clear_pixel(x, y);
                    self.chromatic_buffer.set_pixel(x, y);
                },
            }
        }

        Ok(())
    }
}

// private API

impl<SPI> Epd2in66b<SPI>
where
    SPI: SpiInstance + 'static,
{
    fn convert_point(&self, point: Point) -> Point {
        match self.orientation {
            DisplayOrientation::Portrait => Point::new(point.x, point.y),
            DisplayOrientation::LandscapeFlipped => Point::new(WIDTH as i32 - 1 - point.y, point.x),
            DisplayOrientation::PortraitFlipped => Point::new(WIDTH as i32 - 1 - point.x, HEIGHT as i32 - 1 - point.y),
            DisplayOrientation::Landscape => Point::new(point.y, HEIGHT as i32 - 1 - point.x),
        }
    }

    async fn sw_reset(&mut self) {
        self.cmd(ThreeColorEpdCommand::Reset).await;

        while self.epd.is_busy() {
            Timer::after(Duration::from_millis(10)).await;
        }
    }

    async fn cmd(&mut self, cmd: ThreeColorEpdCommand) {
        self.epd.cmd(cmd as u8).await;
    }

    async fn cmd_data(&mut self, cmd: ThreeColorEpdCommand, data: &[u8]) {
        self.epd.cmd_data(cmd as u8, data).await;
    }

    async fn set_data_entry_mode(&mut self, row: DataEntryRow, sign: DataEntrySign) {
        self.cmd_data(ThreeColorEpdCommand::DataEntryMode, &[row as u8 | sign as u8]).await;
    }

    async fn set_window(&mut self, x_start: usize, x_end: usize, y_start: usize, y_end: usize) {
        self.cmd_data(ThreeColorEpdCommand::SetXAddressRange, &[(x_start >> 3) as u8, (x_end >> 3) as u8]).await;
        self.cmd_data(ThreeColorEpdCommand::SetYAddressRange, &[
            (y_start & 0xff) as u8, 
            (y_start >> 8) as u8,
            (y_end & 0xff) as u8,
            (y_end >> 8) as u8]).await;
    }

    async fn set_display_update(&mut self, bw_mode: WriteMode, red_mode: WriteMode, output_source: OutputSource) {
        self.cmd_data(ThreeColorEpdCommand::DisplayUpdateControl1, &[
            (red_mode as u8) << 4 | (bw_mode as u8),
            (output_source as u8)
        ]).await;
    }

    pub async fn set_cursor(&mut self, x: u8, y: u16) {
        self.cmd_data(ThreeColorEpdCommand::SetXAddressCounter, &[x]).await;
        self.cmd_data(ThreeColorEpdCommand::SetYAddressCounter, &[
            (y & 0xff) as u8, 
            (y >> 8) as u8]).await;
    }

    async fn activate(&mut self) {
        self.cmd(ThreeColorEpdCommand::MasterActivation).await;
        Timer::after(Duration::from_millis(20)).await;

        while self.epd.is_busy() {
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

enum ThreeColorEpdCommand {
    DeepSleepMode = 0x10,
    DataEntryMode = 0x11,
    Reset = 0x12,
    MasterActivation = 0x20,
    DisplayUpdateControl1 = 0x21,
    WriteBlackWhiteRAM = 0x24,
    WriteChromaticRAM = 0x26,
    SetXAddressRange = 0x44,
    SetYAddressRange = 0x45,
    SetXAddressCounter = 0x4e,
    SetYAddressCounter = 0x4f,
}

#[allow(dead_code)]
enum DataEntrySign {
    DecYDecX = 0b00,
    DecYIncX = 0b01,
    IncYDecX = 0b10,
    IncYIncX = 0b11,
}

#[allow(dead_code)]
enum DataEntryRow {
    XMinor = 0b000,
    YMinor = 0b100,
}

#[allow(dead_code)]
enum WriteMode {
    Normal = 0b0000,
    ForceZero = 0b0100,
    Invert = 0b1000,
}

#[allow(dead_code)]
enum OutputSource {
    S0ToS175 = 0x00,
    S8ToS167 = 0x80,
}

#[allow(dead_code)]
enum DeepSleep {
    Awake = 0b00,
    SleepKeepingRAM = 0b01,
    SleepLosingRAM = 0b11,
}
