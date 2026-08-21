#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use panic_rtt_target as _;

use esp_hal::pcnt::Pcnt;

mod geiger_manager;
mod radiation_data;

use geiger_manager::GeigerManager;

use crate::geiger_manager::RAD_DATA_SIGNAL;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn data_consumer() {
    loop {
        let data = RAD_DATA_SIGNAL.wait().await;
        info!("{}", data);
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let pulse_pin = peripherals.GPIO4;
    let pcnt = Pcnt::new(peripherals.PCNT);

    // 153.8 CPM / µSv/h for M4011 (J305), the RadiationD v1.1 (CAJOE) default tube
    // 318.0 CPM / µSv/h for SBT-11A
    const CPM_RATIO: f32 = 318.0;

    let geiger = GeigerManager::new(pcnt, pulse_pin, CPM_RATIO);

    geiger_manager::spawn_tasks(spawner, geiger);

    spawner.spawn(data_consumer().unwrap());
}
