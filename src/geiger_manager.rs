use crate::config;
use alloc::vec;
use core::sync::atomic::{AtomicU32, Ordering};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, InputConfig, Pull, interconnect::PeripheralInput};

pub static RAD_DATA_CHANNEL: Channel<CriticalSectionRawMutex, RadiationData, 8> = Channel::new();

static GEIGER_COUNTS: AtomicU32 = AtomicU32::new(0);

const SAMPLE_MS: u64 = 100;
const SAMPLES_PER_SECOND: u64 = 1000 / SAMPLE_MS;
const WINDOW_SECONDS: u64 = 60;
const WINDOW_SIZE: usize = (SAMPLES_PER_SECOND * WINDOW_SECONDS) as usize;
const REPORT_EVERY_SAMPLES: u64 = SAMPLES_PER_SECOND;
const DEBOUNCE_MS: u64 = 10;

#[derive(Clone, Copy, Debug, defmt::Format)]
pub struct RadiationData {
    pub total_counts: u64,
    pub cpm: f32,
    pub dose_rate_usv_h: f32,
    pub accumulated_usv: f32,
}

#[embassy_executor::task]
async fn geiger_edge_detector(mut pin: Input<'static>) {
    loop {
        pin.wait_for_any_edge().await;

        let trigger_on_low = config::current_polarity().is_low();
        let is_low = pin.is_low();

        if is_low == trigger_on_low {
            GEIGER_COUNTS.fetch_add(1, Ordering::Relaxed);
        }

        Timer::after(Duration::from_millis(DEBOUNCE_MS)).await;
    }
}

#[embassy_executor::task]
async fn geiger_sampler() {
    let mut history = vec![0u32; WINDOW_SIZE];
    let mut idx = 0usize;
    let mut window_sum = 0u64;
    let mut tick = 0u64;
    let mut accumulated_usv = 0.0f32;

    let mut last_raw = GEIGER_COUNTS.load(Ordering::Relaxed);
    let mut total_counts = u64::from(last_raw);

    loop {
        Timer::after(Duration::from_millis(SAMPLE_MS)).await;

        let raw = GEIGER_COUNTS.load(Ordering::Relaxed);
        let delta = raw.wrapping_sub(last_raw);

        last_raw = raw;
        total_counts += u64::from(delta);

        window_sum -= u64::from(history[idx]);
        history[idx] = delta;
        window_sum += u64::from(delta);

        idx = (idx + 1) % WINDOW_SIZE;
        tick += 1;

        if tick.is_multiple_of(REPORT_EVERY_SAMPLES) {
            let collected_secs = (tick.min(WINDOW_SIZE as u64) / SAMPLES_PER_SECOND) as f32;

            let cpm = (window_sum as f32 / collected_secs) * 60.0;
            let dose_rate_usv_h = cpm / config::current_divider();

            let report_secs = (REPORT_EVERY_SAMPLES / SAMPLES_PER_SECOND) as f32;
            accumulated_usv += dose_rate_usv_h * report_secs / 3600.0;

            let data = RadiationData {
                total_counts,
                cpm,
                dose_rate_usv_h,
                accumulated_usv,
            };

            let _ = RAD_DATA_CHANNEL.try_send(data);
        }
    }
}

pub fn spawn_tasks<T: PeripheralInput<'static> + esp_hal::gpio::InputPin + 'static>(
    spawner: Spawner,
    pulse_pin: T,
) {
    let input_config = InputConfig::default().with_pull(Pull::None);
    let pin = Input::new(pulse_pin, input_config);

    spawner.spawn(geiger_edge_detector(pin).unwrap());
    spawner.spawn(geiger_sampler().unwrap());
}
