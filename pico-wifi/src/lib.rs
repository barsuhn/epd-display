#![no_std]

use cyw43::{JoinError, JoinOptions};
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt::{debug, warn};
use embassy_rp::Peri;
use embassy_rp::peripherals::{PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::gpio;
use embassy_rp::pio;
use embassy_rp::dma;
use embassy_net::Stack;

pub mod init;

pub struct WifiPeripherals<DMA, IRQ> where
    DMA: dma::ChannelInstance,
    IRQ: Binding<DMA::Interrupt, dma::InterruptHandler<DMA>>
       + Binding<<PIO0 as pio::Instance>::Interrupt, pio::InterruptHandler<PIO0>>,
{
    pub pwr_pin: Peri<'static, PIN_23>,
    pub cs_pin: Peri<'static, PIN_25>,
    pub dio_pin: Peri<'static, PIN_24>,
    pub clk_pin: Peri<'static, PIN_29>,
    pub pio: Peri<'static, PIO0>,
    pub dma: Peri<'static, DMA>,
    pub irqs: IRQ,
}

pub struct WifiPio<PIO> where
    PIO: pio::Instance + 'static,
{
    pub cs: gpio::Output<'static>,
    pub dio_pin: Peri<'static, PIN_24>,
    pub clk_pin: Peri<'static, PIN_29>,
    pub pio: pio::Pio<'static, PIO>,
    pub dma: dma::Channel<'static>,
}

impl<PIO: pio::Instance + 'static> WifiPio<PIO>
{
    pub fn new<DMA>
    (
        cs_pin: Peri<'static, PIN_25>,
        dio_pin: Peri<'static, PIN_24>,
        clk_pin: Peri<'static, PIN_29>,
        pio: Peri<'static, PIO>,
        dma: Peri<'static, DMA>,
        irqs: impl Binding<PIO::Interrupt, pio::InterruptHandler<PIO>>
                 + Binding<DMA::Interrupt, dma::InterruptHandler<DMA>>
                 + 'static
    ) -> Self where
        DMA: dma::ChannelInstance,
    {
        WifiPio {
            cs: gpio::Output::new(cs_pin, gpio::Level::High),
            dio_pin,
            clk_pin,
            pio: pio::Pio::new(pio, irqs),
            dma: dma::Channel::new(dma, irqs),
        }
    }

    pub fn pio_spi(mut self) -> PioSpi<'static, PIO, 0> {
        PioSpi::new(
            &mut self.pio.common,
            self.pio.sm0,
            DEFAULT_CLOCK_DIVIDER,
            self.pio.irq0,
            self.cs,
            self.dio_pin,
            self.clk_pin,
            self.dma)
    }
}

pub struct WifiDriver {
    pub control: cyw43::Control<'static>,
    pub stack: Stack<'static>,
}

impl WifiDriver {
    pub async fn connect(&mut self, network: &str, password: &str) -> Result<(), JoinError> {
        self.control.join(network, JoinOptions::new(password.as_bytes())).await?;
        self.stack.wait_link_up().await;
        self.stack.wait_config_up().await;

        if let Some(config) = self.stack.config_v4() {
            debug!("IP address: {}", config.address);
            debug!("Gateway: {:?}", config.gateway);
            debug!("DNS servers: {:?}", config.dns_servers);
        } else {
            warn!("No IPv4 configuration available");
        }

        Ok(())
    }
}
