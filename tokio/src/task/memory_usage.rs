//! High-performance memory usage implementation with zero allocation

use sweet_async_api::task::MemoryUsage;

/// Zero-allocation memory usage implementation
#[derive(Debug, Clone, Copy)]
pub struct TokioMemoryUsage {
    current_bytes: u64,
    peak_bytes: u64,
    allocation_count: u64,
    allocation_rate: f64,
}

impl TokioMemoryUsage {
    #[inline]
    pub fn new() -> Self {
        Self {
            current_bytes: 0,
            peak_bytes: 0,
            allocation_count: 0,
            allocation_rate: 0.0,
        }
    }

    #[inline]
    pub fn with_values(
        current_bytes: u64,
        peak_bytes: u64,
        allocation_count: u64,
        allocation_rate: f64,
    ) -> Self {
        Self {
            current_bytes,
            peak_bytes,
            allocation_count,
            allocation_rate,
        }
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
        self.current_bytes
    }

    #[inline]
    fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }

    #[inline]
    fn allocation_count(&self) -> u64 {
        self.allocation_count
    }

    #[inline]
    fn allocation_rate(&self) -> f64 {
        self.allocation_rate
    }
}
