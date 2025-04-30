use embassy_rp::Peripheral;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Input, Output, Level, Pull};
use embassy_rp::spi::{Async, ClkPin, CsPin, MosiPin, Spi, };
use embassy_rp::spi::{Config as SpiConfig, Instance as SpiInstance};
use embassy_rp::gpio::Pin;
use embassy_time::{Duration, Timer};

pub struct SpiInterface<SPI>
where
    SPI: SpiInstance + 'static,
{
    pub spi: Spi<'static, SPI, Async>,
    pub cs: Output<'static>,
}

impl<SPI> SpiInterface<SPI>
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
        SpiInterface { spi, cs }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), embassy_rp::spi::Error> {
        self.cs.set_low();
        let res = self.spi.write(data).await;
        self.cs.set_high();
        res
    }
}

pub struct ThreeColorEpd<SPI>
where
    SPI: SpiInstance + 'static,
{
    spi: SpiInterface<SPI>,
    busy: Input<'static>,
    dc: Output<'static>,
    rst: Output<'static>,
}

impl<SPI> ThreeColorEpd<SPI>
where
    SPI: SpiInstance + 'static,
{
    pub fn new<BUSY,DC,RST>(spi: SpiInterface<SPI>, busy_pin: BUSY, dc_pin: DC, rst_pin: RST)
    -> Self 
    where
        BUSY: Pin,
        DC: Pin,
        RST: Pin
    {
        let busy = Input::new(busy_pin, Pull::None);
        let dc = Output::new(dc_pin, Level::High);
        let rst = Output::new(rst_pin, Level::High);

        ThreeColorEpd { spi, dc, rst, busy }
    }

    pub async fn reset(&mut self) {
        // HW reset
        let _ = self.rst.set_high();
        Timer::after(Duration::from_millis(20)).await;
        let _ = self.rst.set_low();
        Timer::after(Duration::from_millis(2)).await;
        let _ = self.rst.set_high();
        Timer::after(Duration::from_millis(200)).await;

        // SW reset
        let _ = self.dc.set_low();
        let _ = self.spi.write(&[ThreeColorEpdCommand::Reset as u8]).await;

        while self.busy.is_high() {
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

enum ThreeColorEpdCommand {
    Reset = 0x12,
}
