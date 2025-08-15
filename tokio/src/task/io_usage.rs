//! High-performance IO usage implementation with zero allocation

use std::time::Duration;
use sweet_async_api::task::IoUsage;

/// Zero-allocation IO usage implementation
#[derive(Debug, Clone, Copy)]
pub struct TokioIoUsage {
    bytes_read: u64,
    bytes_written: u64,
    read_operations: u64,
    write_operations: u64,
    read_latency_nanos: u64,
    write_latency_nanos: u64,
    operations_per_second: f64,
    io_wait_time_nanos: u64,
}

impl TokioIoUsage {
    #[inline]
    pub fn new() -> Self {
        Self {
            bytes_read: 0,
            bytes_written: 0,
            read_operations: 0,
            write_operations: 0,
            read_latency_nanos: 0,
            write_latency_nanos: 0,
            operations_per_second: 0.0,
            io_wait_time_nanos: 0,
        }
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
        self.bytes_read
    }

    #[inline]
    fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    #[inline]
    fn read_operations(&self) -> u64 {
        self.read_operations
    }

    #[inline]
    fn write_operations(&self) -> u64 {
        self.write_operations
    }

    #[inline]
    fn read_latency(&self) -> Duration {
        Duration::from_nanos(self.read_latency_nanos)
    }

    #[inline]
    fn write_latency(&self) -> Duration {
        Duration::from_nanos(self.write_latency_nanos)
    }

    #[inline]
    fn operations_per_second(&self) -> f64 {
        self.operations_per_second
    }

    #[inline]
    fn io_wait_time(&self) -> Duration {
        Duration::from_nanos(self.io_wait_time_nanos)
    }
}
