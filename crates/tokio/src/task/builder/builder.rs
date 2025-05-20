//! Convenience builder functions for creating Tokio tasks
//!
//! This module provides simple, ergonomic functions for creating builders
//! without having to import the full builder types directly.

use std::sync::Arc;
use std::marker::PhantomData;
use std::time::Duration;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::AsyncTask;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::orchestra::OrchestratorBuilder;
use sweet_async_api::orchestra::orchestrator::TaskOrchestrator;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::task::builder::TokioAsyncTaskBuilder;
use crate::task::spawn::builder::TokioSpawningTaskBuilder;
use crate::task::emit::builder::TokioEmittingTaskBuilder;
use crate::orchestrator::TokioOrchestrator;
use crate::runtime::TokioRuntime;

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
/// let builder = builder::<String, MyTaskId>();
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
pub fn spawning_builder<T: Clone + Send + 'static, E: Send + 'static, I: TaskId>() -> TokioSpawningTaskBuilder<T, E, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(Mutex::new(Vec::new()));
    TokioSpawningTaskBuilder::<T, E, I>::new(runtime, active_tasks)
}

/// Orchestrator builder wrapper that can forward method calls
pub enum DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
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
        _marker: PhantomData<Task>,
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
        _marker: PhantomData<Task>,
    },
}

impl<T, Task, I> DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
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
}

// Implement OrchestratorBuilder
impl<T, Task, I> OrchestratorBuilder<T, Task, I> for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    type Next = TokioAsyncTaskBuilder<T, I>;
    
    fn orchestrator<O: TaskOrchestrator<T, Task, I>>(
        self,
        _orchestrator: &O,
    ) -> Self::Next {
        match self {
            Self::Spawning { runtime, active_tasks, priority, timeout, retry_count, tracing_enabled, name, .. } => {
                let mut builder = TokioAsyncTaskBuilder::new(runtime, active_tasks);
                if let Some(duration) = timeout {
                    builder = <TokioAsyncTaskBuilder<T, I> as AsyncTaskBuilder>::timeout(builder, duration);
                }
                if retry_count > 0 {
                    builder = <TokioAsyncTaskBuilder<T, I> as AsyncTaskBuilder>::retry(builder, retry_count);
                }
                builder = <TokioAsyncTaskBuilder<T, I> as AsyncTaskBuilder>::tracing(builder, tracing_enabled);
                if let Some(n) = name {
                    builder = builder.name(&n);
                }
                builder
            },
            Self::Emitting { runtime, active_tasks, priority, timeout, retry_count, tracing_enabled, name, .. } => {
                let mut builder = TokioAsyncTaskBuilder::new(runtime, active_tasks);
                if let Some(duration) = timeout {
                    builder = <TokioAsyncTaskBuilder<T, I> as AsyncTaskBuilder>::timeout(builder, duration);
                }
                if retry_count > 0 {
                    builder = <TokioAsyncTaskBuilder<T, I> as AsyncTaskBuilder>::retry(builder, retry_count);
                }
                builder = <TokioAsyncTaskBuilder<T, I> as AsyncTaskBuilder>::tracing(builder, tracing_enabled);
                if let Some(n) = name {
                    builder = builder.name(&n);
                }
                builder
            },
        }
    }
}

// Implement AsyncTaskBuilder to allow method chaining
impl<T, Task, I> sweet_async_api::task::builder::AsyncTaskBuilder for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    fn new() -> Self {
        // Default to spawning
        Self::new_spawning()
    }
    
    fn timeout(self, duration: std::time::Duration) -> Self {
        match self {
            Self::Spawning { runtime, active_tasks, priority, timeout: _, retry_count, tracing_enabled, name, _marker } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout: Some(duration),
                retry_count,
                tracing_enabled,
                name,
                _marker,
            },
            Self::Emitting { runtime, active_tasks, priority, timeout: _, retry_count, tracing_enabled, name, _marker } => Self::Emitting {
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
            Self::Spawning { runtime, active_tasks, priority, timeout, retry_count: _, tracing_enabled, name, _marker } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: attempts,
                tracing_enabled,
                name,
                _marker,
            },
            Self::Emitting { runtime, active_tasks, priority, timeout, retry_count: _, tracing_enabled, name, _marker } => Self::Emitting {
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
            Self::Spawning { runtime, active_tasks, priority, timeout, retry_count, tracing_enabled: _, name, _marker } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: enabled,
                name,
                _marker,
            },
            Self::Emitting { runtime, active_tasks, priority, timeout, retry_count, tracing_enabled: _, name, _marker } => Self::Emitting {
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
impl<T, I> sweet_async_api::task::spawn::SpawningTaskBuilder<T, AsyncTaskError, I> 
    for DefaultOrchestratorBuilder<T, crate::task::async_task::AsyncTask<T, I>, I>
where
    T: Clone + Send + 'static,
    I: TaskId,
{
    type Task = crate::task::async_task::AsyncTask<T, I>;
    
    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError> + Send + 'static,
    {
        match self {
            Self::Spawning { runtime, active_tasks, .. } => {
                // Create a new AsyncTask and configure it with the work
                use crate::task::async_task::AsyncTask;
                let task_id = I::from_string("default").unwrap(); // You may need to pass this in
                let task = AsyncTask::new(task_id, TaskPriority::Normal, runtime, active_tasks);
                // TODO: Configure the task with the work function
                task
            },
            Self::Emitting { .. } => panic!("Cannot call run() on an emitting task builder"),
        }
    }
    
    fn await_result<F, R>(self, work: F) -> impl std::future::Future<Output = Result<T, AsyncTaskError>> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError> + Send + 'static,
    {
        match self {
            Self::Spawning { runtime, active_tasks, .. } => {
                // Create a future that executes the work
                Box::pin(async move {
                    let result = work.run().await;
                    match result.into_async_result() {
                        Ok(value) => Ok(value),
                        Err(e) => Err(e),
                    }
                })
            },
            Self::Emitting { .. } => Box::pin(async { Err(AsyncTaskError::Failure("Cannot call await_result() on an emitting task builder".to_string())) }),
        }
    }
    
    fn await_result_with_handler<F, R, H, Out>(
        self,
        work: F,
        handler: H,
    ) -> impl std::future::Future<Output = Out> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError> + Send + 'static,
        H: sweet_async_api::task::builder::AsyncWork<Out> + Send + 'static,
        H::Future: std::future::Future<Output = Out>,
    {
        match self {
            Self::Spawning { runtime, active_tasks, .. } => {
                // Create a future that executes the work and applies the handler
                Box::pin(async move {
                    let result = work.run().await;
                    let async_result = result.into_async_result();
                    // Pass the result to the handler
                    handler.run().await
                })
            },
            Self::Emitting { .. } => {
                Box::pin(async move {
                    // Create a default out value - we need to know what Out is
                    // Since we can't know the specific type, we'll use the handler directly
                    handler.run().await
                })
            },
        }
    }
}