//! I/O usage tracking implementation for Tokio tasks
//!
//! This module provides the TokioIoUsage struct that implements the IoUsage trait
//! from the API, providing thread-safe, lock-free I/O usage tracking with atomic operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sweet_async_api::task::IoUsage;

/// Concrete implementation of IoUsage trait for Tokio
/// 
/// Uses atomic operations for thread-safe, lock-free I/O usage tracking.
/// All measurements are stored in atomic u64 values to ensure blazing-fast
/// access without any locking overhead.
pub struct TokioIoUsage {
    /// Total bytes read from I/O operations
    bytes_read: AtomicU64,
    /// Total bytes written to I/O operations
    bytes_written: AtomicU64,
    /// Number of read operations performed
    read_operations: AtomicU64,
    /// Number of write operations performed
    write_operations: AtomicU64,
    /// Average read latency in nanoseconds
    read_latency_nanos: AtomicU64,
    /// Average write latency in nanoseconds
    write_latency_nanos: AtomicU64,
    /// Operations per second * 100
    /// This scaling avoids floating point operations in hot paths
    ops_per_second_scaled: AtomicU64,
    /// Total time spent waiting for I/O in nanoseconds
    io_wait_nanos: AtomicU64,
}

impl Clone for TokioIoUsage {
    fn clone(&self) -> Self {
        Self {
            bytes_read: AtomicU64::new(self.bytes_read.load(Ordering::Relaxed)),
            bytes_written: AtomicU64::new(self.bytes_written.load(Ordering::Relaxed)),
            read_operations: AtomicU64::new(self.read_operations.load(Ordering::Relaxed)),
            write_operations: AtomicU64::new(self.write_operations.load(Ordering::Relaxed)),
            read_latency_nanos: AtomicU64::new(self.read_latency_nanos.load(Ordering::Relaxed)),
            write_latency_nanos: AtomicU64::new(self.write_latency_nanos.load(Ordering::Relaxed)),
            ops_per_second_scaled: AtomicU64::new(self.ops_per_second_scaled.load(Ordering::Relaxed)),
            io_wait_nanos: AtomicU64::new(self.io_wait_nanos.load(Ordering::Relaxed)),
        }
    }
}

impl TokioIoUsage {
    /// Create a new TokioIoUsage instance with zero-initialized metrics
    #[inline]
    pub fn new() -> Self {
        Self {
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            read_operations: AtomicU64::new(0),
            write_operations: AtomicU64::new(0),
            read_latency_nanos: AtomicU64::new(0),
            write_latency_nanos: AtomicU64::new(0),
            ops_per_second_scaled: AtomicU64::new(0),
            io_wait_nanos: AtomicU64::new(0),
        }
    }

    /// Update read metrics (internal use)
    /// 
    /// Atomically updates read-related metrics for zero-allocation tracking.
    #[inline]
    pub fn update_read_metrics(&self, bytes: u64, latency_nanos: u64) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.read_operations.fetch_add(1, Ordering::Relaxed);
        
        // Update average read latency using exponential moving average
        let current_latency = self.read_latency_nanos.load(Ordering::Relaxed);
        let new_latency = if current_latency == 0 {
            latency_nanos
        } else {
            (current_latency * 7 + latency_nanos) / 8  // EMA with alpha = 0.125
        };
        self.read_latency_nanos.store(new_latency, Ordering::Relaxed);
    }

    /// Update write metrics (internal use)
    /// 
    /// Atomically updates write-related metrics for zero-allocation tracking.
    #[inline]
    pub fn update_write_metrics(&self, bytes: u64, latency_nanos: u64) {
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.write_operations.fetch_add(1, Ordering::Relaxed);
        
        // Update average write latency using exponential moving average
        let current_latency = self.write_latency_nanos.load(Ordering::Relaxed);
        let new_latency = if current_latency == 0 {
            latency_nanos
        } else {
            (current_latency * 7 + latency_nanos) / 8  // EMA with alpha = 0.125
        };
        self.write_latency_nanos.store(new_latency, Ordering::Relaxed);
    }

    /// Update operations per second (internal use)
    /// 
    /// Stores ops per second as scaled integer (ops * 100) to avoid
    /// floating point operations in performance-critical paths.
    #[inline]
    pub fn update_ops_per_second(&self, ops_per_sec: f64) {
        let scaled = (ops_per_sec * 100.0) as u64;
        self.ops_per_second_scaled.store(scaled, Ordering::Relaxed);
    }

    /// Update I/O wait time (internal use)
    /// 
    /// Atomically adds to the total I/O wait time for zero-allocation tracking.
    #[inline]
    pub fn add_io_wait_time(&self, wait_nanos: u64) {
        self.io_wait_nanos.fetch_add(wait_nanos, Ordering::Relaxed);
    }
}

impl Default for TokioIoUsage {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl IoUsage for TokioIoUsage {
    #[inline]
    fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    #[inline]
    fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    #[inline]
    fn read_operations(&self) -> u64 {
        self.read_operations.load(Ordering::Relaxed)
    }

    #[inline]
    fn write_operations(&self) -> u64 {
        self.write_operations.load(Ordering::Relaxed)
    }

    #[inline]
    fn read_latency(&self) -> Duration {
        Duration::from_nanos(self.read_latency_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn write_latency(&self) -> Duration {
        Duration::from_nanos(self.write_latency_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn operations_per_second(&self) -> f64 {
        self.ops_per_second_scaled.load(Ordering::Relaxed) as f64 / 100.0
    }

    #[inline]
    fn io_wait_time(&self) -> Duration {
        Duration::from_nanos(self.io_wait_nanos.load(Ordering::Relaxed))
    }
}