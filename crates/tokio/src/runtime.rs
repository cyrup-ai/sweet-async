use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::task::{AsyncTaskError, TaskId, TaskPriority};
use sweet_async_api::task::spawn::SpawningTask;

use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::task::tokio_task::TokioTask;

/// Tokio-based implementation of the Runtime trait
pub struct TokioRuntime {
    /// The Tokio runtime handle
    handle: Handle,
    /// Track active tasks
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Flag to indicate runtime status
    running: Arc<Mutex<bool>>,
    /// Configuration
    config: TokioRuntimeConfig,
}

/// Configuration for the Tokio runtime
#[derive(Debug, Clone)]
pub struct TokioRuntimeConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    /// Default task priority
    pub default_priority: TaskPriority,
}

impl Default for TokioRuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            default_priority: TaskPriority::Normal,
        }
    }
}

impl TokioRuntime {
    /// Create a new TokioRuntime with default configuration
    pub fn new() -> Self {
        Self {
            handle: Handle::current(),
            active_tasks: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(true)),
            config: TokioRuntimeConfig::default(),
        }
    }

    /// Create a new TokioRuntime with custom worker count
    pub fn with_config(worker_threads: usize) -> Self {
        let mut config = TokioRuntimeConfig::default();
        config.worker_threads = worker_threads;
        
        Self {
            handle: Handle::current(),
            active_tasks: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(true)),
            config,
        }
    }
    
    /// Get a reference to the Tokio runtime handle
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl<T: Send + 'static, I: TaskId> Runtime<T, I> for TokioRuntime {
    type SpawnedTask = TokioTask<T, I>;

    fn spawn(
        &self,
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
    ) -> Self::SpawnedTask {
        // Create a new TokioTask with the given task and priority
        TokioTask::new(task, priority, self.handle.clone(), self.active_tasks.clone())
    }

    fn block_on<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send + 'static,
    {
        // Execute the future on the Tokio runtime
        self.handle.block_on(future)
    }

    fn active_task_count(&self) -> usize {
        // Use Tokio's block_in_place to safely execute blocking code from an async context
        tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                self.active_tasks.lock().await.len()
            })
        })
    }

    fn shutdown(&self, timeout: Duration) -> Result<(), OrchestratorError> {
        tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                // Set running flag to false
                *self.running.lock().await = false;
                
                // Create a future that waits for all tasks to complete
                let tasks_future = async {
                    let mut tasks = self.active_tasks.lock().await;
                    for task in tasks.drain(..) {
                        let _ = task.await;
                    }
                };
                
                // Wait for tasks to complete with timeout
                match tokio::time::timeout(timeout, tasks_future).await {
                    Ok(_) => Ok(()),
                    Err(_) => Err(OrchestratorError::OperationFailed(
                        "Timeout waiting for tasks to complete".to_string(),
                    )),
                }
            })
        })
    }

    fn is_running(&self) -> bool {
        futures::executor::block_on(async {
            *self.running.lock().await
        })
    }
}
