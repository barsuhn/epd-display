#![no_std]

use cyw43::{ControlError, JoinOptions};
use defmt::{debug, warn};
use embassy_rp::Peri;
use embassy_rp::peripherals::{PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::gpio::{Output, Level, Pin};
use embassy_rp::pio::{Pio, Instance as PioInstance, InterruptHandler as PioInterruptHandler};
use embassy_rp::dma::Channel as DmaChannel;
use embassy_net::Stack;

pub mod init;

pub struct WifiPeripherals<DMA: DmaChannel> {
    pub pwr_pin: Peri<'static, PIN_23>,
    pub cs_pin: Peri<'static, PIN_25>,
    pub dio_pin: Peri<'static, PIN_24>,
    pub clk_pin: Peri<'static, PIN_29>,
    pub pio: Peri<'static, PIO0>,
    pub dma: Peri<'static, DMA>,
}

pub struct WifiPio<PIO: PioInstance + 'static> {
    pub cs: Output<'static>,
    pub pio: Pio<'static, PIO>,
}

impl<PIO: PioInstance + 'static> WifiPio<PIO> {
    pub fn new(cs_pin: Peri<'static, impl Pin>, pio_instance: Peri<'static, PIO>, irq: impl Binding<PIO::Interrupt, PioInterruptHandler<PIO>>) -> Self {
        WifiPio {
            cs: Output::new(cs_pin, Level::High),
            pio: Pio::new(pio_instance, irq)
        }
    }
}

pub struct WifiDriver {
    pub control: cyw43::Control<'static>,
    pub stack: Stack<'static>,
}

impl WifiDriver {
    pub async fn connect(&mut self, network: &str, password: &str) -> Result<(), ControlError> {
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
