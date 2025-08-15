//! Extension trait for ergonomic time constants (e.g., 30.seconds(), 5.minutes())
use std::time::Duration;

pub trait TimeExt {
    fn nanos(self) -> Duration;
    fn micros(self) -> Duration;
    fn millis(self) -> Duration;
    fn seconds(self) -> Duration;
    fn minutes(self) -> Duration;
    fn hours(self) -> Duration;
    fn days(self) -> Duration;
}

impl TimeExt for u64 {
    fn nanos(self) -> Duration {
        Duration::from_nanos(self)
    }
    fn micros(self) -> Duration {
        Duration::from_micros(self)
    }
    fn millis(self) -> Duration {
        Duration::from_millis(self)
    }
    fn seconds(self) -> Duration {
        Duration::from_secs(self)
    }
    fn minutes(self) -> Duration {
        Duration::from_secs(self * 60)
    }
    fn hours(self) -> Duration {
        Duration::from_secs(self * 60 * 60)
    }
    fn days(self) -> Duration {
        Duration::from_secs(self * 60 * 60 * 24)
    }
}

impl TimeExt for usize {
    fn nanos(self) -> Duration {
        (self as u64).nanos()
    }
    fn micros(self) -> Duration {
        (self as u64).micros()
    }
    fn millis(self) -> Duration {
        (self as u64).millis()
    }
    fn seconds(self) -> Duration {
        (self as u64).seconds()
    }
    fn minutes(self) -> Duration {
        (self as u64).minutes()
    }
    fn hours(self) -> Duration {
        (self as u64).hours()
    }
    fn days(self) -> Duration {
        (self as u64).days()
    }
}
