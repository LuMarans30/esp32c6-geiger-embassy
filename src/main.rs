#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::pcnt::Pcnt;
use esp_hal::timer::systimer::SystemTimer;
use log::info;
use static_cell::StaticCell;

use crate::geiger_manager::GeigerManager;

mod geiger_manager;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

static GEIGER_MANAGER: StaticCell<GeigerManager<'static>> = StaticCell::new();

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const CPM_RATIO: f32 = 153.8; // CPM per µSv/h for M4011 (J305), the RadiationD v1.1 (CAJOE) default tube

#[embassy_executor::task]
async fn run(geiger: &'static mut GeigerManager<'static>) {
    const SAMPLE_MS: u64 = 100;
    const WINDOW_SIZE: usize = 600; // 60 sec * 10 samples/sec

    let mut history = [0u32; WINDOW_SIZE];
    let mut idx = 0;
    let mut total = geiger.get_total_counts();
    let mut tick = 0;

    loop {
        Timer::after(Duration::from_millis(SAMPLE_MS)).await;

        let new_total = geiger.get_total_counts();
        history[idx] = new_total.saturating_sub(total);
        total = new_total;
        idx = (idx + 1) % WINDOW_SIZE;
        tick += 1;

        if tick % 10 == 0 {
            let collected = (tick.min(WINDOW_SIZE) / 10) as f32; // seconds
            let cpm = (history.iter().sum::<u32>() as f32 / collected) * 60.0;
            let dose = cpm / CPM_RATIO;

            info!(
                "Total: {} | CPM: {:.1} | Dose: {:.3} µSv/h (window: {:.1}s)",
                total, cpm, dose, collected
            );
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    // Wi-Fi connection init
    //let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    //let timer1 = TimerGroup::new(peripherals.TIMG0);
    //let wifi_init =
    //    esp_wifi::init(timer1.timer0, rng).expect("Failed to initialize WIFI/BLE controller");
    //let (mut _wifi_controller, _interfaces) = esp_wifi::wifi::new(&wifi_init, peripherals.WIFI)
    //    .expect("Failed to initialize WIFI controller");

    // Bluetooth connection init
    //let transport = BleConnector::new(&wifi_init, peripherals.BT);
    //let _ble_controller = ExternalController::<_, 20>::new(transport);

    // GPIO pin used to detect pulses
    let pulse_pin = peripherals.GPIO4;
    let pcnt = Pcnt::new(peripherals.PCNT);

    let geiger = GeigerManager::new(pcnt, pulse_pin);

    // Store the geiger manager in a static cell to use it in tasks
    let geiger_static = GEIGER_MANAGER.init(geiger);

    spawner.spawn(run(geiger_static)).ok();
}
