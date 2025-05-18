//! Convenience builder functions for creating Tokio tasks
//!
//! This module provides simple, ergonomic functions for creating builders
//! without having to import the full builder types directly.

use std::sync::Arc;
use std::marker::PhantomData;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::AsyncTask;
use sweet_async_api::orchestra::OrchestratorBuilder;
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
        inner: TokioSpawningTaskBuilder<T, AsyncTaskError, I>,
        _marker: PhantomData<Task>,
    },
    Emitting {
        inner: TokioEmittingTaskBuilder<T, T, AsyncTaskError, I>,
        _marker: PhantomData<Task>,
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
        let inner = TokioSpawningTaskBuilder::<T, AsyncTaskError, I>::new(runtime, active_tasks);
        
        Self::Spawning {
            inner,
            _marker: PhantomData,
        }
    }
    
    /// Create a new orchestrator builder for emitting tasks
    pub fn new_emitting() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        let inner = TokioEmittingTaskBuilder::<T, T, AsyncTaskError, I>::new(runtime, active_tasks);
        
        Self::Emitting {
            inner,
            _marker: PhantomData,
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
    type Next = Self; // Return self to allow chaining
    
    fn orchestrator<O: sweet_async_api::orchestra::TaskOrchestrator<T, Task, I>>(
        self,
        _orchestrator: &O,
    ) -> Self::Next {
        // User is providing their own orchestrator
        // For now, we continue using our default implementation
        // In a full implementation, we would store the orchestrator reference
        self
    }
}

// Implement AsyncTaskBuilder to allow method chaining
impl<T, Task, I> sweet_async_api::task::builder::AsyncTaskBuilder for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
{
    fn new() -> Self {
        // Default to spawning
        Self::new_spawning()
    }
    
    fn timeout(self, duration: std::time::Duration) -> Self {
        match self {
            Self::Spawning { inner, _marker } => Self::Spawning {
                inner: inner.timeout(duration),
                _marker,
            },
            Self::Emitting { inner, _marker } => Self::Emitting {
                inner: inner.timeout(duration),
                _marker,
            },
        }
    }
    
    fn retry(self, attempts: u8) -> Self {
        match self {
            Self::Spawning { inner, _marker } => Self::Spawning {
                inner: inner.retry(attempts),
                _marker,
            },
            Self::Emitting { inner, _marker } => Self::Emitting {
                inner: inner.retry(attempts),
                _marker,
            },
        }
    }
    
    fn tracing(self, enabled: bool) -> Self {
        match self {
            Self::Spawning { inner, _marker } => Self::Spawning {
                inner: inner.tracing(enabled),
                _marker,
            },
            Self::Emitting { inner, _marker } => Self::Emitting {
                inner: inner.tracing(enabled),  
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
            Self::Spawning { inner, .. } => inner.run(work),
            Self::Emitting { .. } => panic!("Cannot call run() on an emitting task builder"),
        }
    }
    
    fn await_result<F, R>(self, work: F) -> impl std::future::Future<Output = Result<T, AsyncTaskError>> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError> + Send + 'static,
    {
        match self {
            Self::Spawning { inner, .. } => inner.await_result(work),
            Self::Emitting { .. } => panic!("Cannot call await_result() on an emitting task builder"),
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
    {
        match self {
            Self::Spawning { inner, .. } => inner.await_result_with_handler(work, handler),
            Self::Emitting { .. } => panic!("Cannot call await_result_with_handler() on an emitting task builder"),
        }
    }
}