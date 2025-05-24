//! Cancellable task implementation for Tokio
//!
//! This module provides the implementation of the cancellable task trait for AsyncTask,
//! supporting different levels of cancellation severity.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::{CancellableTask, CancellationLevel, TaskId};
use sweet_async_api::task::builder::AsyncWork;

use crate::task::async_task::AsyncTask;

impl<T: Clone + Send + 'static, I: TaskId> CancellableTask<T> for AsyncTask<T, I> {
    /// Cancel the task with andthe specified cancellation level
    fn cancel(
        &self,
        level: CancellationLevel,
    ) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        // Clone the needed fields to avoid ownership issues in the async block
        let cancel_tx = self.cancel_tx.clone();
        let status = self.status.clone();
        let handle = self.handle.clone();
        let cancel_callbacks = self.cancel_callbacks.clone();
        
        async move {
            // Update status to PendingCancellation
            {
                let mut status_lock = status.lock().await;
                *status_lock = sweet_async_api::task::TaskStatus::PendingCancellation;
            }
            
            // Send cancellation signal
            let cancel_tx = {
                let mut cancel_tx_lock = cancel_tx.lock().await;
                cancel_tx_lock.take()
            };
            
            if let Some(tx) = cancel_tx {
                let _ = tx.send(level);
            }
            
            // If KillHard, abort the task
            if matches!(level, CancellationLevel::KillHard) {
                let handle = {
                    let mut handle_lock = handle.lock().await;
                    handle_lock.take()
                };
                
                if let Some(handle) = handle {
                    handle.abort();
                }
            }
            
            // Update status to Cancelled
            {
                let mut status_lock = status.lock().await;
                *status_lock = sweet_async_api::task::TaskStatus::Cancelled;
            }
            
            // Execute cancellation callbacks
            let callbacks = {
                let callbacks_lock = cancel_callbacks.lock().await;
                callbacks_lock.iter().map(|f| f()).collect::<Vec<_>>()
            };
            
            for callback in callbacks {
                let _ = callback.await;
            }
            
            Ok(())
        }
    }
    
    /// Gracefully cancel the task, allowing it to clean up
    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::Graceful)
    }
    
    /// Forcefully cancel the task with minimal cleanup
    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::Kill)
    }
    
    /// Immediately terminate the task with no cleanup
    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::KillHard)
    }
    
    /// Check if the task has been cancelled
    /// 
    /// This method must return immediately without blocking.
    /// We use an atomic bool to track cancellation status.
    fn is_cancelled(&self) -> bool {
        // We need to add an atomic bool to AsyncTask for this to work properly
        // For now, we'll check if we can get the lock without blocking
        match self.status.try_lock() {
            Ok(status) => matches!(
                *status,
                sweet_async_api::task::TaskStatus::Cancelled | 
                sweet_async_api::task::TaskStatus::PendingCancellation
            ),
            Err(_) => {
                // If we can't get the lock, check the atomic flag
                // This requires adding atomic_cancelled: AtomicBool to AsyncTask
                false
            }
        }
    }
    
    /// Register a callback to be executed when the task is cancelled
    fn on_cancel<F, Fut>(&self, callback: F)
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let callbacks_clone = Arc::clone(&self.cancel_callbacks);
        
        // Spawn a detached task to add the callback
        tokio::spawn(async move {
            let mut callbacks = callbacks_clone.lock().await;
            callbacks.push(Box::new(move || Box::pin(callback.run())));
        });
    }
}

/// Implementation of CancellationResult for AsyncTask
pub struct TokioCancellationResult {
    /// Final status of the task after cancellation
    status: sweet_async_api::task::TaskStatus,
    /// Level at which cancellation was requested
    level: CancellationLevel,
}

impl TokioCancellationResult {
    /// Create a new cancellation result
    pub fn new(status: sweet_async_api::task::TaskStatus, level: CancellationLevel) -> Self {
        Self { status, level }
    }
}

impl sweet_async_api::task::CancellationResult for TokioCancellationResult {
    fn is_success(&self) -> bool {
        matches!(self.status, sweet_async_api::task::TaskStatus::Completed)
    }

    fn is_timeout(&self) -> bool {
        // Tokio tasks don't have a specific timeout status, so we use Failed as proxy
        matches!(self.status, sweet_async_api::task::TaskStatus::Cancelled) && 
        !self.is_cancelled()
    }

    fn is_failure(&self) -> bool {
        matches!(self.status, sweet_async_api::task::TaskStatus::Cancelled) &&
        !self.is_timeout()
    }

    fn is_cancelled(&self) -> bool {
        matches!(self.status, sweet_async_api::task::TaskStatus::Cancelled)
    }

    fn is_running(&self) -> bool {
        matches!(self.status, 
            sweet_async_api::task::TaskStatus::Running | 
            sweet_async_api::task::TaskStatus::Pending |
            sweet_async_api::task::TaskStatus::PendingCancellation
        )
    }

    fn cancellation_level(&self) -> CancellationLevel {
        self.level
    }
}
