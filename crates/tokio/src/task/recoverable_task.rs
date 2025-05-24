//! Recoverable task implementation for Tokio
//!
//! This module provides the implementation for tasks that can recover from
//! failures using fallback values and policies.

use std::future::Future;
use std::pin::Pin;
use sweet_async_api::task::{AsyncTaskError, TaskId};
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::task::async_task::AsyncTask;

/// Extension trait for AsyncTask to provide additional recovery utilities
/// 
/// Note: These methods now return futures that must be awaited, or use
/// internal atomic operations to avoid blocking.
pub trait AsyncTaskRecovery<T: Clone + Send + 'static, I: TaskId> {
    /// Set a fallback value for recovery (async version)
    fn with_fallback_async(self, value: T) -> Pin<Box<dyn Future<Output = Self> + Send>>;
    
    /// Set a fallback value for recovery (immediate version using Arc<Mutex>)
    fn with_fallback(self, value: T) -> Self;
    
    /// Get the current retry count configuration
    fn retry_count(&self) -> u8;
    
    /// Get the current retry attempt (async)
    fn current_retry_async(&self) -> Pin<Box<dyn Future<Output = u8> + Send>>;
    
    /// Set a retry policy for this task
    fn with_retry(self, count: u8) -> Self;
    
    /// Reset the retry counter (async)
    fn reset_retry_counter_async(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;
    
    /// Check if the task has exhausted all retry attempts (async)
    fn has_exhausted_retries_async(&self) -> Pin<Box<dyn Future<Output = bool> + Send>>;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskRecovery<T, I> for AsyncTask<T, I> {
    /// Set a fallback value for recovery (async version)
    fn with_fallback_async(self, value: T) -> Pin<Box<dyn Future<Output = Self> + Send>> {
        Box::pin(async move {
            let mut fallback = self.fallback.lock().await;
            *fallback = Some(value);
            self
        })
    }
    
    /// Set a fallback value for recovery (immediate version)
    /// This clones the Arc<Mutex> to avoid async in the builder pattern
    fn with_fallback(self, value: T) -> Self {
        // Clone the Arc to move into the spawned task
        let fallback_clone = Arc::clone(&self.fallback);
        
        // Spawn a detached task to set the fallback value
        tokio::spawn(async move {
            let mut fallback = fallback_clone.lock().await;
            *fallback = Some(value);
        });
        
        self
    }
    
    /// Get the current retry count configuration
    fn retry_count(&self) -> u8 {
        self.retry_count
    }
    
    /// Get the current retry attempt (async)
    fn current_retry_async(&self) -> Pin<Box<dyn Future<Output = u8> + Send>> {
        let retry_clone = Arc::clone(&self.current_retry);
        Box::pin(async move {
            *retry_clone.lock().await
        })
    }
    
    /// Set a retry policy for this task
    fn with_retry(mut self, count: u8) -> Self {
        self.retry_count = count;
        self
    }
    
    /// Reset the retry counter (async)
    fn reset_retry_counter_async(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let retry_clone = Arc::clone(&self.current_retry);
        Box::pin(async move {
            let mut retry = retry_clone.lock().await;
            *retry = 0;
        })
    }
    
    /// Check if the task has exhausted all retry attempts (async)
    fn has_exhausted_retries_async(&self) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let retry_count = self.retry_count;
        if retry_count == 0 {
            return Box::pin(async move { true });
        }
        
        let retry_clone = Arc::clone(&self.current_retry);
        Box::pin(async move {
            let current = *retry_clone.lock().await;
            current >= retry_count
        })
    }
}
