//! Tracing task implementation for structured logging

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tracing::{debug, error, info, warn, Instrument, Span};
use sweet_async_api::task::{TracingTask, AsyncTaskError};

/// Tokio-specific tracing task implementation
#[derive(Debug)]
pub struct TokioTracingTask {
    span: Option<Span>,
    fields: HashMap<String, String>,
    task_name: String,
    /// Whether tracing is enabled for this task
    tracing_enabled: AtomicBool,
    /// Count of errors recorded by this task
    error_count: AtomicU64,
}

impl TokioTracingTask {
    /// Create a new tracing task
    pub fn new(task_name: impl Into<String>) -> Self {
        let task_name = task_name.into();
        let span = tracing::info_span!("task", name = %task_name);
        
        Self {
            span: Some(span),
            fields: HashMap::new(),
            task_name,
            tracing_enabled: AtomicBool::new(true),
            error_count: AtomicU64::new(0),
        }
    }

    /// Add a field to the trace context
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Get the current span
    pub fn span(&self) -> Option<&Span> {
        self.span.as_ref()
    }

    /// Execute a future with tracing instrumentation
    pub async fn trace_execution<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        if let Some(span) = &self.span {
            future.instrument(span.clone()).await
        } else {
            future.await
        }
    }

    /// Log task start
    pub fn log_start(&self) {
        info!(
            task_name = %self.task_name,
            "Task started"
        );
    }

    /// Log task completion
    pub fn log_completion(&self, duration_ms: u64) {
        info!(
            task_name = %self.task_name,
            duration_ms = duration_ms,
            "Task completed successfully"
        );
    }

    /// Log task failure
    pub fn log_failure(&self, error: &str, duration_ms: Option<u64>) {
        error!(
            task_name = %self.task_name,
            error = %error,
            duration_ms = ?duration_ms,
            "Task failed"
        );
    }

    /// Log task cancellation
    pub fn log_cancellation(&self, reason: &str) {
        warn!(
            task_name = %self.task_name,
            reason = %reason,
            "Task cancelled"
        );
    }

    /// Log debug information
    pub fn log_debug(&self, message: &str) {
        debug!(
            task_name = %self.task_name,
            message = %message,
        );
    }

    /// Log progress update
    pub fn log_progress(&self, completed: u64, total: u64, message: &str) {
        info!(
            task_name = %self.task_name,
            completed = completed,
            total = total,
            progress_percent = (completed as f64 / total as f64 * 100.0) as u32,
            message = %message,
            "Task progress"
        );
    }
}

impl Default for TokioTracingTask {
    fn default() -> Self {
        Self::new("unnamed_task")
    }
}

impl<T: Send + 'static> TracingTask<T> for TokioTracingTask {
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Record the error for tracing and monitoring
        self.record_error(&error);
        
        // Increment error count atomically (zero allocation)
        self.error_count.fetch_add(1, Ordering::Relaxed);
        
        // Log structured error information based on error type
        match &error {
            AsyncTaskError::Timeout(duration) => {
                error!(
                    task_name = %self.task_name,
                    error_type = "timeout",
                    timeout_duration = ?duration,
                    error_count = self.error_count.load(Ordering::Relaxed),
                    "Task timeout error"
                );
            }
            AsyncTaskError::Cancelled => {
                warn!(
                    task_name = %self.task_name,
                    error_type = "cancelled",
                    error_count = self.error_count.load(Ordering::Relaxed),
                    "Task cancellation"
                );
            }
            AsyncTaskError::Failure(msg) => {
                error!(
                    task_name = %self.task_name,
                    error_type = "failure",
                    message = %msg,
                    error_count = self.error_count.load(Ordering::Relaxed),
                    "Task execution failure"
                );
            }
            AsyncTaskError::Panic(msg) => {
                error!(
                    task_name = %self.task_name,
                    error_type = "panic",
                    message = %msg,
                    error_count = self.error_count.load(Ordering::Relaxed),
                    "Task panic occurred"
                );
            }
            AsyncTaskError::InvalidState(msg) => {
                error!(
                    task_name = %self.task_name,
                    error_type = "invalid_state",
                    message = %msg,
                    error_count = self.error_count.load(Ordering::Relaxed),
                    "Task invalid state error"
                );
            }
            _ => {
                error!(
                    task_name = %self.task_name,
                    error_type = "other",
                    error = ?error,
                    error_count = self.error_count.load(Ordering::Relaxed),
                    "Task error occurred"
                );
            }
        }
        
        // Return the error - no recovery attempt
        Err(error)
    }

    fn record_error(&self, error: &AsyncTaskError) {
        if !self.is_tracing_enabled() {
            return;
        }

        // High-performance structured error recording
        let current_count = self.error_count.load(Ordering::Relaxed);
        
        match error {
            AsyncTaskError::Timeout(duration) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "timeout",
                    duration_ms = duration.as_millis() as u64,
                    total_errors = current_count,
                    "Error recorded: timeout"
                );
            }
            AsyncTaskError::Cancelled => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "cancelled",
                    total_errors = current_count,
                    "Error recorded: cancellation"
                );
            }
            AsyncTaskError::Failure(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "failure",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: execution failure"
                );
            }
            AsyncTaskError::Panic(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "panic",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: panic"
                );
            }
            AsyncTaskError::Rejected(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "rejected",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: rejection"
                );
            }
            AsyncTaskError::ResourceLimit(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "resource_limit",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: resource limit"
                );
            }
            AsyncTaskError::InvalidState(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "invalid_state",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: invalid state"
                );
            }
            AsyncTaskError::InvalidData => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "invalid_data",
                    total_errors = current_count,  
                    "Error recorded: invalid data"
                );
            }
            AsyncTaskError::KeyVersionTooOld(version) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "key_version_too_old",
                    required_version = *version,
                    total_errors = current_count,
                    "Error recorded: key version too old"
                );
            }
            AsyncTaskError::Io(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "io",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: I/O error"
                );
            }
            AsyncTaskError::Unknown(msg) => {
                debug!(
                    task_name = %self.task_name,
                    error_type = "unknown",
                    message = %msg,
                    total_errors = current_count,
                    "Error recorded: unknown error"
                );
            }
        }
    }

    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }

    fn trace_start(&self) {
        self.log_start();
    }

    fn trace_completion(&self, duration_ms: u64) {
        self.log_completion(duration_ms);
    }

    fn trace_error(&self, error: &str, duration_ms: Option<u64>) {
        self.log_failure(error, duration_ms);
    }

    fn trace_event(&self, event: &str, metadata: Option<&str>) {
        info!(
            task_name = %self.task_name,
            event = %event,
            metadata = ?metadata,
            "Task event"
        );
    }

    fn add_trace_field(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }

    fn get_trace_fields(&self) -> std::collections::HashMap<String, String> {
        self.fields.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tracing_task_creation() {
        let tracing_task = TokioTracingTask::new("test_task")
            .with_field("user_id", "123")
            .with_field("operation", "test");
        
        assert_eq!(tracing_task.task_name, "test_task");
        assert_eq!(tracing_task.fields.len(), 2);
        assert!(tracing_task.span.is_some());
    }

    #[tokio::test]
    async fn test_trace_execution() {
        let tracing_task = TokioTracingTask::new("async_test");
        
        let result = tracing_task.trace_execution(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            42
        }).await;
        
        assert_eq!(result, 42);
    }
}