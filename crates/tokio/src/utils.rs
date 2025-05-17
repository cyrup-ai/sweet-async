use crate::runtime::TokioRuntime;
use crate::orchestrator::TokioOrchestrator;
use crate::task::adaptive::AdaptiveConfig;
use crate::task::tokio_task::TokioTask;

use sweet_async_api::task::{AsyncTask, TaskId, TaskPriority};
use sweet_async_api::task::spawn::SpawningTask;
use sweet_async_api::orchestra::Orchestra;

use std::future::Future;
use std::time::Duration;

/// Safely execute a blocking operation in any context (sync or async)
/// 
/// This function uses Tokio's block_in_place when in a Tokio context,
/// ensuring no deadlocks occur when blocking operations are performed.
/// 
/// # Example
/// 
/// ```rust
/// // Safe to call from both synchronous and asynchronous contexts
/// let count = safe_blocking(|| {
///     runtime.block_on(async {
///         tasks.lock().await.len()
///     })
/// });
/// ```
pub fn safe_blocking<F, R>(f: F) -> R 
where
    F: FnOnce() -> R,
{
    // Check if we're in a Tokio context
    if tokio::runtime::Handle::try_current().is_ok() {
        // We're in a Tokio context, use block_in_place
        tokio::task::block_in_place(f)
    } else {
        // Not in a Tokio context, call directly
        f()
    }
}

/// Create a simple TokioTask from a closure
pub fn spawn<F, T, E>(f: F) -> TokioTask<T, impl TaskId>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    let runtime = TokioRuntime::new();
    let spawner = TokioTaskSpawner::new(runtime);
    spawner.spawn(f)
}

/// Spawn a task with a timeout
pub fn spawn_with_timeout<F, T, E>(f: F, timeout: Duration) -> TokioTask<T, impl TaskId>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    let runtime = TokioRuntime::new();
    let spawner = TokioTaskSpawner::new(runtime);
    spawner.spawn_with_timeout(f, timeout)
}

/// Create an orchestrated task with the given runtime and orchestrator
pub fn orchestrate<T, I, F>(runtime: TokioRuntime, f: F) -> impl Future<Output = Result<T, sweet_async_api::task::AsyncTaskError>> + Send
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
    I: TaskId,
{
    let orchestrator = TokioOrchestrator::new(runtime.clone());
    let task = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(f),
        TaskPriority::Normal,
    );
    orchestrator.register_task(task.clone());
    
    task
}

/// Helper struct for spawning Tokio tasks
pub struct TokioTaskSpawner {
    runtime: TokioRuntime,
}

impl TokioTaskSpawner {
    /// Create a new spawner with the given runtime
    pub fn new(runtime: TokioRuntime) -> Self {
        Self { runtime }
    }
    
    /// Spawn a task
    pub fn spawn<F, T, E>(&self, f: F) -> TokioTask<T, impl TaskId>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: std::fmt::Debug + Send + 'static,
    {
        let task = move || {
            match f() {
                Ok(result) => result,
                Err(e) => panic!("Task error: {:?}", e),
            }
        };
        
        self.runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(task),
            TaskPriority::Normal,
        )
    }
    
    /// Spawn a task with a timeout
    pub fn spawn_with_timeout<F, T, E>(&self, f: F, timeout: Duration) -> TokioTask<T, impl TaskId>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: std::fmt::Debug + Send + 'static,
    {
        let task = self.spawn(f);
        task.with_timeout(timeout)
    }
}

/// Process a stream using adaptive concurrency
pub async fn process_adaptive<It, Item, F, Out>(
    items: It,
    map_fn: F,
    config: Option<AdaptiveConfig>,
) -> Vec<Out>
where
    It: IntoIterator<Item = Item> + Send + 'static,
    <It as IntoIterator>::IntoIter: Send,
    Item: Send + Sync + 'static,
    F: Fn(&Item) -> Out + Send + Sync + 'static,
    Out: Send + 'static,
{
    use futures::StreamExt;
    
    let cfg = config.unwrap_or_default();
    let stream = crate::task::adaptive_stream(items, map_fn, cfg);
    
    stream.collect().await
}
