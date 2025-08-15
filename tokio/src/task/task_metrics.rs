//! High-performance task metrics implementation with zero allocation

use crate::task::{TokioCpuUsage, TokioIoUsage, TokioMemoryUsage};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sweet_async_api::task::MetricsEnabledTask;

/// Zero-allocation task metrics implementation
#[derive(Debug)]
pub struct TokioTaskMetrics {
    metrics_enabled: AtomicBool,
    cpu: TokioCpuUsage,
    memory: TokioMemoryUsage,
    io: TokioIoUsage,
    start_time: AtomicU64,
    end_time: AtomicU64,
}

impl TokioTaskMetrics {
    #[inline]
    pub fn new() -> Self {
        Self {
            metrics_enabled: AtomicBool::new(false),
            cpu: TokioCpuUsage::new(),
            memory: TokioMemoryUsage::new(),
            io: TokioIoUsage::new(),
            start_time: AtomicU64::new(0),
            end_time: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn record_start(&self) {
        if self.metrics_enabled.load(Ordering::Relaxed) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            self.start_time.store(now, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn record_end(&self) {
        if self.metrics_enabled.load(Ordering::Relaxed) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            self.end_time.store(now, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn enable_metrics(&self) {
        self.metrics_enabled.store(true, Ordering::Relaxed);
    }

    #[inline]
    pub fn disable_metrics(&self) {
        self.metrics_enabled.store(false, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_metrics_enabled(&self) -> bool {
        self.metrics_enabled.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn execution_time(&self) -> Option<Duration> {
        if !self.is_metrics_enabled() {
            return None;
        }

        let start = self.start_time.load(Ordering::Relaxed);
        let end = self.end_time.load(Ordering::Relaxed);

        if start > 0 && end > start {
            Some(Duration::from_nanos(end - start))
        } else {
            None
        }
    }
}

impl Default for TokioTaskMetrics {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MetricsEnabledTask<T> for TokioTaskMetrics
where
    T: Clone + Send + 'static,
{
    type Cpu = TokioCpuUsage;
    type Memory = TokioMemoryUsage;
    type Io = TokioIoUsage;

    #[inline]
    fn cpu_usage(&self) -> &Self::Cpu {
        &self.cpu
    }

    #[inline]
    fn memory_usage(&self) -> &Self::Memory {
        &self.memory
    }

    #[inline]
    fn io_usage(&self) -> &Self::Io {
        &self.io
    }
}
