use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::SpiDevice;
use embedded_hal::digital::{InputPin, OutputPin};

pub struct EpdSpi<SPI, DC, RST, BUSY>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
{
    spi: SPI,
    dc: DC,
    rst: RST,
    busy: BUSY,
}

impl<SPI, DC, RST, BUSY> EpdSpi<SPI, DC, RST, BUSY>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
{
    pub fn new(spi: SPI,
               dc: DC,
               rst: RST,
               busy: BUSY) -> Self
    {
        EpdSpi { spi, dc, rst, busy }
    }

    pub fn is_busy(&mut self) -> bool {
        self.busy.is_high().unwrap_or(false)
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
