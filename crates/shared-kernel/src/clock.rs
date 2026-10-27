use time::OffsetDateTime;

/// Source of the current UTC time; injected so tests are deterministic.
pub trait Clock: Send + Sync {
    /// Current time in UTC.
    fn now(&self) -> OffsetDateTime;
}

/// Wall-clock implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}
