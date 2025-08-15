//! High-performance timed task implementation

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sweet_async_api::task::timed_task::TimedTask;

/// Zero-allocation timed task with atomic timestamp tracking
#[derive(Debug)]
pub struct TokioTimedTask {
    created_timestamp_nanos: AtomicU64,
    executed_timestamp_nanos: AtomicU64,
    completed_timestamp_nanos: AtomicU64,
    timeout_nanos: AtomicU64,
}

impl TokioTimedTask {
    #[inline]
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        Self {
            created_timestamp_nanos: AtomicU64::new(now),
            executed_timestamp_nanos: AtomicU64::new(0),
            completed_timestamp_nanos: AtomicU64::new(0),
            timeout_nanos: AtomicU64::new(Duration::from_secs(30).as_nanos() as u64),
        }
    }

    #[inline]
    pub fn with_timeout(timeout: Duration) -> Self {
        let mut task = Self::new();
        task.timeout_nanos
            .store(timeout.as_nanos() as u64, Ordering::Relaxed);
        task
    }

    #[inline]
    pub fn mark_executed(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        self.executed_timestamp_nanos.store(now, Ordering::Relaxed);
    }

    #[inline]
    pub fn mark_completed(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        self.completed_timestamp_nanos.store(now, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_timeout(&self, timeout: Duration) {
        self.timeout_nanos
            .store(timeout.as_nanos() as u64, Ordering::Relaxed);
    }
}

impl Default for TokioTimedTask {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TimedTask<T> for TokioTimedTask
where
    T: Send + 'static,
{
    #[inline]
    fn created_timestamp(&self) -> SystemTime {
        let nanos = self.created_timestamp_nanos.load(Ordering::Relaxed);
        UNIX_EPOCH + Duration::from_nanos(nanos)
    }

    #[inline]
    fn executed_timestamp(&self) -> SystemTime {
        let nanos = self.executed_timestamp_nanos.load(Ordering::Relaxed);
        UNIX_EPOCH + Duration::from_nanos(nanos)
    }

    #[inline]
    fn completed_timestamp(&self) -> SystemTime {
        let nanos = self.completed_timestamp_nanos.load(Ordering::Relaxed);
        UNIX_EPOCH + Duration::from_nanos(nanos)
    }

    #[inline]
    fn timeout(&self) -> Duration {
        Duration::from_nanos(self.timeout_nanos.load(Ordering::Relaxed))
    }
}
