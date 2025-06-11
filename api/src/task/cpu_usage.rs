use std::time::Duration;

/// CPU utilization metrics following OpenTelemetry conventions
pub trait CpuUsage: Send + Sync + 'static {
    /// CPU time used by this task in seconds
    fn cpu_time(&self) -> Duration;

    /// CPU utilization as a fraction (0.0 to 1.0 per core)
    fn utilization(&self) -> f64;

    /// CPU time in user mode
    fn user_time(&self) -> Duration;

    /// CPU time in system/kernel mode
    fn system_time(&self) -> Duration;
}
