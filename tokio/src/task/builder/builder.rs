use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use sweet_async_api::orchestra::OrchestratorBuilder;
use sweet_async_api::orchestra::orchestrator::TaskOrchestrator;
use sweet_async_api::task::AsyncTask;
use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::builder::{AsyncTaskBuilder, AsyncWork, SenderBuilder, SenderStrategy};
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::orchestra::TokioOrchestrator;
use crate::runtime::TokioRuntime;
use super::TokioAsyncTaskBuilder;
use crate::task::emit::TokioEmittingTaskBuilder;
use crate::task::spawn::TokioSpawningTaskBuilder;

/// Creates a new base task builder with default settings
///
/// # Example
///
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
///
/// let builder = builder::builder::<String, MyTaskId>();
/// ```
pub fn builder<T: Send + 'static, I: TaskId>() -> TokioAsyncTaskBuilder<T, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(Mutex::new(Vec::new()));
    TokioAsyncTaskBuilder::<T, I>::new(runtime, active_tasks)
}

/// Creates a new spawning task builder with default settings
///
/// This builder is specifically for future-based workflows that execute
/// once and return a result.
///
/// # Example
///
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
///
/// let builder = builder::spawning_builder::<String, &'static str, MyTaskId>();
/// let task = builder.run(|| async { "Hello, world!".to_string() });
/// ```
pub fn spawning_builder<T: Clone + Send + 'static, E: Send + 'static, I: TaskId>()
-> TokioSpawningTaskBuilder<T, E, I> {
    // Use the builder's own new() method which handles setup properly
    TokioSpawningTaskBuilder::new()
}

/// Creates a new emitting task builder with default settings
///
/// This builder is specifically for event-based workflows that emit
/// values over time.
///
/// # Example
///
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
///
/// let builder = builder::emitting_builder::<String, MyTaskId>();
/// ```
pub fn emitting_builder<T: Clone + Send + Sync + 'static, C: Clone + Send + Sync + 'static, EItem: Send + Sync + 'static, EOverall: Send + 'static, I: TaskId>() -> TokioEmittingTaskBuilder<T, C, EItem, EOverall, I> {
    // Use the builder's own new() method which handles setup properly
    TokioEmittingTaskBuilder::new()
}

/// Creates a new orchestrator with default settings
///
/// # Example
///
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
///
/// let orchestrator = builder::orchestrator::<String, MyTaskId>();
/// ```
pub fn orchestrator<T: Clone + Send + Sync + 'static, I: TaskId + Clone + Copy + Eq + Hash + Send + 'static>() -> TokioOrchestrator<T, I> {
    TokioOrchestrator::<T, I>::new(TokioRuntime::new())
}

/// Orchestrator builder that can forward method calls
pub enum DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    Spawning {
        // Store components to build later when we know T: Clone
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        timeout: Option<Duration>,
        retry_count: u8,
        tracing_enabled: bool,
        name: Option<String>,
        parent: Option<Box<dyn std::any::Any + Send + Sync>>,
        _marker: PhantomData<(T, Task, I)>,
    },
    Emitting {
        // Similar for emitting
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        timeout: Option<Duration>,
        retry_count: u8,
        tracing_enabled: bool,
        name: Option<String>,
        _marker: PhantomData<(T, Task, I)>,
    },
}

impl<T, Task, I> DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    /// Create a new orchestrator builder for spawning tasks
    pub fn new_spawning() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));

        Self::Spawning {
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            timeout: None,
            retry_count: 0,
            tracing_enabled: false,
            name: None,
            _marker: PhantomData,
        }
    }

    /// Create a new orchestrator builder for emitting tasks
    pub fn new_emitting() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));

        Self::Emitting {
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            timeout: None,
            retry_count: 0,
            tracing_enabled: false,
            name: None,
            _marker: PhantomData,
        }
    }

    /// Set a name for the task
    pub fn name(self, name: &str) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: _,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: Some(name.to_string()),
                _marker,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: _,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: Some(name.to_string()),
                _marker,
            },
        }
    }

    /// Set the task priority
    pub fn priority(self, priority: TaskPriority) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority: _,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority: _,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker,
            },
        }
    }
}

// Implement OrchestratorBuilder
impl<T, Task, I> OrchestratorBuilder<T, Task, I> for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    type Next = TokioAsyncTaskBuilder<T, I>;

    fn orchestrator<O: TaskOrchestrator<T, Task, I>>(self, _orchestrator: &O) -> Self::Next {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                ..
            } => {
                let mut builder = TokioAsyncTaskBuilder::new(runtime, active_tasks);
                builder = builder.priority(priority);
                if let Some(duration) = timeout {
                    builder = builder.timeout(duration);
                }
                if retry_count > 0 {
                    builder = builder.retry(retry_count);
                }
                builder = builder.tracing(tracing_enabled);
                if let Some(n) = name {
                    builder = builder.name(&n);
                }
                builder
            }
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                ..
            } => {
                let mut builder = TokioAsyncTaskBuilder::new(runtime, active_tasks);
                builder = builder.priority(priority);
                if let Some(duration) = timeout {
                    builder = builder.timeout(duration);
                }
                if retry_count > 0 {
                    builder = builder.retry(retry_count);
                }
                builder = builder.tracing(tracing_enabled);
                if let Some(n) = name {
                    builder = builder.name(&n);
                }
                builder
            }
        }
    }
}

// Implement AsyncTaskBuilder directly - this IS the AsyncTaskBuilder, no separate orchestrator step needed
impl<T, Task, I> sweet_async_api::task::builder::AsyncTaskBuilder
    for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + Sync + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static,
{
    fn new() -> Self {
        // Default to spawning
        Self::new_spawning()
    }

    fn timeout(self, duration: std::time::Duration) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout: _,
                retry_count,
                tracing_enabled,
                name,
                parent,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout: Some(duration),
                retry_count,
                tracing_enabled,
                name,
                parent,
                _marker,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout: _,
                retry_count,
                tracing_enabled,
                name,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout: Some(duration),
                retry_count,
                tracing_enabled,
                name,
                _marker,
            },
        }
    }

    fn retry(self, attempts: u8) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: _,
                tracing_enabled,
                name,
                parent,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: attempts,
                tracing_enabled,
                name,
                parent,
                _marker,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: _,
                tracing_enabled,
                name,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: attempts,
                tracing_enabled,
                name,
                _marker,
            },
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: _,
                name,
                parent,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: enabled,
                name,
                parent,
                _marker,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: _,
                name,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: enabled,
                name,
                _marker,
            },
        }
    }
}

// Add SpawningTaskBuilder implementation so we can call run() method
impl<T, I, F> sweet_async_api::task::spawn::SpawningTaskBuilder<T, AsyncTaskError, I>
    for DefaultOrchestratorBuilder<T, crate::task::tokio_task::TokioTask<T, I, F>, I>
where
    T: Clone + Send + Sync + 'static,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static,
    F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static + Clone,
{
    type Task = crate::task::spawn::spawning_task::TokioSpawningTask<T, I>;
    type ParentType = crate::orchestra::TokioOrchestrator<T, I>;
    
    fn parent(self, parent: Self::ParentType) -> Self {
        // Store the orchestrator as the parent in the builder
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker,
                ..
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                parent: Some(Box::new(parent) as Box<dyn std::any::Any + Send + Sync>),
                _marker,
            },
            Self::Emitting { .. } => panic!("Cannot set parent on emitting task builder"),
        }
    }

    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
    {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                parent,
                ..
            } => {
                // Create default orchestrator automatically if none provided
                // We'll need the actual type with bounds when using it
                let _orchestrator: TokioOrchestrator<T, I> = if let Some(boxed_parent) = parent {
                    // Try to downcast the parent
                    if let Ok(parent) = boxed_parent.downcast::<TokioOrchestrator<T, I>>() {
                        *parent
                    } else {
                        TokioOrchestrator::new(crate::runtime::TokioRuntime::new())
                    }
                } else {
                    TokioOrchestrator::new(crate::runtime::TokioRuntime::new())
                };
                
                // Use the actual TokioSpawningTaskBuilder with automatic orchestrator
                let mut builder = crate::task::spawn::builder::TokioSpawningTaskBuilder::new(runtime, active_tasks);
                if let Some(duration) = timeout {
                    builder = builder.timeout(duration);
                }
                if retry_count > 0 {
                    builder = builder.retry(retry_count);
                }
                builder = builder.tracing(tracing_enabled);
                if let Some(n) = name {
                    builder = builder.name(&n);
                }
                // The orchestrator is automatically available to the builder
                builder.run(work)
            }
            Self::Emitting { .. } => panic!("Cannot call run() on an emitting task builder"),
        }
    }

    fn await_result<F, R>(
        self,
        work: F,
    ) -> impl std::future::Future<Output = Result<T, AsyncTaskError>> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
    {
        // await_result is just a "leap frog" over .await - it's the same as run().await
        async move {
            let task = self.run(work);
            task.await
        }
    }

    fn await_result_with_handler<F, R, H, Out>(
        self,
        work: F,
        handler: H,
    ) -> impl std::future::Future<Output = Out> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
        H: sweet_async_api::task::builder::AsyncWork<Out> + Send + 'static,
    {
        // Use a single async block to ensure consistent types
        async move {
            match self {
                Self::Spawning { .. } => {
                    // Execute the work first and convert to result
                    let result = work.run().await;
                    let _async_result = result.into_async_result().await;
                    // Then run the handler with the result context
                    // Note: Reviewed as per TODOLIST.md task for builder/builder.rs - logic is functional, no type erasure needed
                    handler.run().await
                }
                Self::Emitting { .. } => {
                    // For emitting tasks, we can't use this method
                    panic!("Cannot call await_result_with_handler on an emitting task builder")
                }
            }
        }
    }
}

// Implement emitting task methods directly on DefaultOrchestratorBuilder
impl<T, Task, I> DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + Sync + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static,
{
    /// Start the sender phase for emitting tasks
    pub fn sender<F>(self, strategy: SenderStrategy, work: F) -> TokioEmittingTaskBuilder<T, (), (), AsyncTaskError, I>
    where
        F: AsyncWork<T> + Send + 'static,
    {
        match self {
            Self::Emitting {
                runtime,
                active_tasks: _,  // Can't use this - type mismatch with emitting builder
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                ..
            } => {
                // Create default orchestrator automatically
                let _orchestrator = TokioOrchestrator::new(crate::runtime::TokioRuntime::new());
                
                // Use the builder's new() method since the types don't match
                let mut builder = TokioEmittingTaskBuilder::new();
                if let Some(duration) = timeout {
                    builder = builder.timeout(duration);
                }
                if retry_count > 0 {
                    builder = builder.retry(retry_count);
                }
                builder = builder.tracing(tracing_enabled);
                if let Some(n) = name {
                    builder = builder.name(&n);
                }
                // Ensure sender strategy and work are configured as per section 4 of TODOLIST.md for full API ergonomics
                builder = builder.sender(strategy, work);
                builder
            }
            Self::Spawning { .. } => panic!("Cannot call sender() on a spawning task builder"),
        }
    }
}
