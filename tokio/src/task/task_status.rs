//! Task status tracking for the Tokio implementation

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use sweet_async_api::task::{StatusEnabledTask, TaskStatus};

/// Atomic task status representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokioTaskStatus {
    /// Task is pending execution
    Pending = 0,
    /// Task is currently running
    Running = 1,
    /// Task completed successfully
    Completed = 2,
    /// Task failed with an error
    Failed = 3,
    /// Task was cancelled
    Cancelled = 4,
    /// Task timed out
    TimedOut = 5,
}

impl From<u8> for TokioTaskStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => TokioTaskStatus::Pending,
            1 => TokioTaskStatus::Running,
            2 => TokioTaskStatus::Completed,
            3 => TokioTaskStatus::Failed,
            4 => TokioTaskStatus::Cancelled,
            5 => TokioTaskStatus::TimedOut,
            _ => TokioTaskStatus::Failed,
        }
    }
}

impl From<TokioTaskStatus> for u8 {
    fn from(status: TokioTaskStatus) -> Self {
        status as u8
    }
}

impl TaskStatus for TokioTaskStatus {
    fn is_pending(&self) -> bool {
        matches!(self, TokioTaskStatus::Pending)
    }
    
    fn is_running(&self) -> bool {
        matches!(self, TokioTaskStatus::Running)
    }
    
    fn is_completed(&self) -> bool {
        matches!(self, TokioTaskStatus::Completed)
    }
    
    fn is_failed(&self) -> bool {
        matches!(self, TokioTaskStatus::Failed)
    }
    
    fn is_cancelled(&self) -> bool {
        matches!(self, TokioTaskStatus::Cancelled)
    }
}

/// Thread-safe task status tracker
#[derive(Debug)]
pub struct TokioTaskStatusTracker {
    status: Arc<AtomicU8>,
    start_time: Option<u64>,
    end_time: Option<u64>,
}

impl TokioTaskStatusTracker {
    /// Create a new status tracker
    pub fn new() -> Self {
        Self {
            status: Arc::new(AtomicU8::new(TokioTaskStatus::Pending as u8)),
            start_time: None,
            end_time: None,
        }
    }

    /// Get the current status
    pub fn status(&self) -> TokioTaskStatus {
        TokioTaskStatus::from(self.status.load(Ordering::SeqCst))
    }

    /// Set the status atomically
    pub fn set_status(&mut self, status: TokioTaskStatus) {
        self.status.store(status as u8, Ordering::SeqCst);
        
        match status {
            TokioTaskStatus::Running if self.start_time.is_none() => {
                self.start_time = Some(current_timestamp());
            }
            TokioTaskStatus::Completed 
            | TokioTaskStatus::Failed 
            | TokioTaskStatus::Cancelled 
            | TokioTaskStatus::TimedOut if self.end_time.is_none() => {
                self.end_time = Some(current_timestamp());
            }
            _ => {}
        }
    }

    /// Get execution duration in milliseconds
    pub fn duration_ms(&self) -> Option<u64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end.saturating_sub(start)),
            (Some(start), None) if self.status().is_running() => {
                Some(current_timestamp().saturating_sub(start))
            }
            _ => None,
        }
    }
}

impl Default for TokioTaskStatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> StatusEnabledTask<T> for TokioTaskStatusTracker {
    fn status(&self) -> impl TaskStatus {
        self.status()
    }

    fn set_status(&mut self, status: impl TaskStatus) {
        let tokio_status = if status.is_pending() {
            TokioTaskStatus::Pending
        } else if status.is_running() {
            TokioTaskStatus::Running
        } else if status.is_completed() {
            TokioTaskStatus::Completed
        } else if status.is_failed() {
            TokioTaskStatus::Failed
        } else if status.is_cancelled() {
            TokioTaskStatus::Cancelled
        } else {
            TokioTaskStatus::Failed
        };
        
        self.set_status(tokio_status);
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}