#![no_std]

use cyw43::{ControlError, JoinOptions};
use defmt::{info, warn};
use embassy_rp::Peri;
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_net::Stack;

pub mod init;

pub struct WifiPeripherals {
    pub pwr_pin: Peri<'static, PIN_23>,
    pub cs_pin: Peri<'static, PIN_25>,
    pub dio_pin: Peri<'static, PIN_24>,
    pub clk_pin: Peri<'static, PIN_29>,
    pub pio: Peri<'static, PIO0>,
    pub dma: Peri<'static, DMA_CH0>,
}

pub struct WifiDriver {
    pub control: cyw43::Control<'static>,
    pub stack: Stack<'static>,
}

impl WifiDriver {
    pub async fn connect(&mut self, network: &str, password: &str) -> Result<(), ControlError> {
        self.control.join(network, JoinOptions::new(password.as_bytes())).await?;

        info!("waiting for link...");
        self.stack.wait_link_up().await;

        info!("waiting for DHCP...");
        self.stack.wait_config_up().await;

        info!("Stack is up!");

        if let Some(config) = self.stack.config_v4() {
            info!("IP address: {}", config.address);
            info!("Gateway: {:?}", config.gateway);
            info!("DNS servers: {:?}", config.dns_servers);
        } else {
            warn!("No IPv4 configuration available");
        }

        Ok(())
    }
}
