//! High-performance CPU usage implementation with zero allocation

use std::time::Duration;
use sweet_async_api::task::CpuUsage;

/// Zero-allocation CPU usage implementation
#[derive(Debug, Clone, Copy)]
pub struct TokioCpuUsage {
    cpu_time_nanos: u64,
    utilization: f64,
    user_time_nanos: u64,
    system_time_nanos: u64,
}

impl TokioCpuUsage {
    #[inline]
    pub fn new() -> Self {
        Self {
            cpu_time_nanos: 0,
            utilization: 0.0,
            user_time_nanos: 0,
            system_time_nanos: 0,
        }
    }

    #[inline]
    pub fn with_values(
        cpu_time: Duration,
        utilization: f64,
        user_time: Duration,
        system_time: Duration,
    ) -> Self {
        Self {
            cpu_time_nanos: cpu_time.as_nanos() as u64,
            utilization,
            user_time_nanos: user_time.as_nanos() as u64,
            system_time_nanos: system_time.as_nanos() as u64,
        }
    }
}

impl Default for TokioCpuUsage {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl CpuUsage for TokioCpuUsage {
    #[inline]
    fn cpu_time(&self) -> Duration {
        Duration::from_nanos(self.cpu_time_nanos)
    }

    #[inline]
    fn utilization(&self) -> f64 {
        self.utilization
    }

    #[inline]
    fn user_time(&self) -> Duration {
        Duration::from_nanos(self.user_time_nanos)
    }

    #[inline]
    fn system_time(&self) -> Duration {
        Duration::from_nanos(self.system_time_nanos)
    }
}
