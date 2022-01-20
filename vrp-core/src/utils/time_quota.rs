use crate::construction::Quota;
use rosomaxa::utils::Timer;

/// A time quota.
pub struct TimeQuota {
    start: Timer,
    limit_in_secs: f64,
}

impl TimeQuota {
    /// Creates a new instance of `TimeQuota`.
    pub fn new(limit_in_secs: f64) -> Self {
        Self { start: Timer::start(), limit_in_secs }
    }
}

impl Quota for TimeQuota {
    fn is_reached(&self) -> bool {
        self.start.elapsed_secs_as_f64() > self.limit_in_secs
    }
}
