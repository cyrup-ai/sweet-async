use std::fmt::Debug;
use std::time::Duration;

use crate::api::task::CpuUsage;
use crate::api::task::IoUsage;
use crate::api::task::MemoryUsage;
use crate::api::task::TimedTask;

/// Collection of metrics related to task execution
///
/// This struct aggregates all the metrics collected during task execution,
/// providing a comprehensive view of the task's resource usage and performance.
/// It's designed to be lightweight and easy to consume by monitoring systems.
///
/// # Metric Categories
///
/// TaskMetrics includes several categories of performance data:
///
/// - **CPU Usage**: Processor time, utilization percentage, context switches
/// - **Memory Usage**: Allocation size, peak usage, garbage collection metrics
/// - **I/O Usage**: Read/write operations, bytes transferred, wait times
/// - **Duration**: Total execution time and sub-operation timings
///
/// # Observability Integration
///
/// The metrics can be exported to various observability systems:
///
/// - Logging systems (for human-readable reports)
/// - Time-series databases (for trend analysis)
/// - Distributed tracing systems (for execution context)
/// - Alerting systems (for threshold violations)
pub struct TaskMetrics<'a> {
    /// CPU usage metrics, if available
    pub cpu_usage: Option<&'a dyn CpuUsage>,
    
    /// Memory usage metrics, if available
    pub memory_usage: Option<&'a dyn MemoryUsage>,
    
    /// I/O usage metrics, if available
    pub io_usage: Option<&'a dyn IoUsage>,
    
    /// Execution duration of the task, if available
    pub duration: Option<Duration>,
}

/// A task that collects and reports execution metrics
///
/// This trait extends a timed task to include resource usage metrics, allowing
/// for comprehensive monitoring and performance analysis.
///
/// # Metrics Collection Design
///
/// The metrics system follows these principles:
///
/// 1. **Low Overhead**: Metrics collection should have minimal performance impact
/// 2. **Optional Components**: Each metric type can be independently enabled/disabled
/// 3. **Non-intrusive**: Metrics can be collected without modifying the task logic
/// 4. **Consistent Interface**: All metrics are accessed through a unified API
///
/// # Use Cases
///
/// Task metrics enable several important capabilities:
///
/// - **Performance Optimization**: Identify bottlenecks and optimization opportunities
/// - **Resource Planning**: Understand resource needs for capacity planning
/// - **SLA Monitoring**: Ensure tasks meet performance requirements
/// - **Anomaly Detection**: Identify unusual behavior patterns
/// - **Cost Attribution**: Associate resource usage with specific operations
///
/// # Implementation Example
///
/// ```rust,no_run
/// impl<Id, T> MetricsEnabledTask<Id, T> for MyTask<Id, T>
/// where
///     Id: TaskId,
///     T: Send + 'static,
/// {
///     fn metrics(&self) -> Option<TaskMetrics> {
///         // Check if metrics collection is enabled
///         if !self.metrics_enabled() {
///             return None;
///         }
///         
///         // Collect all available metrics
///         Some(TaskMetrics {
///             cpu_usage: self.cpu_profiler.as_ref().map(|p| p as &dyn CpuUsage),
///             memory_usage: self.memory_tracker.as_ref().map(|m| m as &dyn MemoryUsage),
///             io_usage: self.io_monitor.as_ref().map(|io| io as &dyn IoUsage),
///             duration: self.execution_duration(),
///         })
///     }
/// }
/// ```
pub trait MetricsEnabledTask<Id: Debug + Send + Sync + 'static, T: Send + 'static>: TimedTask<Id, T> {
    /// Get collected metrics for this task
    fn metrics(&self) -> Option<TaskMetrics>;
}
