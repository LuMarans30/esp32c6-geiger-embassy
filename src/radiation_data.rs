use alloc::fmt;
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

impl fmt::Display for RadiationData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Total counts: {} | CPM: {:.1} | Dose: {:.3} µSv/h | Accum: {:.3} µSv",
            self.total_counts, self.cpm, self.dose_rate_usv_h, self.accumulated_usv
        )
    }
}
