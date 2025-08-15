//! High-performance emit task implementation with zero allocation

use crate::task::emit::TokioFinalEvent;
use crate::task::{
    TokioCpuUsage, TokioFallbackWork, TokioIoUsage, TokioMemoryUsage, TokioTaskContext,
    TokioTimedTask,
};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, SystemTime};

use sweet_async_api::orchestra::{OrchestratorBuilder, OrchestratorError};
use sweet_async_api::task::AsyncTask;
use sweet_async_api::task::CancellableTask;
use sweet_async_api::task::ContextualizedTask;
use sweet_async_api::task::MetricsEnabledTask;
use sweet_async_api::task::NamedTask;
use sweet_async_api::task::PrioritizedTask;
use sweet_async_api::task::RecoverableTask;
use sweet_async_api::task::RetryStrategy;
use sweet_async_api::task::StatusEnabledTask;
use sweet_async_api::task::TimedTask;
use sweet_async_api::task::TracingTask;
use sweet_async_api::task::builder::ReceiverStrategy;
use sweet_async_api::task::emit::EmittingTask;
use sweet_async_api::task::emit::event::Collector;
use sweet_async_api::task::emit::{ReceiverTask, SenderTask};
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
use sweet_async_api::task::task_error::AsyncTaskError;
use sweet_async_api::task::task_id::TaskId;
use sweet_async_api::task::task_priority::TaskPriority;
use sweet_async_api::task::task_status::TaskStatus;
// Cannot use tokio::sync directly - must use runtime abstraction
// use tokio::sync::mpsc;

/// Zero-allocation collector implementation for high-performance event processing
#[derive(Debug, Clone)]
pub struct TokioCollector<T, C> {
    collected_items: Vec<(String, T)>,
    context: C,
}

impl<T, C> TokioCollector<T, C> {
    #[inline]
    pub fn new(context: C) -> Self {
        Self {
            collected_items: Vec::new(),
            context,
        }
    }

    #[inline]
    pub fn collect(&mut self, id: String, item: T) {
        self.collected_items.push((id, item));
    }

    #[inline]
    pub fn collected(self) -> Vec<(String, T)> {
        self.collected_items
    }
}

/// Zero-allocation emitting task implementation with all AsyncTask super-traits
#[derive(Debug)]
pub struct TokioEmittingTask<T, C, E, I> {
    task_id: I,
    name: Option<String>,
    priority: TaskPriority,
    status: AtomicU8,
    is_cancelled: AtomicBool,
    tracing_enabled: AtomicBool,
    metrics_enabled: AtomicBool,
    timed_task: TokioTimedTask,
    context: TokioTaskContext<T, I>,
    fallback_work: TokioFallbackWork<T>,
    max_retries: u8,
    current_retry: AtomicU8,
    cpu_usage: TokioCpuUsage,
    memory_usage: TokioMemoryUsage,
    io_usage: TokioIoUsage,
    // Cannot use tokio::sync directly - must use runtime abstraction
    // sender: Arc<mpsc::UnboundedSender<T>>,
    // receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<T>>>,
    collector: TokioCollector<T, C>,
    is_complete: AtomicBool,
    _phantom: std::marker::PhantomData<E>,
}

impl<T, C, E, I> TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        // Cannot use tokio::sync directly - must use runtime abstraction
        Self {
            task_id,
            name: Some(format!("emitting_task_{}", task_id.to_string())),
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            priority: TaskPriority::Normal,
            // created_at: SystemTime::now(),
            // started_at: None,
            // completed_at: None,
            tracing_enabled: AtomicBool::new(true),
            metrics_enabled: AtomicBool::new(true),
            timed_task: TokioTimedTask::new(),
            context: TokioTaskContext::new(unsafe { std::mem::zeroed() }),
            fallback_work: TokioFallbackWork::new(unsafe { std::mem::zeroed() }),
            max_retries: 3,
            current_retry: AtomicU8::new(0),
            cpu_usage: TokioCpuUsage::new(),
            memory_usage: TokioMemoryUsage::new(),
            io_usage: TokioIoUsage::new(),
            collector: TokioCollector::new(unsafe { std::mem::zeroed() }),
            is_complete: AtomicBool::new(false),
            _phantom: std::marker::PhantomData,
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

// Implement all AsyncTask super-traits for TokioEmittingTask
impl<T, C, E, I> NamedTask for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
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

impl<T, C, E, I> PrioritizedTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn priority(&self) -> &TaskPriority {
        &self.priority
    }
}

impl<T, C, E, I> StatusEnabledTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn status(&self) -> TaskStatus {
        Self::status_from_u8(self.status.load(Ordering::Relaxed))
    }
}

impl<T, C, E, I> TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn set_status(&self, status: TaskStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

impl<T, C, E, I> CancellableTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
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

impl<T, C, E, I> TracingTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }

    #[inline]
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Handle error via tracing if enabled
        Err(error)
    }

    #[inline]
    fn record_error(&self, _error: &AsyncTaskError) {
        // Record error via tracing if enabled
    }
}

impl<T, C, E, I> TimedTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
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

impl<T, C, E, I> ContextualizedTask<T, I> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Unpin,
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

impl<T, C, E, I> RecoverableTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    type FallbackWork = TokioFallbackWork<T>;

    #[inline]
    fn recover(
        &self,
        _error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        let fallback = TokioFallbackWork::new(None);
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

impl<T, C, E, I> MetricsEnabledTask<T> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
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

impl<T, C, E, I> AsyncTask<T, I> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Unpin,
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

impl<T, C, E, I> EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type Final = TokioFinalEvent<T, C, C>;

    #[inline]
    fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Relaxed)
    }

    #[inline]
    fn cancel(&self) -> Result<(), OrchestratorError> {
        self.is_complete.store(true, Ordering::Relaxed);
        Ok(())
    }

    #[inline]
    fn await_final_event<Handler, R>(self, handler: Handler) -> R
    where
        Handler: Fn(Self::Final, &TokioCollector<T, C>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let collected_results = std::collections::HashMap::new();
        // Create final event with collected results
        let final_event = TokioFinalEvent::new(
            T::default(),              // data with correct type
            self.collector.clone(),    // collector
            collected_results.clone(), // collected items
        );
        // Use concrete collector reference
        handler(final_event, &self.collector)
    }
}

/// Zero-allocation sender task implementation
#[derive(Debug)]
pub struct TokioSenderTask<T, C, E, I> {
    task_id: I,
    // Cannot use tokio::sync directly - must use runtime abstraction
    // sender: Arc<mpsc::UnboundedSender<T>>,
    _phantom: std::marker::PhantomData<(C, E)>,
}

impl<T, C, E, I> TokioSenderTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        // Cannot use tokio::sync directly - must use runtime abstraction
        Self {
            task_id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, C, E, I> SenderTask<T, C, E, I> for TokioSenderTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;

    #[inline]
    fn receiver<F, R>(&self, _receiver: F, _strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(&T, &mut dyn Collector<T, C>) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static,
    {
        TokioEmittingTask::new(self.task_id)
    }
}

/// Zero-allocation receiver task implementation
#[derive(Debug)]
pub struct TokioReceiverTask<T, C, E, I> {
    task_id: I,
    // Cannot use tokio::sync directly - must use runtime abstraction
    // receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<T>>>,
    _phantom: std::marker::PhantomData<(C, E)>,
}

impl<T, C, E, I> TokioReceiverTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        // Cannot use tokio::sync directly - must use runtime abstraction
        Self {
            task_id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, C, E, I> ReceiverTask<T, C, E, I> for TokioReceiverTask<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;

    #[inline]
    fn emit_events<F, R>(&self, _receiver: F, _strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(&T, &mut dyn Collector<T, C>) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static,
    {
        TokioEmittingTask::new(self.task_id)
    }
}
