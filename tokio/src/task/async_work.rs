//! Implementation of async work execution for Tokio tasks

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::{AsyncTaskError, TaskId};
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use super::builder::TokioAsyncTaskBuilder;

/// Execute async work using Tokio runtime
pub struct TokioAsyncWork<T, I, W>
where
    T: Send + 'static,
    I: TaskId,
    W: AsyncWork<Result<T, AsyncTaskError>> + Send + 'static,
{
    /// The async work to execute
    work: W,
    /// Builder configuration for the task
    builder: TokioAsyncTaskBuilder<T, I>,
}

impl<T, I, W> TokioAsyncWork<T, I, W>
where
    T: Send + 'static,
    I: TaskId,
    W: AsyncWork<Result<T, AsyncTaskError>> + Send + 'static,
{
    /// Create a new async work instance with the given work and builder
    pub fn new(work: W, builder: TokioAsyncTaskBuilder<T, I>) -> Self {
        Self { work, builder }
    }

    /// Get the configured timeout
    pub fn timeout(&self) -> Duration {
        self.builder.get_timeout()
    }

    /// Get the runtime handle
    pub fn runtime(&self) -> &Handle {
        self.builder.runtime()
    }

    /// Get the active tasks registry
    pub fn active_tasks(&self) -> &Arc<Mutex<Vec<JoinHandle<()>>>> {
        self.builder.active_tasks()
    }
}

impl<T, I, W> AsyncWork<Result<T, AsyncTaskError>> for TokioAsyncWork<T, I, W>
where
    T: Send + 'static,
    I: TaskId,
    W: AsyncWork<Result<T, AsyncTaskError>> + Send + 'static,
{
    fn run(self) -> Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>> {
        let timeout_duration = self.timeout();
        let runtime_handle = self.runtime().clone();
        let work_future = self.work.run();
        Box::pin(async move {
            // Spawn the work on the Tokio runtime
            let handle = runtime_handle.spawn(async move {
                work_future.await
            });
            // Store the handle in active tasks if tracing or management is needed
            if self.builder.is_tracing_enabled() {
                let mut active_tasks = self.active_tasks().lock().await;
                active_tasks.push(handle.clone().into());
            }
            // Apply timeout to the task execution
            match timeout(timeout_duration, handle).await {
                Ok(result) => match result {
                    Ok(output) => output,
                    Err(e) => Err(AsyncTaskError::RuntimeError(e.to_string())),
                },
                Err(_) => Err(AsyncTaskError::Timeout(
                    format!("Task timed out after {:?}", timeout_duration),
                )),
            }
        })
    }
}
