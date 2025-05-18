//! Tracing task implementation for Tokio
//!
//! This module provides implementations for tracing and monitoring task execution,
//! including error handling, logging, and telemetry.

use sweet_async_api::task::{AsyncTaskError, TaskId, TracingTask};

use crate::task::async_task::AsyncTask;
use crate::task::recoverable_task::AsyncTaskRecovery;

impl<T: Clone + Send + 'static, I: TaskId> TracingTask<T> for AsyncTask<T, I> {
    /// Handle an error that occurred during task execution
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Record the error first
        self.record_error(&error);
        
        // If tracing is enabled, log the error
        if self.tracing_enabled {
            tracing::error!("Task error: {:?}", error);
        }
        
        // Check if we can recover from this error
        if self.can_recover_from(&error) {
            return self.recover(error);
        }
        
        // Otherwise, return the error
        Err(error)
    }
    
    /// Record an error for later analysis
    fn record_error(&self, error: &AsyncTaskError) {
        // In a real implementation, this would interface with telemetry systems
        if self.tracing_enabled {
            tracing::error!("Recording error: {:?}", error);
            
            // Get task information for context
            let task_id = self.task_id().to_string();
            let task_name = self.name().unwrap_or_else(|| "unnamed".to_string());
            
            // Log with structured context
            tracing::error!(
                task_id = %task_id,
                task_name = %task_name,
                error_type = %error_type_name(error),
                error_message = %error.to_string(),
                "Task execution error"
            );
        }
    }
    
    /// Check if tracing is enabled for this task
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled
    }
}

/// Extension trait for AsyncTask to provide additional tracing utilities
pub trait AsyncTaskTracing<T: Clone + Send + 'static, I: TaskId> {
    /// Enable or disable tracing for this task
    fn with_tracing(self, enabled: bool) -> Self;
    
    /// Check if this error type is retriable
    fn is_retriable_error(&self, error: &AsyncTaskError) -> bool;
    
    /// Calculate backoff time for retries
    fn calculate_backoff_time(&self, retry_number: u8) -> std::time::Duration;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskTracing<T, I> for AsyncTask<T, I> {
    /// Enable or disable tracing for this task
    fn with_tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }
    
    /// Check if this error type is retriable
    fn is_retriable_error(&self, error: &AsyncTaskError) -> bool {
        match error {
            // Typically retriable errors
            AsyncTaskError::Timeout(_) => true,
            AsyncTaskError::Overloaded => true, 
            AsyncTaskError::ResourceExhausted => true,
            
            // Non-retriable errors
            AsyncTaskError::Cancelled => false,
            AsyncTaskError::InvalidState(_) => false,
            AsyncTaskError::InvalidTaskId => false,
            AsyncTaskError::TaskPanicked => false,
            
            // Case-by-case errors (could implement more sophisticated logic)
            AsyncTaskError::Failure(_) => false,
        }
    }
    
    /// Calculate backoff time for retries
    fn calculate_backoff_time(&self, retry_number: u8) -> std::time::Duration {
        // Implement exponential backoff with jitter
        let base = 500; // Base milliseconds
        let max_backoff = 60_000; // Max 60 seconds
        
        // Calculate exponential backoff
        let backoff = base * (2_u64.pow(retry_number as u32));
        
        // Add jitter to prevent synchronized retries
        let jitter = rand::random::<u64>() % 100;
        let backoff_with_jitter = backoff + jitter;
        
        // Cap at maximum backoff
        let capped_backoff = std::cmp::min(backoff_with_jitter, max_backoff);
        
        std::time::Duration::from_millis(capped_backoff)
    }
}

/// Helper function to get the error type name as a string
fn error_type_name(error: &AsyncTaskError) -> &'static str {
    match error {
        AsyncTaskError::Timeout(_) => "timeout",
        AsyncTaskError::Cancelled => "cancelled",
        AsyncTaskError::InvalidState(_) => "invalid_state",
        AsyncTaskError::Failure(_) => "failure",
        AsyncTaskError::Overloaded => "overloaded",
        AsyncTaskError::ResourceExhausted => "resource_exhausted",
        AsyncTaskError::InvalidTaskId => "invalid_task_id",
        AsyncTaskError::TaskPanicked => "task_panicked",
    }
}
