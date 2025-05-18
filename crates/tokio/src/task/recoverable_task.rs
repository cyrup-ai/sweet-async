//! Recoverable task implementation for Tokio
//!
//! This module provides the implementation for tasks that can recover from
//! failures using fallback values and policies.

use sweet_async_api::task::{AsyncTaskError, TaskId};

use crate::task::async_task::AsyncTask;

/// Extension trait for AsyncTask to provide additional recovery utilities
pub trait AsyncTaskRecovery<T: Clone + Send + 'static, I: TaskId> {
    /// Set a fallback value for recovery
    fn with_fallback(self, value: T) -> Self;
    
    /// Get the current retry count configuration
    fn retry_count(&self) -> u8;
    
    /// Get the current retry attempt
    fn current_retry(&self) -> u8;
    
    /// Set a retry policy for this task
    fn with_retry(self, count: u8) -> Self;
    
    /// Reset the retry counter
    fn reset_retry_counter(&self);
    
    /// Check if the task has exhausted all retry attempts
    fn has_exhausted_retries(&self) -> bool;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskRecovery<T, I> for AsyncTask<T, I> {
    /// Set a fallback value for recovery
    fn with_fallback(self, value: T) -> Self {
        futures::executor::block_on(async {
            let mut fallback = self.fallback.lock().await;
            *fallback = Some(value);
        });
        self
    }
    
    /// Get the current retry count configuration
    fn retry_count(&self) -> u8 {
        self.retry_count
    }
    
    /// Get the current retry attempt
    fn current_retry(&self) -> u8 {
        futures::executor::block_on(async {
            *self.current_retry.lock().await
        })
    }
    
    /// Set a retry policy for this task
    fn with_retry(mut self, count: u8) -> Self {
        self.retry_count = count;
        self
    }
    
    /// Reset the retry counter
    fn reset_retry_counter(&self) {
        futures::executor::block_on(async {
            let mut retry = self.current_retry.lock().await;
            *retry = 0;
        });
    }
    
    /// Check if the task has exhausted all retry attempts
    fn has_exhausted_retries(&self) -> bool {
        if self.retry_count == 0 {
            return true;
        }
        
        futures::executor::block_on(async {
            let current = *self.current_retry.lock().await;
            current >= self.retry_count
        })
    }
}
