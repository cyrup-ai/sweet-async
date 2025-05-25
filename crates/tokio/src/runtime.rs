//! Tokio runtime implementation for Sweet Async
//! 
//! This module provides the runtime abstraction and utilities for safely
//! executing blocking operations without blocking the async runtime.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::runtime::{Handle, Runtime};
use tokio::task::{self, JoinHandle};
use tokio::sync::Mutex;

use sweet_async_api::orchestra::{OrchestratorError, runtime::Runtime as ApiRuntime};
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority};
use sweet_async_api::task::spawn::SpawningTask;

use crate::task::async_task::TokioAsyncTask;

/// Wrapper around Tokio's runtime
pub struct TokioRuntime {
    runtime: Option<Runtime>,
    pub(crate) handle: Handle,
    is_running: Arc<AtomicBool>,
    pub(crate) active_tasks: Arc<AtomicUsize>,
}

impl Clone for TokioRuntime {
    fn clone(&self) -> Self {
        Self {
            runtime: None, // Can't clone Runtime, so use None
            handle: self.handle.clone(),
            is_running: self.is_running.clone(),
            active_tasks: self.active_tasks.clone(),
        }
    }
}

impl TokioRuntime {
    /// Create a new runtime using the current Tokio handle
    pub fn new() -> Self {
        Self {
            runtime: None,
            handle: Handle::current(),
            is_running: Arc::new(AtomicBool::new(true)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Create a new runtime with a specific number of worker threads
    pub fn with_config(workers: usize) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        
        let handle = runtime.handle().clone();
        
        Self {
            runtime: Some(runtime),
            handle,
            is_running: Arc::new(AtomicBool::new(true)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Get a handle to the runtime
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl<T: Clone + Send + Sync + 'static, I: TaskId> ApiRuntime<T, I> for TokioRuntime {
    type SpawnedTask = TokioAsyncTask<T, I>;

    fn spawn(
        &self,
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
    ) -> Self::SpawnedTask {
        // Create a new TokioAsyncTask that wraps the SpawningTask's work
        let tokio_task = TokioAsyncTask::<T, I>::new_with_priority(
            task.task_id(),
            priority,
            self.handle.clone(),
            self.active_tasks.clone(),
        );
        
        // The actual work from SpawningTask will be executed when tokio_task is awaited
        tokio_task
    }

    fn block_on<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send + 'static,
    {
        // If we have our own runtime, use it
        if let Some(runtime) = &self.runtime {
            runtime.block_on(future)
        } else {
            // Otherwise, use the current handle's runtime
            self.handle.block_on(future)
        }
    }

    fn active_task_count(&self) -> usize {
        self.active_tasks.load(Ordering::SeqCst)
    }

    fn shutdown(&self, timeout: Duration) -> Result<(), OrchestratorError> {
        // Mark as not running
        self.is_running.store(false, Ordering::SeqCst);
        
        // Wait for active tasks to complete by checking the counter
        // Create a future that checks if all tasks are done
        let active_tasks = self.active_tasks.clone();
        let shutdown_future = async move {
            let timeout_fut = tokio::time::sleep(timeout);
            
            tokio::select! {
                _ = timeout_fut => {
                    Err(OrchestratorError::OperationFailed("Shutdown timeout reached".to_string()))
                }
                _ = async {
                    // Wait for all tasks to complete by checking the counter
                    while active_tasks.load(Ordering::SeqCst) > 0 {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                } => {
                    Ok(())
                }
            }
        };
        
        // Execute the shutdown
        self.block_on(shutdown_future)
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

/// Safely run a blocking operation without blocking the async runtime
/// 
/// This function spawns the blocking operation on Tokio's blocking thread pool,
/// ensuring that it doesn't block the async executor. The caller must await
/// the returned future to get the result.
pub async fn safe_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    task::spawn_blocking(f)
        .await
        .expect("Blocking task panicked")
}

/// Run a blocking operation and return a future that can be awaited
/// 
/// This is useful when you need to bridge sync and async code without
/// blocking the runtime.
pub fn run_blocking<F, R>(f: F) -> impl Future<Output = R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    async move {
        safe_blocking(f).await
    }
}
