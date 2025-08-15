use std::time::{Duration, SystemTime};

pub trait TimedTask<T: Send + 'static> {
    fn created_timestamp(&self) -> SystemTime;
    fn executed_timestamp(&self) -> SystemTime;
    fn completed_timestamp(&self) -> SystemTime;
    fn timeout(&self) -> Duration;
}
