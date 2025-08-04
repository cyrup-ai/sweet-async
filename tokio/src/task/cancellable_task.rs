//! Cancellable task implementation for Tokio runtime
//!
//! This module provides comprehensive task cancellation support with graceful
//! shutdown, resource cleanup, and hierarchical cancellation propagation.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

use sweet_async_api::task::{CancellableTask, CancellationLevel, CancellationResult, AsyncTaskError};
use sweet_async_api::orchestra::OrchestratorError;

/// Tokio-specific cancellation result
#[derive(Debug, Clone)]
pub struct TokioCancellationResult {
    success: bool,
    timeout: bool,
    failure: bool,
    cancelled: bool,
    running: bool,
    level: CancellationLevel,
    error: Option<String>,
}

impl TokioCancellationResult {
    /// Create a successful cancellation result
    pub fn success(level: CancellationLevel) -> Self {
        Self {
            success: true,
            timeout: false,
            failure: false,
            cancelled: true,
            running: false,
            level,
            error: None,
        }
    }

    /// Create a timeout result
    pub fn timeout(level: CancellationLevel) -> Self {
        Self {
            success: false,
            timeout: true,
            failure: false,
            cancelled: false,
            running: true,
            level,
            error: Some("Cancellation timed out".to_string()),
        }
    }

    /// Create a failure result
    pub fn failure(level: CancellationLevel, error: String) -> Self {
        Self {
            success: false,
            timeout: false,
            failure: true,
            cancelled: false,
            running: false,
            level,
            error: Some(error),
        }
    }

    /// Create a still running result
    pub fn still_running(level: CancellationLevel) -> Self {
        Self {
            success: false,
            timeout: false,
            failure: false,
            cancelled: false,
            running: true,
            level,
            error: None,
        }
    }
}

impl CancellationResult for TokioCancellationResult {
    fn is_success(&self) -> bool {
        self.success
    }

    fn is_timeout(&self) -> bool {
        self.timeout
    }

    fn is_failure(&self) -> bool {
        self.failure
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn cancellation_level(&self) -> CancellationLevel {
        self.level
    }
}

/// Cancellation callback type
type CancellationCallback = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Tokio-specific cancellable task implementation
pub struct TokioCancellableTask<T: Send + 'static> {
    /// Cancellation flag - atomic for lock-free access
    cancelled: Arc<AtomicBool>,
    
    /// Current cancellation level
    cancellation_level: Arc<RwLock<Option<CancellationLevel>>>,
    
    /// Task handle for actual cancellation
    task_handle: Arc<RwLock<Option<JoinHandle<Result<T, AsyncTaskError>>>>>,
    
    /// Immutable cancellation callback property
    on_cancel_callback: Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>,
    
    /// Child tasks for hierarchical cancellation
    children: Arc<RwLock<Vec<Arc<TokioCancellableTask<T>>>>>,
}

impl<T: Send + 'static> TokioCancellableTask<T> {
    /// Create a new cancellable task
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            cancellation_level: Arc::new(RwLock::new(None)),
            task_handle: Arc::new(RwLock::new(None)),
            on_cancel_callback: None,
            children: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set the task handle
    pub async fn set_task_handle(&self, handle: JoinHandle<Result<T, AsyncTaskError>>) {
        let mut task_handle = self.task_handle.write().await;
        *task_handle = Some(handle);
    }

    /// Add a child task for hierarchical cancellation
    pub async fn add_child(&self, child: Arc<TokioCancellableTask<T>>) {
        let mut children = self.children.write().await;
        children.push(child);
    }

    /// Run cancellation callbacks
    async fn run_callbacks(&self) {
        // Execute the immutable callback property if present
        if let Some(ref callback) = self.on_cancel_callback {
            let future = callback();
            if let Err(e) = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                future
            ).await {
                warn!("Cancellation callback timed out: {:?}", e);
            }
        }
    }

    /// Cancel all child tasks
    async fn cancel_children(&self, level: CancellationLevel) {
        let children = self.children.read().await;
        let futures: Vec<_> = children
            .iter()
            .map(|child| child.cancel(level))
            .collect();
        
        // Wait for all children to be cancelled
        for future in futures {
            if let Err(e) = future.await {
                warn!("Failed to cancel child task: {:?}", e);
            }
        }
    }

    /// Perform cleanup based on cancellation level
    async fn perform_cleanup(&self, level: CancellationLevel) {
        match level {
            CancellationLevel::Graceful => {
                debug!("Performing graceful cleanup");
                self.run_callbacks().await;
            }
            CancellationLevel::Kill => {
                debug!("Performing essential cleanup");
                // Run critical callback with timeout
                if let Some(ref callback) = self.on_cancel_callback {
                    let future = callback();
                    if let Err(e) = tokio::time::timeout(
                        tokio::time::Duration::from_secs(1),
                        future
                    ).await {
                        warn!("Essential cleanup callback timed out: {:?}", e);
                    }
                }
            }
            CancellationLevel::KillHard => {
                debug!("Performing immediate termination (no cleanup)");
                // No cleanup for hard kill
            }
        }
    }
}

impl<T: Send + 'static> Default for TokioCancellableTask<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> CancellableTask<T> for TokioCancellableTask<T> {
    fn cancel(&self, level: CancellationLevel) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let cancelled = self.cancelled.clone();
        let cancellation_level = self.cancellation_level.clone();
        let task_handle = self.task_handle.clone();
        let child_tasks = self.children.clone();
        
        async move {
            // Set the cancellation flag atomically
            cancelled.store(true, Ordering::SeqCst);
            
            // Store the cancellation level
            {
                let mut level_guard = cancellation_level.write().await;
                *level_guard = Some(level);
            }
            
            tracing::debug!("Cancelling task with level: {:?}", level);
            
            // Cancel all child tasks first
            {
                let children = child_tasks.read().await;
                for child in children.iter() {
                    if let Err(e) = Box::pin(child.cancel_immediately()).await {
                        warn!("Failed to cancel child task: {:?}", e);
                    }
                }
            }
            
            // Cancel the main task handle if it exists
            {
                let handle_guard = task_handle.read().await;
                if let Some(handle) = handle_guard.as_ref() {
                    handle.abort();
                }
            }
            
            tracing::debug!("Task cancellation completed");
            Ok(())
        }
    }

    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::Graceful)
    }

    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::Kill)
    }

    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::KillHard)
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn on_cancel<F, Fut>(self, callback: F) -> Self
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Immutable builder pattern: return new instance with callback property
        let callback_wrapper = Box::new(move || {
            Box::pin(async move {
                let fut = callback.run().await;
                fut.await
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });
        
        Self {
            cancelled: self.cancelled,
            cancellation_level: self.cancellation_level,
            task_handle: self.task_handle,
            on_cancel_callback: Some(callback_wrapper),
            children: self.children,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_advanced_cancellation() {
        let task = TokioCancellableTask::<i32>::new();
        
        assert!(!task.is_cancelled());
        
        let result = task.cancel_gracefully().await;
        assert!(result.is_ok());
        assert!(task.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_levels() {
        let task = TokioCancellableTask::<i32>::new();
        
        // Test graceful cancellation
        let result = task.cancel(CancellationLevel::Graceful).await;
        assert!(result.is_ok());
        
        let level = task.cancellation_level.read().await;
        assert_eq!(*level, Some(CancellationLevel::Graceful));
    }

    #[tokio::test]
    async fn test_cancellation_callback() {
        let task = TokioCancellableTask::<i32>::new();
        let callback_executed = Arc::new(AtomicBool::new(false));
        let callback_executed_clone = callback_executed.clone();
        
        task.on_cancel(move || {
            let callback_executed = callback_executed_clone.clone();
            async move {
                callback_executed.store(true, Ordering::SeqCst);
            }
        });
        
        // Give the callback time to be registered
        sleep(Duration::from_millis(10)).await;
        
        let result = task.cancel_gracefully().await;
        assert!(result.is_ok());
        
        // Give the callback time to execute
        sleep(Duration::from_millis(100)).await;
        
        assert!(callback_executed.load(Ordering::SeqCst));
    }
}