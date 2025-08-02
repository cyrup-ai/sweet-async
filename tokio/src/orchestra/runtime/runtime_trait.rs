//! Tokio implementation of the Runtime trait from sweet_async_api
//!
//! This module implements the Runtime trait exactly as defined in the API
//! using Tokio's async runtime capabilities.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::runtime::Handle;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::task::{AsyncTaskError, TaskId, TaskPriority};
use sweet_async_api::task::spawn::SpawningTask;

use crate::task::spawn::spawning_task::TokioSpawningTask;

/// Tokio implementation of the Runtime trait
#[derive(Debug)]
pub struct TokioRuntime {
    handle: Handle,
    is_running: AtomicBool,
    active_tasks: Arc<AtomicUsize>,
    _runtime: Option<Arc<tokio::runtime::Runtime>>, // Keep custom runtime alive, zero allocation when None
}

impl TokioRuntime {
    /// Create a new runtime using the current Tokio handle
    pub fn new() -> Self {
        Self {
            handle: Handle::current(),
            is_running: AtomicBool::new(true),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            _runtime: None, // No custom runtime, zero allocation
        }
    }

    /// Create a runtime with custom tokio::runtime::Runtime for sophisticated configuration
    pub fn with_custom_runtime(handle: Handle, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            handle,
            is_running: AtomicBool::new(true),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            _runtime: Some(runtime), // Keep custom runtime alive
        }
    }
}

impl Clone for TokioRuntime {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            is_running: AtomicBool::new(self.is_running.load(Ordering::SeqCst)),
            active_tasks: self.active_tasks.clone(),
            _runtime: self._runtime.clone(), // Arc clone is zero allocation
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> Runtime<T, I> for TokioRuntime {
    type SpawnedTask = TokioSpawningTask<T, I>;

    fn spawn(
        &self,
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
    ) -> Self::SpawnedTask {
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
        
        TokioSpawningTask::new_with_spawning_task(
            task,
            priority,
            self.handle.clone(),
            self.active_tasks.clone(),
        )
    }

    fn block_on<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send + 'static,
    {
        self.handle.block_on(future)
    }

    fn active_task_count(&self) -> usize {
        self.active_tasks.load(Ordering::Relaxed)
    }

    fn shutdown(&self, timeout: Duration) -> Result<(), OrchestratorError> {
        self.is_running.store(false, Ordering::SeqCst);
        
        // Wait for active tasks to complete within timeout
        let start = std::time::Instant::now();
        while self.active_task_count() > 0 && start.elapsed() < timeout {
            std::thread::sleep(Duration::from_millis(10));
        }
        
        if self.active_task_count() > 0 {
            Err(OrchestratorError::OperationFailed(
                format!("Shutdown timeout: {} tasks still active", self.active_task_count())
            ))
        } else {
            Ok(())
        }
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}