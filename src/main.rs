#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use panic_rtt_target as _;

mod cli;
mod config;
mod geiger_manager;
mod tube;

use geiger_manager::RAD_DATA_CHANNEL;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn data_consumer() {
    loop {
        let data = RAD_DATA_CHANNEL.receive().await;
        info!("{}", data);
    }
}

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let hal_config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(hal_config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let mut config_store = config::ConfigStore::new(peripherals.FLASH);
    let (divider, polarity) = config_store.init();

    info!("CPM divider: {}", divider);
    info!("Pulse polarity: {}", polarity);

    let usb = esp_hal::usb_serial_jtag::UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();

    cli::spawn(spawner, usb, config_store);
    geiger_manager::spawn_tasks(spawner, peripherals.GPIO4);
    spawner.spawn(data_consumer().unwrap());
}
