use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::SpawningTaskBuilder;
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
use sweet_async_api::orchestra::{OrchestratorBuilder, orchestrator::TaskOrchestrator};

use crate::task::TokioAsyncTaskBuilder;
use crate::task::relationships::TokioTaskRelationships;
use crate::task::spawn::spawning_task::TokioSpawningTask;

/// Builder for creating and configuring spawning tasks
///
/// A SpawningTaskBuilder is used to create future-based tasks that
/// execute once and return a result.
#[derive(Clone)]
pub struct TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// The base builder with common configuration
    base: TokioAsyncTaskBuilder<T, I>,
    /// Task priority
    priority: TaskPriority,
    /// Parent task relationships for hierarchical task management
    parent_relationships: Option<TokioTaskRelationships<T, I>>,
    /// Phantom data for error type parameter
    _phantom_e: PhantomData<E>,
}

impl<T, E, I> TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Create a new spawning task builder with default settings
    pub fn new() -> Self {
        Self {
            base: TokioAsyncTaskBuilder::new(),
            priority: TaskPriority::Normal,
            parent_relationships: None,
            _phantom_e: PhantomData,
        }
    }

    /// Create a new spawning task builder with specific runtime and active tasks
    fn with_runtime(
        runtime: tokio::runtime::Handle,
        active_tasks: Arc<tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    ) -> Self {
        Self::new_internal(runtime, active_tasks)
    }

    /// Internal constructor that does the actual initialization
    fn new_internal(
        runtime: tokio::runtime::Handle,
        active_tasks: Arc<tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    ) -> Self {
        Self {
            base: TokioAsyncTaskBuilder::new_with_runtime(runtime, active_tasks),
            priority: TaskPriority::Normal,
            parent_relationships: None,
            _phantom_e: PhantomData,
        }
    }

    /// Set the task priority (internal method)
    fn priority(self, priority: TaskPriority) -> Self {
        Self { priority, ..self }
    }

    /// Set a descriptive name for the task (internal method)
    fn name(self, name: &str) -> Self {
        Self {
            base: self.base.name(name),
            ..self
        }
    }
}

impl<T, E, I> AsyncTaskBuilder for TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    fn new() -> Self {
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        Self::new_internal(runtime, active_tasks)
    }

    // Note: name is not part of the API trait, implement on struct directly

    fn timeout(self, duration: std::time::Duration) -> Self {
        Self {
            base: self.base.timeout(duration),
            ..self
        }
    }

    fn retry(self, attempts: u8) -> Self {
        Self {
            base: self.base.retry(attempts),
            ..self
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        Self {
            base: self.base.tracing(enabled),
            ..self
        }
    }
}

impl<T, E, I> SpawningTaskBuilder<T, E, I> for TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + Sync + 'static,
    E: std::fmt::Display + Send + 'static,
    I: TaskId,
    E: Into<AsyncTaskError> + From<AsyncTaskError>,
{
    type Task = TokioSpawningTask<T, I>;
    type ParentType = TokioTaskRelationships<T, I>;

    fn parent(self, parent: Self::ParentType) -> Self {
        Self {
            parent_relationships: Some(parent),
            ..self
        }
    }

    /// Create a task with the given work function
    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        // Generate a unique task ID
        let timestamp_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_else(|_| {
                // Fallback to current process nanos if system time is invalid
                std::time::Instant::now().elapsed().as_nanos()
            });

        let random_id = format!("task-{}", timestamp_nanos);
        let id = I::from_string(&random_id).unwrap_or_else(|| {
            // Create a fallback ID string using a timestamp with a different prefix
            let timestamp_millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_else(|_| {
                    // Fallback to current process millis if system time is invalid
                    std::time::Instant::now().elapsed().as_millis()
                });

            let fallback_id = format!("fallback-task-{}", timestamp_millis);
            I::from_string(&fallback_id).unwrap_or_else(|| {
                // Final fallback: use a simple incrementing counter
                static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let simple_id = format!("simple-task-{}", counter);
                I::from_string(&simple_id).unwrap_or_else(|| {
                    // If all else fails, try to create the simplest possible ID
                    tracing::error!("Failed to create any task ID, using default");
                    I::default()
                })
            })
        });

        // Create a spawning task with the provided work and parent relationships
        TokioSpawningTask::from_work_with_relationships(id, work, self.parent_relationships)
    }

    /// Create and immediately await a task
    fn await_result<F, R>(self, work: F) -> Result<T, E>
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        let task = self.run(work);

        match self.base.runtime.block_on(task) {
            Ok(value) => Ok(value),
            Err(err) => Err(E::from(err)),
        }
    }

    /// Create, await, and process with a handler
    fn await_result_with_handler<F, R, H, Out>(self, work: F, handler: H) -> Out
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
        H: AsyncWork<Out> + Send + 'static,
    {
        let _result = self.await_result(work);

        self.base.runtime.block_on(handler.run())
    }
}

// Polymorphic implementation: Allow using custom orchestrator
impl<T, E, I> OrchestratorBuilder<T, TokioSpawningTask<T, I>, I> for TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    type Next = crate::orchestra::TokioTaskBuilderWithOrchestrator<T, I>;
    
    fn orchestrator<O: TaskOrchestrator<T, TokioSpawningTask<T, I>, I>>(
        self,
        orchestrator: &O,
    ) -> Self::Next {
        use crate::orchestra::TokioTaskBuilderWithOrchestrator;
        TokioTaskBuilderWithOrchestrator::new_with_orchestrator(orchestrator)
    }
}
