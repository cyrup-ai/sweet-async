//! Timed task implementation for Tokio
//!
//! This module provides the implementation for tasks with timing capabilities,
//! including tracking creation, execution, and completion times.

use std::time::{Duration, SystemTime};

use sweet_async_api::task::TaskId;

use crate::task::async_task::AsyncTask;

/// Extension trait for AsyncTask to provide additional timing utilities
pub trait AsyncTaskTiming<T: Clone + Send + 'static, I: TaskId> {
    /// Get the total execution time of the task
    fn execution_time(&self) -> Duration;
    
    /// Get the time since task creation
    fn time_since_creation(&self) -> Duration;
    
    /// Check if the task has timed out
    fn has_timed_out(&self) -> bool;
    
    /// Get the remaining time before timeout
    fn remaining_time(&self) -> Option<Duration>;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskTiming<T, I> for AsyncTask<T, I> {
    /// Get the total execution time of the task
    fn execution_time(&self) -> Duration {
        futures::executor::block_on(async {
            let start = self.start_time.lock().await;
            let end = self.end_time.lock().await;
            
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
    fn time_since_creation(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_time)
            .unwrap_or(Duration::from_secs(0))
    }
    
    /// Check if the task has timed out
    fn has_timed_out(&self) -> bool {
        if self.timeout == Duration::from_secs(0) {
            // No timeout set
            return false;
        }
        
        futures::executor::block_on(async {
            let start = self.start_time.lock().await;
            
            if let Some(s) = *start {
                let elapsed = SystemTime::now()
                    .duration_since(s)
                    .unwrap_or(Duration::from_secs(0));
                
                elapsed > self.timeout
            } else {
                false
            }
        })
    }
    
    /// Get the remaining time before timeout
    fn remaining_time(&self) -> Option<Duration> {
        if self.timeout == Duration::from_secs(0) {
            // No timeout set
            return None;
        }
        
        futures::executor::block_on(async {
            let start = self.start_time.lock().await;
            
            if let Some(s) = *start {
                let elapsed = SystemTime::now()
                    .duration_since(s)
                    .unwrap_or(Duration::from_secs(0));
                
                if elapsed < self.timeout {
                    Some(self.timeout - elapsed)
                } else {
                    Some(Duration::from_secs(0))
                }
            } else {
                Some(self.timeout)
            }
        })
    }
}
