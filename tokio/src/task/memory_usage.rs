//! Memory usage tracking for tasks

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use sweet_async_api::task::MemoryUsage;

/// Memory usage tracker for tasks
#[derive(Debug)]
pub struct TokioMemoryUsageTracker {
    current_memory_bytes: Arc<AtomicU64>,
    peak_memory_bytes: Arc<AtomicU64>,
    allocations: Arc<AtomicU64>,
    deallocations: Arc<AtomicU64>,
}

impl TokioMemoryUsageTracker {
    /// Create a new memory usage tracker
    pub fn new() -> Self {
        Self {
            current_memory_bytes: Arc::new(AtomicU64::new(0)),
            peak_memory_bytes: Arc::new(AtomicU64::new(0)),
            allocations: Arc::new(AtomicU64::new(0)),
            deallocations: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record memory allocation
    pub fn record_allocation(&self, bytes: u64) {
        let new_current = self.current_memory_bytes.fetch_add(bytes, Ordering::SeqCst) + bytes;
        self.allocations.fetch_add(1, Ordering::SeqCst);
        
        // Update peak if necessary
        let current_peak = self.peak_memory_bytes.load(Ordering::SeqCst);
        if new_current > current_peak {
            self.peak_memory_bytes.compare_exchange_weak(
                current_peak, 
                new_current, 
                Ordering::SeqCst, 
                Ordering::SeqCst
            ).ok(); // Ignore failure - another thread may have updated it
        }
    }

    /// Record memory deallocation
    pub fn record_deallocation(&self, bytes: u64) {
        self.current_memory_bytes.fetch_sub(bytes, Ordering::SeqCst);
        self.deallocations.fetch_add(1, Ordering::SeqCst);
    }

    /// Get net allocations
    pub fn net_allocations(&self) -> i64 {
        let allocs = self.allocations.load(Ordering::SeqCst) as i64;
        let deallocs = self.deallocations.load(Ordering::SeqCst) as i64;
        allocs - deallocs
    }

    /// Reset memory tracking
    pub fn reset(&self) {
        self.current_memory_bytes.store(0, Ordering::SeqCst);
        self.peak_memory_bytes.store(0, Ordering::SeqCst);
        self.allocations.store(0, Ordering::SeqCst);
        self.deallocations.store(0, Ordering::SeqCst);
    }
}

impl Default for TokioMemoryUsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryUsage for TokioMemoryUsageTracker {
    fn current_bytes(&self) -> u64 {
        self.current_memory_bytes.load(Ordering::SeqCst)
    }

    fn peak_bytes(&self) -> u64 {
        self.peak_memory_bytes.load(Ordering::SeqCst)
    }

    fn allocation_count(&self) -> u64 {
        self.allocations.load(Ordering::SeqCst)
    }

    fn allocation_rate(&self) -> f64 {
        // For now, return zero allocation rate
        // In a real implementation, this would track allocations over time
        0.0
    }
}