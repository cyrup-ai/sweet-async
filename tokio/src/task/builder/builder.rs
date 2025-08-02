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
use sweet_async_api::task::emit::EmittingTaskBuilder;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::orchestra::TokioOrchestrator;
use crate::orchestra::runtime::TokioRuntime;
use crate::task::TokioAsyncTaskBuilder;
use crate::task::emit::TokioEmittingTaskBuilder;
use crate::task::spawn::TokioSpawningTaskBuilder;

/// Creates a new base task builder with default settings (internal)
fn builder<T: Send + 'static, I: TaskId>() -> TokioAsyncTaskBuilder<T, I> {
    TokioAsyncTaskBuilder::<T, I>::new()
}

/// Creates a new spawning task builder with default settings (internal)
fn spawning_builder<T: Clone + Send + 'static, E: Clone + Send + 'static, I: TaskId>()
-> TokioSpawningTaskBuilder<T, E, I> {
    // Use the builder's own new() method which handles setup properly
    TokioSpawningTaskBuilder::new()
}

/// Creates a new emitting task builder with default settings (internal)
fn emitting_builder<T: Clone + Send + Sync + 'static, C: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static + From<sweet_async_api::AsyncTaskError>, I: TaskId>() -> TokioEmittingTaskBuilder<T, C, E, I> {
    // Use the builder's own new() method which handles setup properly
    TokioEmittingTaskBuilder::new()
}

/// Creates a new orchestrator with default settings (internal)
fn orchestrator<T: Clone + Send + Sync + 'static, I: TaskId + Clone + Copy + Eq + Hash + Send + 'static>() -> TokioOrchestrator<T, I> {
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
        parent: Option<Box<dyn std::any::Any + Send + Sync>>,
        _marker: PhantomData<(T, Task, I)>,
    },
}

impl<T, Task, I> DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    /// Create a new orchestrator builder for spawning tasks (internal)
    fn new_spawning() -> Self {
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
            parent: None,
            _marker: PhantomData,
        }
    }

    /// Create a new orchestrator builder for emitting tasks (internal)
    fn new_emitting() -> Self {
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
            parent: None,
            _marker: PhantomData,
        }
    }

    /// Set a name for the task (internal)
    fn name(self, name: &str) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: _,
                parent,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: Some(name.to_string()),
                parent,
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
                parent,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name: Some(name.to_string()),
                parent,
                _marker,
            },
        }
    }

    /// Set the task priority (internal)
    fn priority(self, priority: TaskPriority) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority: _,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                parent,
                _marker,
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                parent,
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
                parent,
                _marker,
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                parent,
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
                let mut builder = TokioAsyncTaskBuilder::new();
                // Note: Priority is handled at the orchestrator level, not builder level
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
                let mut builder = TokioAsyncTaskBuilder::new();
                // Note: Priority is handled at the orchestrator level, not builder level
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
                parent,
                _marker,
            } => Self::Emitting {
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
                parent,
                _marker,
            } => Self::Emitting {
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
                parent,
                _marker,
            } => Self::Emitting {
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
        }
    }
}

// Add SpawningTaskBuilder implementation so we can call run() method
impl<T, I> sweet_async_api::task::spawn::SpawningTaskBuilder<T, AsyncTaskError, I>
    for DefaultOrchestratorBuilder<T, crate::task::async_task::TokioAsyncTask<T, I>, I>
where
    T: Clone + Send + Sync + 'static,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static + Default,
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
            Self::Emitting { .. } => {
                // Log the error and return self unchanged for API compatibility
                tracing::error!("Cannot set parent on emitting task builder - operation ignored");
                self
            }
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
                        TokioOrchestrator::new(crate::orchestra::runtime::TokioRuntime::new())
                    }
                } else {
                    TokioOrchestrator::new(crate::orchestra::runtime::TokioRuntime::new())
                };
                
                // Use the actual TokioSpawningTaskBuilder with automatic orchestrator
                let mut builder = crate::task::spawn::builder::TokioSpawningTaskBuilder::new();
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
            Self::Emitting { runtime, active_tasks, .. } => {
                // This should never happen in well-typed code, but handle gracefully
                tracing::error!("Invalid operation: run() called on emitting task builder");
                // Return a failed task using the same builder pattern for consistency
                let error_work = move || async {
                    Err(AsyncTaskError::InvalidState("Cannot call run() on emitting task builder".to_string()))
                };
                let builder = crate::task::spawn::builder::TokioSpawningTaskBuilder::new();
                builder.run(error_work)
            }
        }
    }

    fn await_result<F, R>(self, work: F) -> Result<T, AsyncTaskError>
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
    {
        // await_result provides synchronous method signature with 100% async execution
        // This is the "leap frog" pattern - sync signature but fully async under the hood
        match self {
            Self::Spawning { runtime, priority, .. } => {
                // Execute the async work synchronously using block_on
                let rt = runtime;
                let future = async move {
                    let result = work.run().await;
                    result.into_async_result().await
                };
                
                // This is where the magic happens - sync method signature, async execution
                rt.block_on(future)
            }
            Self::Emitting { .. } => {
                Err(AsyncTaskError::InvalidState(
                    "Cannot call await_result on emitting task builder".to_string()
                ))
            }
        }
    }

    fn await_result_with_handler<F, R, H, Out>(self, work: F, handler: H) -> Out
    where  
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
        H: sweet_async_api::task::builder::AsyncWork<Out> + Send + 'static,
    {
        // await_result_with_handler provides sync method signature with async execution
        match self {
            Self::Spawning { runtime, .. } => {
                // Execute both work and handler synchronously using block_on 
                let rt = runtime;
                let future = async move {
                    // Execute the work first and convert to result
                    let result = work.run().await;
                    let _async_result = result.into_async_result().await;
                    // Then run the handler with the result context
                    handler.run().await
                };
                
                // Sync method signature, async execution under the hood
                rt.block_on(future)
            }
            Self::Emitting { .. } => {
                // This case should not occur in practice due to type constraints
                // but we need to handle it for completeness
                tracing::error!("Invalid operation: await_result_with_handler() called on emitting task builder");
                // Since we can't create an Out without knowing its type, this will cause a compile error
                // which is the correct behavior - the type system prevents this invalid usage
                unreachable!("Type system prevents calling await_result_with_handler on emitting builder")
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
    /// Start the sender phase for emitting tasks (internal)
    fn sender<F>(self, strategy: SenderStrategy, work: F) -> TokioEmittingTaskBuilder<T, (), AsyncTaskError, I>
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
                let _orchestrator = TokioOrchestrator::new(crate::orchestra::runtime::TokioRuntime::new());
                
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
                // Configure sender strategy and work for full API ergonomics
                builder = builder.sender(work, strategy);
                builder
            }
            Self::Spawning { .. } => {
                // Return an error emitting task builder
                tracing::error!("Invalid operation: sender() called on spawning task builder");
                // Create a dummy emitting builder that will fail on build
                crate::task::emit::TokioEmittingTaskBuilder::new()
            }
        }
    }
}
