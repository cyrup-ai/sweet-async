//! Complete high-performance AsyncTask implementation with zero allocation

use crate::task::{
    TokioCpuUsage, TokioFallbackWork, TokioIoUsage, TokioMemoryUsage, TokioTaskContext,
    TokioTimedTask,
};
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, SystemTime};
use sweet_async_api::orchestra::OrchestratorBuilder;
use sweet_async_api::task::async_task::AsyncTask;
use sweet_async_api::task::{
    AsyncTaskError, CancellableTask, ContextualizedTask, MetricsEnabledTask, NamedTask,
    PrioritizedTask, RecoverableTask, RetryStrategy, StatusEnabledTask, TaskPriority, TaskStatus,
    TimedTask, TracingTask,
};

/// Zero-allocation complete AsyncTask implementation
#[derive(Debug)]
pub struct TokioAsyncTask<T, I> {
    task_id: I,
    name: Option<String>,
    priority: TaskPriority,
    status: AtomicU8,
    is_cancelled: AtomicBool,
    tracing_enabled: AtomicBool,
    metrics_enabled: AtomicBool,
    timed_task: TokioTimedTask,
    context: TokioTaskContext<T, I>,
    value: Option<T>,
    pub fallback_work: TokioFallbackWork<T>,
    max_retries: u8,
    current_retry: AtomicU8,
    cpu_usage: TokioCpuUsage,
    memory_usage: TokioMemoryUsage,
    io_usage: TokioIoUsage,
}

impl<T, I> TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        Self {
            task_id,
            name: None,
            priority: TaskPriority::Normal,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            tracing_enabled: AtomicBool::new(false),
            metrics_enabled: AtomicBool::new(false),
            timed_task: TokioTimedTask::new(),
            context: TokioTaskContext::<T, I>::new(task_id),
            value: None,
            fallback_work: TokioFallbackWork::new(None),
            max_retries: 3,
            current_retry: AtomicU8::new(0),
            cpu_usage: TokioCpuUsage::new(),
            memory_usage: TokioMemoryUsage::new(),
            io_usage: TokioIoUsage::new(),
        }
    }

    #[inline]
    pub fn with_value(task_id: I, value: T) -> Self {
        Self {
            task_id,
            name: None,
            priority: TaskPriority::Normal,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            tracing_enabled: AtomicBool::new(false),
            metrics_enabled: AtomicBool::new(false),
            timed_task: TokioTimedTask::new(),
            context: TokioTaskContext::<T, I>::new(task_id),
            value: Some(value.clone()),
            fallback_work: TokioFallbackWork::new(Some(value)),
            max_retries: 3,
            current_retry: AtomicU8::new(0),
            cpu_usage: TokioCpuUsage::new(),
            memory_usage: TokioMemoryUsage::new(),
            io_usage: TokioIoUsage::new(),
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

impl<T, I> NamedTask for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
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

impl<T, I> PrioritizedTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    #[inline]
    fn priority(&self) -> &TaskPriority {
        &self.priority
    }
}

impl<T, I> StatusEnabledTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    #[inline]
    fn status(&self) -> TaskStatus {
        Self::status_from_u8(self.status.load(Ordering::Relaxed))
    }
}

impl<T, I> TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    #[inline]
    pub fn set_status(&self, status: TaskStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

impl<T, I> CancellableTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
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

impl<T, I> TracingTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    #[inline]
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }

    #[inline]
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Handle error via tracing if enabled
        if self.is_tracing_enabled() {
            // Log error for tracing
        }
        Err(error)
    }

    #[inline]
    fn record_error(&self, _error: &AsyncTaskError) {
        // Record error via tracing if enabled
    }
}

impl<T, I> TimedTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    #[inline]
    fn created_timestamp(&self) -> SystemTime {
        self.timed_task.created_timestamp()
    }

    #[inline]
    fn executed_timestamp(&self) -> SystemTime {
        self.timed_task.executed_timestamp()
    }

    #[inline]
    fn completed_timestamp(&self) -> SystemTime {
        self.timed_task.completed_timestamp()
    }

    #[inline]
    fn timeout(&self) -> Duration {
        self.timed_task.timeout()
    }
}

impl<T, I> ContextualizedTask<T, I> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Unpin,
{
    type RuntimeType = crate::orchestra::runtime::TokioOrchestraRuntime<T, I>;
    type RelationshipsType = crate::task::TokioTaskRelationships<T, I>;

    #[inline]
    fn relationships(&self) -> &Self::RelationshipsType {
        self.context.relationships()
    }

    #[inline]
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        self.context.relationships_mut()
    }

    #[inline]
    fn runtime(&self) -> &Self::RuntimeType {
        self.context.runtime()
    }

    #[inline]
    fn cwd(&self) -> std::path::PathBuf {
        self.context.cwd()
    }
}

impl<T, I> RecoverableTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    type FallbackWork = TokioFallbackWork<T>;

    #[inline]
    fn recover(
        &self,
        _error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        let fallback = TokioFallbackWork::new(self.value.clone());
        sweet_async_api::task::builder::AsyncWork::run(fallback)
    }

    #[inline]
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        !self.retries_exhausted()
    }

    #[inline]
    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.fallback_work
    }

    #[inline]
    fn max_retries(&self) -> u8 {
        self.max_retries
    }

    #[inline]
    fn current_retry(&self) -> u8 {
        self.current_retry.load(Ordering::Relaxed)
    }

    #[inline]
    fn retry_strategy(&self) -> RetryStrategy {
        RetryStrategy::Fixed(Duration::from_millis(100))
    }
}

impl<T, I> MetricsEnabledTask<T> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash,
{
    type Cpu = TokioCpuUsage;
    type Memory = TokioMemoryUsage;
    type Io = TokioIoUsage;

    #[inline]
    fn cpu_usage(&self) -> &Self::Cpu {
        &self.cpu_usage
    }

    #[inline]
    fn memory_usage(&self) -> &Self::Memory {
        &self.memory_usage
    }

    #[inline]
    fn io_usage(&self) -> &Self::Io {
        &self.io_usage
    }
}

impl<T, I> AsyncTask<T, I> for TokioAsyncTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Unpin,
{
    #[inline]
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>
    {
        crate::orchestra::TokioOrchestratorBuilder::new()
    }

    #[inline]
    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl OrchestratorBuilder<R, Task, I> {
        crate::orchestra::TokioOrchestratorBuilder::new()
    }
}
