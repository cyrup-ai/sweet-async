//! CPU usage tracking implementation for Tokio tasks
//!
//! This module provides the TokioCpuUsage struct that implements the CpuUsage trait
//! from the API, providing thread-safe, lock-free CPU usage tracking with atomic operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sweet_async_api::task::CpuUsage;

/// Concrete implementation of CpuUsage trait for Tokio
/// 
/// Uses atomic operations for thread-safe, lock-free CPU usage tracking.
/// All measurements are stored in atomic u64 values to ensure blazing-fast
/// access without any locking overhead.
pub struct TokioCpuUsage {
    /// Total CPU time in nanoseconds
    total_time_nanos: AtomicU64,
    /// Utilization as percentage * 100 (e.g., 50.0% = 5000)
    /// This scaling avoids floating point operations in hot paths
    utilization_scaled: AtomicU64,
    /// User mode CPU time in nanoseconds
    user_time_nanos: AtomicU64,
    /// System mode CPU time in nanoseconds
    system_time_nanos: AtomicU64,
}

impl Clone for TokioCpuUsage {
    fn clone(&self) -> Self {
        Self {
            total_time_nanos: AtomicU64::new(self.total_time_nanos.load(Ordering::Relaxed)),
            utilization_scaled: AtomicU64::new(self.utilization_scaled.load(Ordering::Relaxed)),
            user_time_nanos: AtomicU64::new(self.user_time_nanos.load(Ordering::Relaxed)),
            system_time_nanos: AtomicU64::new(self.system_time_nanos.load(Ordering::Relaxed)),
        }
    }
}

impl TokioCpuUsage {
    /// Create a new TokioCpuUsage instance with zero-initialized metrics
    #[inline]
    pub fn new() -> Self {
        Self {
            total_time_nanos: AtomicU64::new(0),
            utilization_scaled: AtomicU64::new(0),
            user_time_nanos: AtomicU64::new(0),
            system_time_nanos: AtomicU64::new(0),
        }
    }

    /// Update CPU time measurements (internal use)
    /// 
    /// This method provides zero-allocation updates to CPU metrics
    /// using relaxed ordering for maximum performance.
    #[inline]
    pub fn update_cpu_time(&self, total_nanos: u64, user_nanos: u64, system_nanos: u64) {
        self.total_time_nanos.store(total_nanos, Ordering::Relaxed);
        self.user_time_nanos.store(user_nanos, Ordering::Relaxed);
        self.system_time_nanos.store(system_nanos, Ordering::Relaxed);
    }

    /// Update CPU utilization (internal use)
    /// 
    /// Stores utilization as scaled integer (percentage * 100) to avoid
    /// floating point operations in performance-critical paths.
    #[inline]
    pub fn update_utilization(&self, utilization_percent: f64) {
        let scaled = (utilization_percent * 100.0) as u64;
        self.utilization_scaled.store(scaled, Ordering::Relaxed);
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
        Duration::from_nanos(self.total_time_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn utilization(&self) -> f64 {
        self.utilization_scaled.load(Ordering::Relaxed) as f64 / 100.0
    }

    #[inline]
    fn user_time(&self) -> Duration {
        Duration::from_nanos(self.user_time_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn system_time(&self) -> Duration {
        Duration::from_nanos(self.system_time_nanos.load(Ordering::Relaxed))
    }
}