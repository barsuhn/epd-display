use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use embassy_executor::{SpawnError, Spawner};
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;
use defmt::info;
use embassy_rp::bind_interrupts;
use embassy_rp::dma::Channel;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, DMA_CH4, DMA_CH5, DMA_CH6, DMA_CH7, DMA_CH8, DMA_CH9, DMA_CH10, DMA_CH11, PIO0};
use crate::{WifiDriver, WifiPeripherals};

static FIRMWARE: &[u8]  = include_bytes!("../../cyw43-firmware/43439A0.bin");
static FIRMWARE_CLM: &[u8] = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => InterruptHandler<PIO0>;
    }
);

pub trait SpawnCyw43Task {
    fn spawn_task(spawner: &Spawner, runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, Self>>) -> Result<(), SpawnError>
    where Self: Channel;
}

pub async fn init_wifi<DMA: Channel + SpawnCyw43Task>(spawner: &Spawner, peripherals: WifiPeripherals<DMA>, config: Config) -> WifiDriver {
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
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FIRMWARE).await;

    match DMA::spawn_task(spawner, runner) {
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

    control.init(FIRMWARE_CLM).await;
    control
          .set_power_management(cyw43::PowerManagementMode::PowerSave)
          .await;

    WifiDriver {
        control,
        stack,
    }
}

macro_rules! create_cyw43_task {
    ($name:ident, $dma:ty) => {
        impl SpawnCyw43Task for $dma {
            fn spawn_task(spawner: &Spawner, runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, Self>>) -> Result<(), SpawnError> {
                spawner.spawn($name(runner))
            }
        }

        #[embassy_executor::task]
        async fn $name(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, $dma>>) -> ! {
            runner.run().await
        }
    };
}

create_cyw43_task!(cyw43_task_ch0, DMA_CH0);
create_cyw43_task!(cyw43_task_ch1, DMA_CH1);
create_cyw43_task!(cyw43_task_ch2, DMA_CH2);
create_cyw43_task!(cyw43_task_ch3, DMA_CH3);
create_cyw43_task!(cyw43_task_ch4, DMA_CH4);
create_cyw43_task!(cyw43_task_ch5, DMA_CH5);
create_cyw43_task!(cyw43_task_ch6, DMA_CH6);
create_cyw43_task!(cyw43_task_ch7, DMA_CH7);
create_cyw43_task!(cyw43_task_ch8, DMA_CH8);
create_cyw43_task!(cyw43_task_ch9, DMA_CH9);
create_cyw43_task!(cyw43_task_ch10, DMA_CH10);
create_cyw43_task!(cyw43_task_ch11, DMA_CH11);

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
