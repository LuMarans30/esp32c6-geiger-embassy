use core::sync::atomic::{AtomicU32, Ordering};

use crate::radiation_data::RadiationData;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, InputConfig, Pull, interconnect::PeripheralInput};

pub static RAD_DATA_SIGNAL: Signal<CriticalSectionRawMutex, RadiationData> = Signal::new();

static GEIGER_COUNTS: AtomicU32 = AtomicU32::new(0);

#[embassy_executor::task]
async fn geiger_edge_detector(mut pin: Input<'static>) {
    loop {
        pin.wait_for_rising_edge().await;
        GEIGER_COUNTS.fetch_add(1, Ordering::Relaxed);
        Timer::after(Duration::from_millis(10)).await;
        pin.wait_for_falling_edge().await;
    }
}

#[embassy_executor::task]
async fn geiger_sampler(cpm_ratio: f32) {
    const SAMPLE_MS: u64 = 100;
    const WINDOW_SIZE: usize = 600; // 60 sec * 10 samples/sec

    let mut history = [0u32; WINDOW_SIZE];
    let mut idx = 0;
    let mut total = 0u32;
    let mut tick = 0u64;
    let mut accumulated_usv: f32 = 0.0;

    loop {
        Timer::after(Duration::from_millis(SAMPLE_MS)).await;

        let new_total = GEIGER_COUNTS.load(Ordering::Relaxed);
        let delta = new_total.saturating_sub(total);

        history[idx] = delta;
        total = new_total;
        idx = (idx + 1) % WINDOW_SIZE;
        tick += 1;

        if tick.is_multiple_of(10) {
            let collected_secs = (tick.min(WINDOW_SIZE as u64) / 10) as f32;
            let cpm = (history.iter().sum::<u32>() as f32 / collected_secs) * 60.0;
            let dose_rate = cpm / cpm_ratio;
            accumulated_usv += dose_rate / 3600.0;

            let data = RadiationData {
                total_counts: new_total,
                cpm,
                dose_rate_usv_h: dose_rate,
                accumulated_usv,
            };
            RAD_DATA_SIGNAL.signal(data);
        }
    }
}

pub fn spawn_tasks<T: PeripheralInput<'static> + esp_hal::gpio::InputPin + 'static>(
    spawner: Spawner,
    pulse_pin: T,
    cpm_ratio: f32,
) {
    let config = InputConfig::default().with_pull(Pull::None);
    let pin = Input::new(pulse_pin, config);

    spawner.spawn(geiger_edge_detector(pin).unwrap());
    spawner.spawn(geiger_sampler(cpm_ratio).unwrap());
}
