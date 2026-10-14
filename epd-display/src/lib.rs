#![no_std]

use embassy_rp::dma::{ChannelInstance, InterruptHandler as DmaInterruptHandler};
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::gpio::{Input, Level, Output, Pin, Pull};
use embassy_rp::Peri;
use embassy_rp::spi::{ClkPin, CsPin, MosiPin, Async, Spi, Config as SpiConfig, Instance as SpiInstance};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

use crate::epd::epd_2in66b::Epd2in66b;

pub mod epd;

pub struct EpdPeripherals<CS,CLK,MOSI,DC,RST,BUSY,SPI,DMA,IRQ> where
    CS: CsPin<SPI>,
    CLK: ClkPin<SPI>,
    MOSI: MosiPin<SPI>,
    DC: Pin,
    RST: Pin,
    BUSY: Pin,
    SPI: SpiInstance + 'static,
    DMA: ChannelInstance,
    IRQ: Binding<DMA::Interrupt, DmaInterruptHandler<DMA>> + 'static,
{
    pub spi: Peri<'static, SPI>,
    pub dma: Peri<'static, DMA>,
    pub irqs: IRQ,
    pub cs_pin: Peri<'static, CS>,
    pub clk_pin: Peri<'static, CLK>,
    pub mosi_pin: Peri<'static, MOSI>,
    pub dc_pin: Peri<'static, DC>,
    pub rst_pin: Peri<'static, RST>,
    pub busy_pin: Peri<'static, BUSY>,
}

type SpiDeviceType<SPI> = ExclusiveDevice<Spi<'static, SPI, Async>, Output<'static>, NoDelay>;
pub type EpdType<SPI> = Epd2in66b<SpiDeviceType<SPI>, Output<'static>, Output<'static>, Input<'static>>;

impl <SPI: SpiInstance + 'static> EpdType<SPI> {
    pub fn from_peripherals<CS,CLK,MOSI,DC,RST,BUSY,DMA,IRQ>(p: EpdPeripherals<CS,CLK,MOSI,DC,RST,BUSY,SPI,DMA,IRQ>) -> Self
    where
        CS: CsPin<SPI>,
        CLK: ClkPin<SPI>,
        MOSI: MosiPin<SPI>,
        DC: Pin,
        RST: Pin,
        BUSY: Pin,
        DMA: ChannelInstance,
        IRQ: Binding<DMA::Interrupt, DmaInterruptHandler<DMA>> + 'static,
    {
        let spi_bus = Spi::new_txonly(
            p.spi,
            p.clk_pin,
            p.mosi_pin,
            p.dma,
            p.irqs,
            SpiConfig::default(),
        );
        let cs = Output::new(p.cs_pin, Level::High);
        let Ok(spi_device) = ExclusiveDevice::new(spi_bus, cs, NoDelay);

        let dc = Output::new(p.dc_pin, Level::High);
        let rst = Output::new(p.rst_pin, Level::High);
        let busy = Input::new(p.busy_pin, Pull::None);
        Epd2in66b::new(spi_device, dc, rst, busy)
    }
}
