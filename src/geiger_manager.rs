#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_hal::{
    gpio::{interconnect::PeripheralInput, Input, InputConfig, Pull},
    pcnt::{channel::EdgeMode, unit::Unit, Pcnt},
};

pub struct GeigerManager<'a> {
    unit: Unit<'a, 0>,
}

impl<'a> GeigerManager<'a> {
    pub fn new<T: PeripheralInput<'a> + esp_hal::gpio::InputPin>(
        pcnt: Pcnt<'a>,
        pulse_pin: T,
    ) -> Self {
        let unit: esp_hal::pcnt::unit::Unit<'_, 0> = pcnt.unit0;

        unit.set_low_limit(Some(-32767)).ok();
        unit.set_high_limit(Some(32767)).ok();
        unit.clear();

        let config = InputConfig::default().with_pull(Pull::None);
        let pulse_pin: Input<'_> = Input::new(pulse_pin, config);
        let pulse_signal = pulse_pin.peripheral_input();

        unit.channel0.set_edge_signal(pulse_signal);
        unit.channel0
            .set_input_mode(EdgeMode::Hold, EdgeMode::Increment); // falling edge

        Self { unit }
    }

    /// Read raw signed counter value
    pub fn get_total_counts(&self) -> u32 {
        self.unit.counter.get() as u32
    }

    /// Clear the hardware counter
    #[allow(unused)]
    pub fn clear_counter(&mut self) {
        self.unit.clear();
    }
}
