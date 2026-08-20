#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::pcnt::Pcnt;
use log::info;

use crate::geiger_manager::GeigerManager;
use crate::radiation_data::RadiationData;

mod geiger_manager;
mod radiation_data;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

static RAD_DATA_SIGNAL: Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    RadiationData,
> = Signal::new();

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn geiger_sampler(geiger: GeigerManager<'static>) {
    const SAMPLE_MS: u64 = 100;
    const WINDOW_SIZE: usize = 600; // 60 sec * 10 samples/sec

    let mut history = [0u32; WINDOW_SIZE];
    let mut idx = 0;
    let mut total = geiger.get_total_counts();
    let mut tick = 0u64;
    let mut accumulated_usv: f32 = 0.0;

    loop {
        Timer::after(Duration::from_millis(SAMPLE_MS)).await;

        let new_total = geiger.get_total_counts();
        let delta = new_total.saturating_sub(total);
        history[idx] = delta;
        total = new_total;
        idx = (idx + 1) % WINDOW_SIZE;
        tick += 1;

        if tick.is_multiple_of(10) {
            let collected_secs = (tick.min(WINDOW_SIZE as u64) / 10) as f32;
            let cpm = (history.iter().sum::<u32>() as f32 / collected_secs) * 60.0;
            let dose_rate = cpm / geiger.cpm_ratio;

            accumulated_usv += dose_rate / 3600.0;

            let data = RadiationData {
                total_counts: total,
                cpm,
                dose_rate_usv_h: dose_rate,
                accumulated_usv,
            };

            RAD_DATA_SIGNAL.signal(data);
        }
    }
}

#[embassy_executor::task]
async fn data_consumer() {
    loop {
        let data = RAD_DATA_SIGNAL.wait().await;
        info!("{}", data);
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    info!("esp-rtos initialized!");

    let pulse_pin = peripherals.GPIO4;
    let pcnt = Pcnt::new(peripherals.PCNT);

    // 153.8 CPM / µSv/h for M4011 (J305), the RadiationD v1.1 (CAJOE) default tube
    // 318.0 CPM / µSv/h for SBT-11A
    const CPM_RATIO: f32 = 318.0;

    let geiger = GeigerManager::new(pcnt, pulse_pin, CPM_RATIO);

    spawner.spawn(geiger_sampler(geiger).unwrap());
    spawner.spawn(data_consumer().unwrap());
}
