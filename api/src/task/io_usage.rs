use std::time::Duration;

/// IO metrics for task monitoring following OpenTelemetry conventions
pub trait IoUsage: Send + Sync + 'static {
    /// Total number of bytes read
    fn bytes_read(&self) -> u64;

    /// Total number of bytes written
    fn bytes_written(&self) -> u64;

    /// Number of read operations performed
    fn read_operations(&self) -> u64;

    /// Number of write operations performed
    fn write_operations(&self) -> u64;

    /// Average latency for read operations
    fn read_latency(&self) -> Duration;

    /// Average latency for write operations
    fn write_latency(&self) -> Duration;

    /// Current IO operations per second
    fn operations_per_second(&self) -> f64;

    /// IO wait time (time spent waiting for IO operations)
    fn io_wait_time(&self) -> Duration;
}
