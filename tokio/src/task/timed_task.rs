//! Timed task implementation with timeout and deadline support

use std::future::Future;

use std::time::{Duration, Instant, SystemTime};
use tokio::time::{sleep_until, timeout, Instant as TokioInstant};
use sweet_async_api::task::TimedTask;
use sweet_async_api::task::AsyncTaskError;

/// Tokio-specific timed task implementation
#[derive(Debug)]
pub struct TokioTimedTask {
    timeout_duration: Duration,
    deadline: Option<SystemTime>,
    start_time: Option<Instant>,
    end_time: Option<Instant>,
    /// Task creation timestamp
    created_timestamp: SystemTime,
    /// Task execution start timestamp
    executed_timestamp: Option<SystemTime>,
    /// Task completion timestamp
    completed_timestamp: Option<SystemTime>,
}

impl TokioTimedTask {
    /// Create a new timed task
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            timeout_duration: Duration::from_secs(30), // Default 30 second timeout
            deadline: None,
            start_time: None,
            end_time: None,
            created_timestamp: now,
            executed_timestamp: None,
            completed_timestamp: None,
        }
    }

    /// Set timeout duration
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    /// Set absolute deadline
    pub fn with_deadline(mut self, deadline: SystemTime) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Mark task as started
    pub fn mark_started(&mut self) {
        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        self.start_time = Some(now_instant);
        self.executed_timestamp = Some(now_system);
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self) {
        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        self.end_time = Some(now_instant);
        self.completed_timestamp = Some(now_system);
    }

    /// Get execution duration
    pub fn duration(&self) -> Option<Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            (Some(start), None) => Some(start.elapsed()),
            _ => None,
        }
    }

    /// Check if task has timed out
    pub fn is_timed_out(&self) -> bool {
        if let Some(start) = self.start_time {
            if start.elapsed() > self.timeout_duration {
                return true;
            }
        }
        
        if let Some(deadline) = self.deadline {
            return SystemTime::now() > deadline;
        }
        
        false
    }

    /// Execute a future with timeout
    pub async fn execute_with_timeout<F, T>(
        &mut self,
        future: F,
    ) -> Result<T, AsyncTaskError>
    where
        F: Future<Output = Result<T, AsyncTaskError>>,
    {
        self.mark_started();
        
        let result = if let Some(deadline) = self.deadline {
            let now = SystemTime::now();
            if now >= deadline {
                self.mark_completed();
                return Err(AsyncTaskError::Timeout(Duration::from_millis(0)));
            }
            
            let time_left = deadline.duration_since(now).unwrap_or(Duration::ZERO);
            let effective_timeout = self.timeout_duration.min(time_left);
            match timeout(effective_timeout, future).await {
                Ok(result) => result,
                Err(_) => {
                    self.mark_completed();
                    return Err(AsyncTaskError::Timeout(effective_timeout));
                }
            }
        } else {
            match timeout(self.timeout_duration, future).await {
                Ok(result) => result,
                Err(_) => {
                    self.mark_completed();
                    return Err(AsyncTaskError::Timeout(self.timeout_duration));
                }
            }
        };
        
        self.mark_completed();
        result
    }
}

impl Default for TokioTimedTask {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> TimedTask<T> for TokioTimedTask {
    fn created_timestamp(&self) -> SystemTime {
        self.created_timestamp
    }

    fn executed_timestamp(&self) -> SystemTime {
        self.executed_timestamp.unwrap_or(self.created_timestamp)
    }

    fn completed_timestamp(&self) -> SystemTime {
        self.completed_timestamp.unwrap_or_else(|| {
            if self.executed_timestamp.is_some() {
                SystemTime::now()
            } else {
                self.created_timestamp
            }
        })
    }

    fn timeout(&self) -> Duration {
        self.timeout_duration
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_timeout_execution() {
        let mut timed_task = TokioTimedTask::new().with_timeout(Duration::from_millis(100));
        
        let result = timed_task.execute_with_timeout(async {
            sleep_until(TokioInstant::now() + Duration::from_millis(200)).await;
            Ok::<i32, AsyncTaskError>(42)
        }).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().is_timeout());
    }

    #[tokio::test]
    async fn test_successful_execution() {
        let mut timed_task = TokioTimedTask::new().with_timeout(Duration::from_millis(200));
        
        let result = timed_task.execute_with_timeout(async {
            sleep_until(TokioInstant::now() + Duration::from_millis(50)).await;
            Ok::<i32, AsyncTaskError>(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(timed_task.duration().is_some());
    }
}