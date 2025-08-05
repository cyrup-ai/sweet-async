//! Execution statistics implementation for Tokio orchestra
//!
//! This module provides the TokioExecutionStats struct that implements the ExecutionStats trait
//! from the API, providing thread-safe, atomic statistics tracking for task orchestration.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use sweet_async_api::orchestra::ExecutionStats;

/// Concrete implementation of ExecutionStats trait for Tokio
/// 
/// Uses atomic operations for thread-safe, lock-free statistics tracking.
/// All counters are stored in atomic usize values to ensure blazing-fast
/// access without any locking overhead.
pub struct TokioExecutionStats {
    /// Number of tasks that have completed successfully
    completed_count: AtomicUsize,
    /// Number of tasks that have failed
    failed_count: AtomicUsize,
    /// Number of tasks currently running
    running_count: AtomicUsize,
    /// Number of tasks waiting to start
    pending_count: AtomicUsize,
    /// Total number of tasks that have been scheduled
    total_scheduled: AtomicUsize,
    /// Total execution time across all completed tasks (in milliseconds)
    total_execution_time_ms: AtomicU64,
    /// Maximum execution time observed (in milliseconds)
    max_execution_time_ms: AtomicU64,
    /// Minimum execution time observed (in milliseconds)
    min_execution_time_ms: AtomicU64,
}

impl Clone for TokioExecutionStats {
    fn clone(&self) -> Self {
        Self {
            completed_count: AtomicUsize::new(self.completed_count.load(Ordering::Relaxed)),
            failed_count: AtomicUsize::new(self.failed_count.load(Ordering::Relaxed)),
            running_count: AtomicUsize::new(self.running_count.load(Ordering::Relaxed)),
            pending_count: AtomicUsize::new(self.pending_count.load(Ordering::Relaxed)),
            total_scheduled: AtomicUsize::new(self.total_scheduled.load(Ordering::Relaxed)),
            total_execution_time_ms: AtomicU64::new(self.total_execution_time_ms.load(Ordering::Relaxed)),
            max_execution_time_ms: AtomicU64::new(self.max_execution_time_ms.load(Ordering::Relaxed)),
            min_execution_time_ms: AtomicU64::new(self.min_execution_time_ms.load(Ordering::Relaxed)),
        }
    }
}

impl TokioExecutionStats {
    /// Create a new TokioExecutionStats instance with zero-initialized counters
    #[inline]
    pub fn new() -> Self {
        Self {
            completed_count: AtomicUsize::new(0),
            failed_count: AtomicUsize::new(0),
            running_count: AtomicUsize::new(0),
            pending_count: AtomicUsize::new(0),
            total_scheduled: AtomicUsize::new(0),
            total_execution_time_ms: AtomicU64::new(0),
            max_execution_time_ms: AtomicU64::new(0),
            min_execution_time_ms: AtomicU64::new(u64::MAX), // Initialize to MAX so first timing becomes min
        }
    }

    /// Increment completed task counter (internal use)
    #[inline]
    pub fn increment_completed(&self) {
        self.completed_count.fetch_add(1, Ordering::Relaxed);
        self.running_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment failed task counter (internal use)
    #[inline]
    pub fn increment_failed(&self) {
        self.failed_count.fetch_add(1, Ordering::Relaxed);
        self.running_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment running task counter (internal use)
    #[inline]
    pub fn increment_running(&self) {
        self.running_count.fetch_add(1, Ordering::Relaxed);
        self.pending_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment pending task counter (internal use)
    #[inline]
    pub fn increment_pending(&self) {
        self.pending_count.fetch_add(1, Ordering::Relaxed);
        self.total_scheduled.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the total number of scheduled tasks
    #[inline]
    pub fn total_scheduled(&self) -> usize {
        self.total_scheduled.load(Ordering::Relaxed)
    }

    /// Record the execution time of a completed task
    #[inline]
    pub fn record_execution_time(&self, duration_ms: u64) {
        self.total_execution_time_ms.fetch_add(duration_ms, Ordering::Relaxed);
        
        // Update max execution time
        self.max_execution_time_ms.fetch_max(duration_ms, Ordering::Relaxed);
        
        // Update min execution time
        self.min_execution_time_ms.fetch_min(duration_ms, Ordering::Relaxed);
    }
}

impl Default for TokioExecutionStats {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionStats for TokioExecutionStats {
    #[inline]
    fn completed_count(&self) -> usize {
        self.completed_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn failed_count(&self) -> usize {
        self.failed_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn running_count(&self) -> usize {
        self.running_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn avg_execution_time_ms(&self) -> f64 {
        let completed = self.completed_count.load(Ordering::Relaxed);
        if completed == 0 {
            0.0
        } else {
            let total = self.total_execution_time_ms.load(Ordering::Relaxed);
            total as f64 / completed as f64
        }
    }

    #[inline]
    fn max_execution_time_ms(&self) -> u64 {
        self.max_execution_time_ms.load(Ordering::Relaxed)
    }

    #[inline]
    fn min_execution_time_ms(&self) -> u64 {
        let min = self.min_execution_time_ms.load(Ordering::Relaxed);
        if min == u64::MAX {
            0 // No tasks completed yet
        } else {
            min
        }
    }
}