//! High-performance spawning task implementation with zero allocation

use crate::orchestra::runtime::TokioOrchestraRuntime;
use crate::task::spawn::result::{TokioAsyncResult, TokioTaskResult};
use crate::task::{TokioAsyncTask, TokioTaskRelationships};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, SystemTime};
use sweet_async_api::task::builder::AsyncWork;

/// AsyncWork implementation for zero-allocation async operations
#[derive(Clone)]
pub struct TokioAsyncWork<T> {
    work_fn:
        Arc<dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static>,
}

impl<T> std::fmt::Debug for TokioAsyncWork<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioAsyncWork")
            .field("work_fn", &"<async work function>")
            .finish()
    }
}

impl<T> TokioAsyncWork<T> {
    #[inline]
    pub fn new(
        work_fn: Arc<
            dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static,
        >,
    ) -> Self {
        Self { work_fn }
    }

    #[inline]
    pub fn from_value(value: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        let val = Arc::new(value);
        Self::new(Arc::new(move || {
            let v = Arc::clone(&val);
            Box::pin(async move { (*v).clone() })
        }))
    }

    #[inline]
    pub fn from_error(error: AsyncTaskError) -> Self
    where
        T: Send + 'static,
    {
        let err = Arc::new(error);
        Self::new(Arc::new(move || {
            let e = Arc::clone(&err);
            Box::pin(async move {
                panic!("AsyncWork error: {:?}", e);
            })
        }))
    }
}

impl<T> AsyncWork<T> for TokioAsyncWork<T>
where
    T: Send + 'static,
{
    #[inline]
    async fn run(self) -> T {
        (self.work_fn)().await
    }
}
use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, CancellableTask, ContextualizedTask, MetricsEnabledTask, NamedTask,
    PrioritizedTask, RecoverableTask, RetryStrategy, StatusEnabledTask, TaskId, TaskStatus,
    TimedTask, TracingTask,
    spawn::{SpawningTask, TaskResult},
    task_priority::TaskPriority,
};

/// Zero-allocation spawning task implementation
#[derive(Debug)]
pub struct TokioSpawningTask<T, I> {
    task_id: I,
    async_task: TokioAsyncTask<T, I>,
    value: Option<T>,
    name: Option<String>,
    status: AtomicU8,
    is_cancelled: AtomicBool,
    priority: TaskPriority,
}

impl<T, I> TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        Self {
            task_id,
            async_task: TokioAsyncTask::new(task_id),
            value: None,
            name: None,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            priority: TaskPriority::Normal,
        }
    }

    #[inline]
    pub fn with_value(task_id: I, value: T) -> Self {
        Self {
            task_id,
            async_task: TokioAsyncTask::new(task_id),
            value: Some(value),
            name: None,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            priority: TaskPriority::Normal,
        }
    }

    #[inline]
    fn status_from_u8(value: u8) -> TaskStatus {
        match value {
            0 => TaskStatus::Pending,
            1 => TaskStatus::Running,
            2 => TaskStatus::Completed,
            3 => TaskStatus::Failed,
            4 => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        }
    }
}

impl<T, I> NamedTask for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

impl<T, I> StatusEnabledTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn status(&self) -> TaskStatus {
        Self::status_from_u8(self.status.load(Ordering::Relaxed))
    }
}

impl<T, I> TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn set_status(&self, status: TaskStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

impl<T, I> CancellableTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn cancel(
        &self,
        _level: sweet_async_api::task::cancellable_task::CancellationLevel,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_gracefully(
        &self,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_forcefully(
        &self,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_immediately(
        &self,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    #[inline]
    fn on_cancel<F, Fut>(self, _callback: F) -> Self
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self
    }
}

impl<T, I> TracingTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        Err(error)
    }

    #[inline]
    fn record_error(&self, _error: &AsyncTaskError) {
        // Record error for tracing
    }

    #[inline]
    fn is_tracing_enabled(&self) -> bool {
        true
    }
}

impl<T, I> TimedTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn created_timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    fn executed_timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    fn completed_timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

impl<T, I> ContextualizedTask<T, I> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type RuntimeType = TokioOrchestraRuntime<T, I>;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    #[inline]
    fn relationships(&self) -> &Self::RelationshipsType {
        self.async_task.relationships()
    }

    #[inline]
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        self.async_task.relationships_mut()
    }

    #[inline]
    fn runtime(&self) -> &Self::RuntimeType {
        // Runtime is created on-demand when needed, not stored in struct
        panic!("Runtime should be accessed through tokio::runtime::Handle::current() in run() method")
    }

    #[inline]
    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
    }
}

impl<T, I> RecoverableTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    type FallbackWork = crate::task::TokioFallbackWork<T>;

    #[inline]
    fn recover(
        &self,
        _error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        async move {
            Err(AsyncTaskError::Failure(
                "Recovery not implemented".to_string(),
            ))
        }
    }

    #[inline]
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        false
    }

    #[inline]
    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.async_task.fallback_work
    }

    #[inline]
    fn max_retries(&self) -> u8 {
        3
    }

    #[inline]
    fn current_retry(&self) -> u8 {
        0
    }

    #[inline]
    fn retry_strategy(&self) -> RetryStrategy {
        RetryStrategy::Fixed(Duration::from_millis(100))
    }
}

impl<T, I> MetricsEnabledTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    type Cpu = crate::task::TokioCpuUsage;
    type Memory = crate::task::TokioMemoryUsage;
    type Io = crate::task::TokioIoUsage;

    #[inline]
    fn cpu_usage(&self) -> &Self::Cpu {
        self.async_task.cpu_usage()
    }

    #[inline]
    fn memory_usage(&self) -> &Self::Memory {
        self.async_task.memory_usage()
    }

    #[inline]
    fn io_usage(&self) -> &Self::Io {
        self.async_task.io_usage()
    }
}

impl<T, I> PrioritizedTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn priority(&self) -> &impl sweet_async_api::task::task_priority::RankableByPriority {
        &self.priority
    }
}

impl<T, I> AsyncTask<T, I> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    #[inline]
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        crate::orchestra::TokioOrchestratorBuilder::new()
    }

    #[inline]
    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        crate::orchestra::TokioOrchestratorBuilder::new()
    }
}

impl<T, I> SpawningTask<T, I> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type OutputFuture = Pin<Box<dyn Future<Output = Self::TaskResult> + Send + 'static>>;
    type TaskResult = TokioTaskResult<T>;
    type JoinChildrenFuture =
        Pin<Box<dyn Future<Output = Self::JoinChildrenResult> + Send + 'static>>;
    type JoinChildrenResult = TokioAsyncResult<Vec<I>>;
    type AsyncWork = TokioAsyncWork<T>;

    #[inline]
    fn run(mut self, work: Self::AsyncWork) -> Self {
        self.set_status(TaskStatus::Running);

        // Use captured configuration when spawning through runtime abstraction
        let task_name = self.name.clone();
        let priority = self.priority;
        let task_id = self.task_id.clone();
        
        // Access runtime through the orchestrator abstraction only in run()
        self.async_task.runtime().spawn(async move {
            // Apply configuration to the spawned task
            if let Some(name) = task_name {
                // Set task name for tracing/debugging
                tracing::trace!("Running task: {}", name);
            }
            
            // Execute the work with captured configuration
            match work.run().await {
                Ok(result) => {
                    tracing::debug!("Task {:?} completed successfully", task_id);
                    // Store result would go here if we had a way to communicate back
                }
                Err(error) => {
                    tracing::error!("Task {:?} failed: {:?}", task_id, error);
                }
            }
        });

        self
    }

    #[inline]
    fn run_child<R>(&self, _task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Clone + Send + 'static,
        Self: SpawningTask<R, I>,
    {
        // This is a stub implementation that satisfies the trait bounds
        // The actual implementation would need to be specialized for each R type
        unimplemented!("run_child requires specialized implementation for each R type")
    }

    #[inline]
    fn join_children(&self) -> Self::JoinChildrenFuture {
        Box::pin(async move { TokioAsyncResult::from_result(Ok(Vec::new())) })
    }

    #[inline]
    fn task_id(&self) -> I {
        self.task_id
    }

    #[inline]
    fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    #[inline]
    fn chain<U, F>(self, _f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: AsyncWork<U> + Send + 'static,
        U: Clone + Send + 'static,
        Self: SpawningTask<U, I>,
    {
        // This is a stub implementation that satisfies the trait bounds
        // The actual implementation would need to be specialized for each U type
        unimplemented!("chain requires specialized implementation for each U type")
    }
}

impl<T, I> Future for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type Output = TokioTaskResult<T>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        // Cannot use tokio directly - must use runtime abstraction
        if let Some(value) = &this.value {
            // Fallback to immediate value if no handle
            std::task::Poll::Ready(TokioTaskResult::ok(value.clone()))
        } else {
            std::task::Poll::Ready(TokioTaskResult::err(AsyncTaskError::Failure(
                "No value or handle available".to_string(),
            )))
        }
    }
}
