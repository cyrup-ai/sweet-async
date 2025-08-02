//! Memory usage tracking implementation for Tokio tasks
//!
//! This module provides the TokioMemoryUsage struct that implements the MemoryUsage trait
//! from the API, providing thread-safe, lock-free memory usage tracking with atomic operations.

use std::sync::atomic::{AtomicU64, Ordering};

use sweet_async_api::task::MemoryUsage;

/// Concrete implementation of MemoryUsage trait for Tokio
/// 
/// Uses atomic operations for thread-safe, lock-free memory usage tracking.
/// All measurements are stored in atomic u64 values to ensure blazing-fast
/// access without any locking overhead.
pub struct TokioMemoryUsage {
    /// Current memory usage in bytes
    current_bytes: AtomicU64,
    /// Peak memory usage in bytes since creation
    peak_bytes: AtomicU64,
    /// Total number of allocations performed
    allocation_count: AtomicU64,
    /// Allocation rate in bytes per second * 100
    /// This scaling avoids floating point operations in hot paths
    allocation_rate_scaled: AtomicU64,
}

impl Clone for TokioMemoryUsage {
    fn clone(&self) -> Self {
        Self {
            current_bytes: AtomicU64::new(self.current_bytes.load(Ordering::Relaxed)),
            peak_bytes: AtomicU64::new(self.peak_bytes.load(Ordering::Relaxed)),
            allocation_count: AtomicU64::new(self.allocation_count.load(Ordering::Relaxed)),
            allocation_rate_scaled: AtomicU64::new(self.allocation_rate_scaled.load(Ordering::Relaxed)),
        }
    }
}

impl TokioMemoryUsage {
    /// Create a new TokioMemoryUsage instance with zero-initialized metrics
    #[inline]
    pub fn new() -> Self {
        Self {
            current_bytes: AtomicU64::new(0),
            peak_bytes: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
            allocation_rate_scaled: AtomicU64::new(0),
        }
    }

    /// Update current memory usage (internal use)
    /// 
    /// This method provides zero-allocation updates to memory metrics
    /// using relaxed ordering for maximum performance. Automatically
    /// tracks peak usage.
    #[inline]
    pub fn update_current_bytes(&self, bytes: u64) {
        self.current_bytes.store(bytes, Ordering::Relaxed);
        
        // Update peak if current exceeds it (atomic compare-and-swap loop)
        let mut current_peak = self.peak_bytes.load(Ordering::Relaxed);
        while bytes > current_peak {
            match self.peak_bytes.compare_exchange_weak(
                current_peak,
                bytes,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }
    }

    /// Update allocation count (internal use)
    /// 
    /// Atomically increments the allocation counter for zero-allocation tracking.
    #[inline]
    pub fn increment_allocation_count(&self) {
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update allocation rate (internal use)
    /// 
    /// Stores allocation rate as scaled integer (rate * 100) to avoid
    /// floating point operations in performance-critical paths.
    #[inline]
    pub fn update_allocation_rate(&self, rate_bytes_per_sec: f64) {
        let scaled = (rate_bytes_per_sec * 100.0) as u64;
        self.allocation_rate_scaled.store(scaled, Ordering::Relaxed);
    }
}

impl Default for TokioMemoryUsage {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryUsage for TokioMemoryUsage {
    #[inline]
    fn current_bytes(&self) -> u64 {
        self.current_bytes.load(Ordering::Relaxed)
    }

    #[inline]
    fn peak_bytes(&self) -> u64 {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    #[inline]
    fn allocation_count(&self) -> u64 {
        self.allocation_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn allocation_rate(&self) -> f64 {
        self.allocation_rate_scaled.load(Ordering::Relaxed) as f64 / 100.0
    }
}