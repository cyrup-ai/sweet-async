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
    task_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Clone for TokioRuntime {
    fn clone(&self) -> Self {
        Self {
            runtime: None, // Can't clone Runtime, so use None
            handle: self.handle.clone(),
            is_running: self.is_running.clone(),
            active_tasks: self.active_tasks.clone(),
            task_handles: self.task_handles.clone(),
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
            task_handles: Arc::new(Mutex::new(Vec::new())),
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
            task_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Get a handle to the runtime
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl<T: Clone + Send + 'static, I: TaskId> ApiRuntime<T, I> for TokioRuntime {
    type SpawnedTask = TokioAsyncTask<T, I>;

    fn spawn(
        &self,
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
    ) -> Self::SpawnedTask {
        let active_tasks = self.active_tasks.clone();
        let task_handles = self.task_handles.clone();
        
        // Create a new TokioAsyncTask
        let tokio_task = TokioAsyncTask::<T, I>::new_with_priority(
            task.task_id(),
            priority,
            self.handle.clone(),
            task_handles.clone(),
        );
        
        // Increment active task count
        active_tasks.fetch_add(1, Ordering::SeqCst);
        
        // Spawn the actual work
        let task_id = task.task_id();
        let task_result = tokio_task.result.clone();
        let task_status = tokio_task.status.clone();
        let active_tasks_clone = active_tasks.clone();
        
        let handle = self.handle.spawn(async move {
            // Update status to Running
            task_status.store(sweet_async_api::task::TaskStatus::Running as u8, Ordering::SeqCst);
            
            // Execute the task
            let result = task.await;
            
            // Store the result
            if let Ok(mut guard) = task_result.try_lock() {
                *guard = Some(result.clone());
            } else {
                // If we can't get the lock immediately, spawn a task to update it
                let task_result_clone = task_result.clone();
                tokio::spawn(async move {
                    let mut guard = task_result_clone.lock().await;
                    *guard = Some(result);
                });
            }
            
            // Update status based on result
            let status = if result.is_ok() {
                sweet_async_api::task::TaskStatus::Completed
            } else {
                sweet_async_api::task::TaskStatus::Cancelled
            };
            task_status.store(status as u8, Ordering::SeqCst);
            
            // Decrement active task count
            active_tasks_clone.fetch_sub(1, Ordering::SeqCst);
        });
        
        // Store the join handle
        let handles_clone = task_handles.clone();
        tokio::spawn(async move {
            let mut handles = handles_clone.lock().await;
            handles.push(handle);
        });
        
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
        
        // Get all task handles
        let handles = {
            let mut guard = self.task_handles.blocking_lock();
            std::mem::take(&mut *guard)
        };
        
        // Create a future that waits for all tasks
        let shutdown_future = async move {
            let timeout_fut = tokio::time::sleep(timeout);
            let join_all = async {
                for handle in handles {
                    let _ = handle.await;
                }
            };
            
            tokio::select! {
                _ = timeout_fut => {
                    Err(OrchestratorError::OperationFailed("Shutdown timeout reached".to_string()))
                }
                _ = join_all => {
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
