use defmt::{Formatter, write};
use serde::{Deserialize, Serialize};

/// Radiation measurement data shared between tasks
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RadiationData {
    /// Total counts since startup
    pub total_counts: u32,
    /// Counts per minute (rolling window)
    pub cpm: f32,
    /// Current dose rate
    pub dose_rate_usv_h: f32,
    /// Accumulated dose since startup
    pub accumulated_usv: f32,
}

impl defmt::Format for RadiationData {
    fn format(&self, fmt: Formatter) {
        write!(
            fmt,
            "Total counts: {} | CPM: {} | Dose: {} µSv/h | Accum: {} µSv",
            self.total_counts, self.cpm, self.dose_rate_usv_h, self.accumulated_usv
        )
    }
}
