//! High-performance task status tracking

use std::sync::atomic::{AtomicU8, Ordering};
use sweet_async_api::task::task_status::{StatusEnabledTask, TaskStatus};

/// Zero-allocation task status tracker with atomic operations
#[derive(Debug)]
pub struct TokioStatusEnabledTask {
    status: AtomicU8,
}

impl TokioStatusEnabledTask {
    #[inline]
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(TaskStatus::Pending as u8),
        }
    }

    #[inline]
    pub fn set_status(&self, status: TaskStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

impl Default for TokioStatusEnabledTask {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> StatusEnabledTask<T> for TokioStatusEnabledTask {
    #[inline]
    fn status(&self) -> TaskStatus {
        TaskStatus::from_u8(self.status.load(Ordering::Relaxed))
    }
}

/// Type alias for TokioTaskStatus
pub type TokioTaskStatus = TokioStatusEnabledTask;
