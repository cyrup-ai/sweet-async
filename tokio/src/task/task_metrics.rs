use std::time::Duration;

use sweet_async_api::task::{CpuUsage, IoUsage, MemoryUsage};

/// Task metrics implementation for Tokio tasks
#[derive(Clone, Debug, Default)]
pub struct TaskMetrics {
    // CPU metrics
    cpu_time_ns: u64,
    cpu_utilization: f64,
    user_time_ns: u64,
    system_time_ns: u64,
    
    // Memory metrics
    current_bytes: u64,
    peak_bytes: u64,
    allocation_count: u64,
    allocation_rate: f64,
    
    // I/O metrics
    bytes_read: u64,
    bytes_written: u64,
    read_ops: u64,
    write_ops: u64,
    read_latency_ns: u64,
    write_latency_ns: u64,
    ops_per_second: f64,
    io_wait_time_ns: u64,
}

impl TaskMetrics {
    /// Create a new TaskMetrics instance with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    // CPU metric methods
    pub fn record_cpu_time(&mut self, duration: Duration) {
        self.cpu_time_ns = duration.as_nanos() as u64;
    }
    
    pub fn record_utilization(&mut self, utilization: f64) {
        self.cpu_utilization = utilization.clamp(0.0, 1.0);
    }
    
    // Memory metric methods
    pub fn record_memory_usage(&mut self, current: u64, peak: u64) {
        self.current_bytes = current;
        self.peak_bytes = peak.max(self.peak_bytes);
    }
    
    pub fn record_allocation(&mut self, size: u64) {
        self.allocation_count += 1;
        self.current_bytes += size;
        self.peak_bytes = self.peak_bytes.max(self.current_bytes);
    }
    
    // I/O metric methods
    pub fn record_read(&mut self, bytes: u64, latency: Duration) {
        self.bytes_read += bytes;
        self.read_ops += 1;
        self.read_latency_ns = latency.as_nanos() as u64;
    }
    
    pub fn record_write(&mut self, bytes: u64, latency: Duration) {
        self.bytes_written += bytes;
        self.write_ops += 1;
        self.write_latency_ns = latency.as_nanos() as u64;
    }
}

impl CpuUsage for TaskMetrics {
    fn cpu_time(&self) -> Duration {
        Duration::from_nanos(self.cpu_time_ns)
    }
    
    fn utilization(&self) -> f64 {
        self.cpu_utilization
    }
    
    fn user_time(&self) -> Duration {
        Duration::from_nanos(self.user_time_ns)
    }
    
    fn system_time(&self) -> Duration {
        Duration::from_nanos(self.system_time_ns)
    }
}

impl MemoryUsage for TaskMetrics {
    fn current_bytes(&self) -> u64 {
        self.current_bytes
    }
    
    fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }
    
    fn allocation_count(&self) -> u64 {
        self.allocation_count
    }
    
    fn allocation_rate(&self) -> f64 {
        self.allocation_rate
    }
}

impl IoUsage for TaskMetrics {
    fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
    
    fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
    
    fn read_operations(&self) -> u64 {
        self.read_ops
    }
    
    fn write_operations(&self) -> u64 {
        self.write_ops
    }
    
    fn read_latency(&self) -> Duration {
        Duration::from_nanos(self.read_latency_ns)
    }
    
    fn write_latency(&self) -> Duration {
        Duration::from_nanos(self.write_latency_ns)
    }
    
    fn operations_per_second(&self) -> f64 {
        self.ops_per_second
    }
    
    fn io_wait_time(&self) -> Duration {
        Duration::from_nanos(self.io_wait_time_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_task_metrics() {
        let mut metrics = TaskMetrics::new();
        
        // Test CPU metrics
        let cpu_time = Duration::from_millis(42);
        metrics.record_cpu_time(cpu_time);
        metrics.record_utilization(0.75);
        
        assert_eq!(metrics.cpu_time(), cpu_time);
        assert_eq!(metrics.utilization(), 0.75);
        
        // Test memory metrics
        metrics.record_memory_usage(1024, 2048);
        metrics.record_allocation(512);
        
        assert_eq!(metrics.current_bytes(), 1536);
        assert_eq!(metrics.peak_bytes(), 2048);
        
        // Test I/O metrics
        let read_latency = Duration::from_millis(5);
        let write_latency = Duration::from_millis(10);
        
        metrics.record_read(4096, read_latency);
        metrics.record_write(2048, write_latency);
        
        assert_eq!(metrics.bytes_read(), 4096);
        assert_eq!(metrics.bytes_written(), 2048);
        assert_eq!(metrics.read_latency(), read_latency);
        assert_eq!(metrics.write_latency(), write_latency);
    }
}
