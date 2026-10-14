use cyw43::{aligned_bytes, Aligned, A4, SpiBus};
use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::PIO0;
use static_cell::StaticCell;
use defmt::{debug, error};

use crate::WifiDriver;

static FIRMWARE: &Aligned<A4, [u8]> = aligned_bytes!("../../cyw43-firmware/43439A0.bin");
static FIRMWARE_CLM: &Aligned<A4, [u8]> = aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
static NVRAM: &Aligned<A4, [u8]> = aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");

pub type WifiSpi = PioSpi<'static, PIO0, 0>;
type Cyw43Runner = cyw43::Runner<'static, SpiBus<Output<'static>, WifiSpi>>;

pub async fn init_wifi(spawner: &Spawner, pwr: Output<'static>, spi: WifiSpi, config: Config) -> WifiDriver {
    static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();
    
    let state = CYW43_STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FIRMWARE, NVRAM).await;

    match cyw43_task(runner).map(|token| spawner.spawn(token)) {
        Ok(_) => debug!("Cyw43 runner task spawned"),
        Err(_) => error!("Cyw43 runner task failed")
    };

    // This allocates memory for 3 sockets. DHCP and DNS each require one socket.
    static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    
    let mut rng = RoscRng;
    let seed = rng.next_u64();
    let (stack, runner) = embassy_net::new(net_device, config, STACK_RESOURCES.init(StackResources::new()), seed);

    match net_task(runner).map(|token| spawner.spawn(token)) {
        Ok(_) => debug!("Net runner task spawned"),
        Err(_) => error!("Net runner task failed")
    }

    control.init(FIRMWARE_CLM).await;
    control
          .set_power_management(cyw43::PowerManagementMode::PowerSave)
          .await;

    WifiDriver {
        control,
        stack,
    }
}

#[embassy_executor::task]
async fn cyw43_task(runner: Cyw43Runner) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
