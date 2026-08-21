#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use panic_rtt_target as _;

mod geiger_manager;
mod radiation_data;

use geiger_manager::RAD_DATA_SIGNAL;

extern crate alloc;
esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn data_consumer() {
    loop {
        let data = RAD_DATA_SIGNAL.wait().await;
        info!("{}", data);
    }
}

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    const CPM_RATIO: f32 = 153.8; // default CAJOE tube (M4011 / J305)

    geiger_manager::spawn_tasks(spawner, peripherals.GPIO4, CPM_RATIO);
    spawner.spawn(data_consumer().unwrap());
}
