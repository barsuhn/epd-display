use embassy_rp::Peripheral;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Output, Level};
use embassy_rp::spi::{Async, ClkPin, CsPin, MosiPin, Spi};
use embassy_rp::spi::{Config as SpiConfig, Instance as SpiInstance, Error as SpiError};

pub struct DisplaySpi<SPI>
where
    SPI: SpiInstance + 'static,
{
    pub spi: Spi<'static, SPI, Async>,
    pub cs: Output<'static>,
}

impl<SPI> DisplaySpi<SPI>
where
    SPI: SpiInstance + 'static,
{
    pub fn new<CS, CLK, MOSI, DMA>(
        spi_instance: impl Peripheral<P = SPI> + 'static,
        cs_pin: CS,
        clk_pin: CLK,
        mosi_pin: MOSI,
        dma_channel: DMA,
    ) -> Self
    where
        CS: CsPin<SPI>,
        CLK: ClkPin<SPI>,
        MOSI: MosiPin<SPI>,
        DMA: Channel,
    {
        let cs = Output::new(cs_pin, Level::High);
        let spi = Spi::new_txonly(
            spi_instance,
            clk_pin,
            mosi_pin,
            dma_channel,
            SpiConfig::default(),
        );

        DisplaySpi { spi, cs }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), SpiError> {
        self.cs.set_low();
        let res = self.spi.write(data).await;
        self.cs.set_high();
        res
    }
}
