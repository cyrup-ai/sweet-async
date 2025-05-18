//! Task result implementation for Tokio
//!
//! This module provides the result type for Tokio task execution.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::spawn::TaskResult;

/// Tokio implementation of TaskResult
pub struct AsyncTaskResult<T: Send + 'static, I: TaskId> {
    /// Task result
    result: Arc<Mutex<Option<Result<T, AsyncTaskError>>>>,
    /// Task ID
    id: I,
}

impl<T: Send + 'static, I: TaskId> AsyncTaskResult<T, I> {
    /// Create a new task result
    pub fn new(id: I) -> Self {
        Self {
            result: Arc::new(Mutex::new(None)),
            id,
        }
    }
    
    /// Set the task result
    pub fn set_result(&self, result: Result<T, AsyncTaskError>) {
        let mut res = self.result.lock().unwrap();
        *res = Some(result);
    }
}

impl<T: Send + 'static, I: TaskId> Future for AsyncTaskResult<T, I> {
    type Output = Result<T, AsyncTaskError>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        let res = this.result.lock().unwrap();
        
        if let Some(result) = res.clone() {
            std::task::Poll::Ready(result)
        } else {
            // We're still waiting for the result
            // In a real implementation, we would register the waker
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }
}

impl<T: Send + 'static, I: TaskId> TaskResult<T, I> for AsyncTaskResult<T, I> {
    fn task_id(&self) -> I {
        self.id
    }
    
    fn result(&self) -> Option<Result<T, AsyncTaskError>> {
        self.result.lock().unwrap().clone()
    }
}
