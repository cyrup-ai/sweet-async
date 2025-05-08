use crate::api::task::{AsyncTaskError, MetricsEnabledTask};

/// Trait for tasks that support error tracing
///
/// This trait provides methods for handling and analyzing errors that occur
/// during task execution, enabling sophisticated error handling patterns.
///
/// # Error Handling Flow
///
/// The TracingTask trait implements a structured approach to error handling:
///
/// 1. **Detection**: When an error occurs during task execution
/// 2. **Recording**: Capture error details for logging and analysis
/// 3. **Analysis**: Determine if the error is retriable
/// 4. **Retry Decision**: Check if retry attempts remain
/// 5. **Handling**: Apply appropriate error handling logic
///
/// # Integration with Observability
///
/// TracingTask works closely with telemetry systems to provide insight into:
///
/// - Error frequencies and patterns
/// - Retry success rates
/// - Error correlations across task hierarchies
/// - Error trends over time
///
/// # Implementation Example
///
/// ```rust,no_run
/// impl<Id, T> TracingTask<Id, T> for MyTask<Id, T>
/// where
///     Id: TaskId,
///     T: Send + 'static,
/// {
///     fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
///         // Record the error first
///         self.record_error(&error);
///         
///         // Determine if we should retry
///         if self.should_retry(&error) && self.current_attempts() < self.max_retry_attempts() {
///             // If retriable and attempts remain, prepare for retry
///             self.increment_retry_counter();
///             return Err(error.mark_for_retry());
///         }
///         
///         // Otherwise, propagate the error
///         Err(error)
///     }
///
///     fn should_retry(&self, error: &AsyncTaskError) -> bool {
///         // Only retry transient errors, not permanent failures
///         matches!(error, 
///             AsyncTaskError::Timeout |
///             AsyncTaskError::Overloaded |
///             AsyncTaskError::TemporaryResourceFailure(_)
///         )
///     }
///     
///     fn max_retry_attempts(&self) -> u8 {
///         // Configure based on task criticality
///         match self.priority() {
///             TaskPriority::Critical => 5,
///             TaskPriority::High => 3,
///             _ => 1,
///         }
///     }
///     
///     fn record_error(&self, error: &AsyncTaskError) {
///         // Record detailed error metrics
///         let task_id = self.id().to_string();
///         log::error!("Task {task_id} failed with error: {error:?}");
///         metrics::increment_counter("task_errors", &[
///             ("task_type", self.name()),
///             ("error_type", error.error_type()),
///         ]);
///     }
/// }
/// ```
pub trait TracingTask<T: Send + 'static>: MetricsEnabledTask<T> {
    /// Handle an error that occurred during task execution
    ///
    /// This method is called when an error occurs during task execution.
    /// It allows for custom error handling logic to be applied.
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError>;
    
    /// Record an error for later analysis
    ///
    /// This method logs the error with appropriate context for
    /// later debugging and monitoring.
    fn record_error(&self, error: &AsyncTaskError);

    /// Check if tracing is enabled for this task
    fn is_tracing_enabled(&self) -> bool;
    
}