// geiger_manager.rs
use crate::radiation_data::RadiationData;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{Input, InputConfig, Pull, interconnect::PeripheralInput},
    pcnt::{Pcnt, channel::EdgeMode, unit::Unit},
};

pub static RAD_DATA_SIGNAL: Signal<CriticalSectionRawMutex, RadiationData> = Signal::new();

pub struct GeigerManager<'a> {
    unit: Unit<'a, 0>,
    pub cpm_ratio: f32,
}

impl<'a> GeigerManager<'a> {
    pub fn new<T: PeripheralInput<'a> + esp_hal::gpio::InputPin>(
        pcnt: Pcnt<'a>,
        pulse_pin: T,
        cpm_ratio: f32,
    ) -> Self {
        let unit: esp_hal::pcnt::unit::Unit<'_, 0> = pcnt.unit0;

        unit.set_low_limit(Some(-32767)).ok();
        unit.set_high_limit(Some(32767)).ok();

        unit.set_filter(Some(10)).ok();

        unit.clear();

        let config = InputConfig::default().with_pull(Pull::None);
        let pulse_pin: Input<'_> = Input::new(pulse_pin, config);
        let pulse_signal = pulse_pin.peripheral_input();

        unit.channel0.set_edge_signal(pulse_signal);
        unit.channel0
            .set_input_mode(EdgeMode::Hold, EdgeMode::Increment);

        Self { unit, cpm_ratio }
    }

    pub fn get_total_counts(&self) -> u32 {
        self.unit.counter.get() as u32
    }
}

#[embassy_executor::task]
async fn geiger_sampler(geiger: GeigerManager<'static>) {
    const SAMPLE_MS: u64 = 100;
    const WINDOW_SIZE: usize = 600;

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

            RAD_DATA_SIGNAL.signal(RadiationData {
                total_counts: total,
                cpm,
                dose_rate_usv_h: dose_rate,
                accumulated_usv,
            });
        }
    }
}

pub fn spawn_tasks(spawner: Spawner, geiger: GeigerManager<'static>) {
    spawner.spawn(geiger_sampler(geiger).unwrap());
}
