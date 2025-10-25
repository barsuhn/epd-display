use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Output, Level, Pull};
use embassy_rp::spi::Instance as SpiInstance;
use embassy_rp::gpio::Pin;
use embassy_time::{Duration, Timer};

use crate::display_spi::DisplaySpi;

pub struct Epd<SPI>
where
    SPI: SpiInstance + 'static,
{
    spi: DisplaySpi<SPI>,
    busy: Input<'static>,
    dc: Output<'static>,
    rst: Output<'static>,
}

impl<SPI> Epd<SPI>
where
    SPI: SpiInstance + 'static,
{
    pub fn new(spi: DisplaySpi<SPI>, 
               busy_pin: Peri<'static, impl Pin>, 
               dc_pin: Peri<'static, impl Pin>, 
               rst_pin: Peri<'static, impl Pin>) -> Self 
    {
        let busy = Input::new(busy_pin, Pull::None);
        let dc = Output::new(dc_pin, Level::High);
        let rst = Output::new(rst_pin, Level::High);

        Epd { spi, dc, rst, busy }
    }

    pub fn is_busy(&self) -> bool {
        self.busy.is_high()
    }

    pub async fn hw_reset(&mut self) {
        // HW reset
        let _ = self.rst.set_high();
        Timer::after(Duration::from_millis(20)).await;
        let _ = self.rst.set_low();
        Timer::after(Duration::from_millis(2)).await;
        let _ = self.rst.set_high();
        Timer::after(Duration::from_millis(200)).await;
    }

    pub async fn cmd(&mut self, cmd: u8) {
        let _ = self.dc.set_low();
        let _ = self.spi.write(&[cmd]).await;
    }

    pub async fn cmd_data(&mut self, cmd: u8, data: &[u8]) {
        self.cmd(cmd).await;

        let _ = self.dc.set_high();
        let _ = self.spi.write(data).await;
    }
}
