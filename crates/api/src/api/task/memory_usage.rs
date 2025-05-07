use std::time::Duration;

/// Memory metrics for task monitoring following OpenTelemetry conventions
pub trait MemoryUsage: Send + Sync + 'static {
    /// Current memory allocated by this task in bytes
    fn current_bytes(&self) -> u64;

    /// Peak memory usage by this task in bytes
    fn peak_bytes(&self) -> u64;

    /// Total number of allocations performed by this task
    fn allocation_count(&self) -> u64;

    /// Memory allocation rate (bytes per second)
    fn allocation_rate(&self) -> f64;
}
