//! Tokio OrchestratorBuilder implementation
//!
//! This module provides the TokioOrchestratorBuilder which implements the OrchestratorBuilder
//! trait from the API. It serves as the entry point for creating async tasks with orchestrators.

use std::marker::PhantomData;
use std::time::Duration;

use sweet_async_api::orchestra::{OrchestratorBuilder, TaskOrchestrator};
use sweet_async_api::task::builder::AsyncTaskBuilder;
use sweet_async_api::task::{AsyncTask, TaskId};

// TokioAsyncTaskBuilder is defined in task/builder - we define our own orchestrator-specific builder here

/// Tokio implementation of OrchestratorBuilder
///
/// This struct implements the initial builder pattern for async tasks, allowing users
/// to specify an orchestrator before configuring task-specific settings.
///
/// # Usage
/// ```rust
/// use sweet_async::TokioAsyncTask;
/// use sweet_async_api::task::AsyncTask;
///
/// let task = TokioAsyncTask::to::<String, _>()
///     .orchestrator(&orchestrator)
///     .timeout(Duration::from_secs(30))
///     .run(|| async { "Hello".to_string() })
///     .await?;
/// ```
pub struct TokioOrchestratorBuilder<
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: TaskId,
> {
    /// Builder type (task or emitting task)
    builder_type: BuilderType,
    
    /// Type markers
    _phantom: PhantomData<(T, Task, I)>,
}

/// Type of builder being created
#[derive(Debug, Clone, Copy)]
enum BuilderType {
    /// Regular task builder (from AsyncTask::to)
    Task,
    /// Emitting task builder (from AsyncTask::emits)
    EmittingTask,
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId>
    TokioOrchestratorBuilder<T, Task, I>
{
    /// Create a new orchestrator builder for regular tasks
    pub fn new_for_task() -> Self {
        Self {
            builder_type: BuilderType::Task,
            _phantom: PhantomData,
        }
    }
    
    /// Create a new orchestrator builder for emitting tasks
    pub fn new_for_emitting_task() -> Self {
        Self {
            builder_type: BuilderType::EmittingTask,
            _phantom: PhantomData,
        }
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId>
    OrchestratorBuilder<T, Task, I> for TokioOrchestratorBuilder<T, Task, I>
{
    type Next = TokioTaskBuilderWithOrchestrator<T, I>;
    
    fn orchestrator<O: TaskOrchestrator<T, Task, I>>(
        self,
        orchestrator: &O,
    ) -> Self::Next {
        // Create the appropriate task builder based on the builder type
        match self.builder_type {
            BuilderType::Task => {
                // Create a regular task builder
                TokioTaskBuilderWithOrchestrator::new_with_orchestrator(orchestrator)
            }
            BuilderType::EmittingTask => {
                // Create an emitting task builder
                TokioTaskBuilderWithOrchestrator::new_emitting_with_orchestrator(orchestrator)
            }
        }
    }
}

/// Tokio task builder with orchestrator configured
///
/// This struct provides the fluent builder interface for configuring async tasks
/// after an orchestrator has been specified.
pub struct TokioTaskBuilderWithOrchestrator<T: Clone + Send + 'static, I: TaskId> {
    /// Task timeout duration
    timeout_duration: Option<Duration>,
    
    /// Number of retry attempts
    retry_attempts: u8,
    
    /// Whether tracing is enabled
    tracing_enabled: bool,
    
    /// Builder type for determining execution path
    builder_type: BuilderType,
    
    /// Type markers
    _phantom: PhantomData<(T, I)>,
}

impl<T: Clone + Send + 'static, I: TaskId> TokioTaskBuilderWithOrchestrator<T, I> {
    /// Create a new task builder with orchestrator (regular task)
    pub fn new_with_orchestrator<Task: AsyncTask<T, I>, O: TaskOrchestrator<T, Task, I>>(
        _orchestrator: &O,
    ) -> Self {
        Self {
            timeout_duration: None,
            retry_attempts: 0,
            tracing_enabled: false,
            builder_type: BuilderType::Task,
            _phantom: PhantomData,
        }
    }
    
    /// Create a new emitting task builder with orchestrator
    pub fn new_emitting_with_orchestrator<Task: AsyncTask<T, I>, O: TaskOrchestrator<T, Task, I>>(
        _orchestrator: &O,
    ) -> Self {
        Self {
            timeout_duration: None,
            retry_attempts: 0,
            tracing_enabled: false,
            builder_type: BuilderType::EmittingTask,
            _phantom: PhantomData,
        }
    }
    
    /// Execute async work and return the result
    pub async fn run<F, R>(self, work: F) -> Result<R, sweet_async_api::task::AsyncTaskError>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send>> + Send + 'static,
        R: Send + 'static,
    {
        use sweet_async_api::task::builder::AsyncWork;
        
        // Create a wrapper that implements AsyncWork
        let work_wrapper = AsyncWorkWrapper { work };
        
        // Execute the work with configured settings
        let result_future = work_wrapper.run();
        
        // Apply timeout if configured
        match self.timeout_duration {
            Some(timeout) => {
                match tokio::time::timeout(timeout, result_future).await {
                    Ok(result) => Ok(result),
                    Err(_) => Err(sweet_async_api::task::AsyncTaskError::Timeout(timeout)),
                }
            }
            None => Ok(result_future.await),
        }
    }
}

/// Wrapper to implement AsyncWork for closures
struct AsyncWorkWrapper<F> {
    work: F,
}

impl<F, R> sweet_async_api::task::builder::AsyncWork<R> for AsyncWorkWrapper<F>
where
    F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send>> + Send + 'static,
    R: Send + 'static,
{
    fn run(self) -> impl std::future::Future<Output = R> + Send + 'static {
        async move {
            let future = (self.work)();
            future.await
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskBuilder for TokioTaskBuilderWithOrchestrator<T, I> {
    fn timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = Some(duration);
        self
    }
    
    fn retry(mut self, attempts: u8) -> Self {
        self.retry_attempts = attempts;
        self
    }
    
    fn tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }
    
    fn new() -> Self {
        Self {
            timeout_duration: None,
            retry_attempts: 0,
            tracing_enabled: false,
            builder_type: BuilderType::Task,
            _phantom: PhantomData,
        }
    }
}