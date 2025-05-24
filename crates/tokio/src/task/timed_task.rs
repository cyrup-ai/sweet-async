//! Timed task implementation for Tokio
//!
//! This module provides the implementation for tasks with timing capabilities,
//! including tracking creation, execution, and completion times.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

use sweet_async_api::task::TaskId;

use crate::task::async_task::AsyncTask;

/// Extension trait for AsyncTask to provide additional timing utilities
/// 
/// Note: These are now async-friendly versions that don't block the runtime
pub trait AsyncTaskTiming<T: Clone + Send + 'static, I: TaskId> {
    /// Get the total execution time of the task (async)
    fn execution_time_async(&self) -> Pin<Box<dyn Future<Output = Duration> + Send>>;
    
    /// Get the time since task creation
    fn time_since_creation(&self) -> Duration;
    
    /// Check if the task has timed out (async)
    fn has_timed_out_async(&self) -> Pin<Box<dyn Future<Output = bool> + Send>>;
    
    /// Get the remaining time before timeout (async)
    fn remaining_time_async(&self) -> Pin<Box<dyn Future<Output = Option<Duration>> + Send>>;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskTiming<T, I> for AsyncTask<T, I> {
    /// Get the total execution time of the task
    fn execution_time_async(&self) -> Pin<Box<dyn Future<Output = Duration> + Send>> {
        let start_clone = Arc::clone(&self.start_time);
        let end_clone = Arc::clone(&self.end_time);
        
        Box::pin(async move {
            let start = start_clone.lock().await;
            let end = end_clone.lock().await;
            
            match (*start, *end) {
                (Some(s), Some(e)) => {
                    e.duration_since(s).unwrap_or(Duration::from_secs(0))
                },
                (Some(s), None) => {
                    SystemTime::now().duration_since(s).unwrap_or(Duration::from_secs(0))
                },
                _ => Duration::from_secs(0),
            }
        })
    }
    
    /// Get the time since task creation
    /// This doesn't need async as created_time is immutable
    fn time_since_creation(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_time)
            .unwrap_or(Duration::from_secs(0))
    }
    
    /// Check if the task has timed out
    fn has_timed_out_async(&self) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        if self.timeout == Duration::from_secs(0) {
            // No timeout set
            return Box::pin(async { false });
        }
        
        let start_clone = Arc::clone(&self.start_time);
        let timeout = self.timeout;
        
        Box::pin(async move {
            let start = start_clone.lock().await;
            
            if let Some(s) = *start {
                let elapsed = SystemTime::now()
                    .duration_since(s)
                    .unwrap_or(Duration::from_secs(0));
                
                elapsed > timeout
            } else {
                false
            }
        })
    }
    
    /// Get the remaining time before timeout
    fn remaining_time_async(&self) -> Pin<Box<dyn Future<Output = Option<Duration>> + Send>> {
        if self.timeout == Duration::from_secs(0) {
            // No timeout set
            return Box::pin(async { None });
        }
        
        let start_clone = Arc::clone(&self.start_time);
        let timeout = self.timeout;
        
        Box::pin(async move {
            let start = start_clone.lock().await;
            
            if let Some(s) = *start {
                let elapsed = SystemTime::now()
                    .duration_since(s)
                    .unwrap_or(Duration::from_secs(0));
                
                if elapsed < timeout {
                    Some(timeout - elapsed)
                } else {
                    Some(Duration::from_secs(0))
                }
            } else {
                Some(timeout)
            }
        })
    }
}
