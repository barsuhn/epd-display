use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;
use defmt::info;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use crate::{WifiDriver, WifiPeripherals};

bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => InterruptHandler<PIO0>;
    }
);

pub async fn init_wifi(spawner: &Spawner, peripherals: WifiPeripherals, config: Config) -> WifiDriver {
    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(peripherals.pwr_pin, Level::Low);
    let cs = Output::new(peripherals.cs_pin, Level::High);
    let mut pio = Pio::new(peripherals.pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        peripherals.dio_pin,
        peripherals.clk_pin,
        peripherals.dma,
    );

    static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = CYW43_STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) =cyw43::new(state, pwr, spi, fw).await;

    match spawner.spawn(cyw43_task(runner)) {
        Ok(_) => info!("Cyw43 runner task spawned"),
        Err(_) => info!("Cyw43 runner task failed")
    };

    // This allocates memory for 3 sockets. DHCP and DNS each require one socket.
    static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let mut rng = RoscRng;
    let seed = rng.next_u64();
    let (stack, runner) = embassy_net::new(net_device, config, STACK_RESOURCES.init(StackResources::new()), seed);

    match spawner.spawn(net_task(runner)) {
        Ok(_) => info!("Net runner task spawned"),
        Err(_) => info!("Net runner task failed")
    }

    control.init(clm).await;
    control
          .set_power_management(cyw43::PowerManagementMode::PowerSave)
          .await;

    WifiDriver {
        control,
        stack,
    }
}

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
