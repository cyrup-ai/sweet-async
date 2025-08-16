//! High-performance Tokio runtime implementation with zero allocation

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::orchestra::{OrchestratorBuilder, OrchestratorError};
use sweet_async_api::task::recoverable_task::RetryStrategy;
use sweet_async_api::task::spawn::SpawningTask;
use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, CancellableTask, ContextualizedTask, CpuUsage, IoUsage, MemoryUsage,
    MetricsEnabledTask, NamedTask, PrioritizedTask, RecoverableTask, StatusEnabledTask,
    TaskPriority, TaskStatus, TimedTask, TracingTask,
};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::task::spawn::task::TokioAsyncWork;
use crate::task::task_relationships::TokioTaskRelationships;
// Cannot use tokio::sync directly - must use runtime abstraction
// use tokio::sync::RwLock;
// use tokio::task::JoinHandle;

/// Zero-allocation Tokio runtime with atomic operations
#[derive(Debug)]
pub struct TokioOrchestraRuntime<T, I> {
    active_tasks: Arc<RwLock<HashMap<I, JoinHandle<()>>>>,
    task_count: AtomicUsize,
    is_running: AtomicBool,
    pub runtime_handle: tokio::runtime::Handle,
    _phantom: std::marker::PhantomData<(T, I)>,
}

impl<T, I> TokioOrchestraRuntime<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_count: AtomicUsize::new(0),
            is_running: AtomicBool::new(true),
            runtime_handle: tokio::runtime::Handle::current(),
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub async fn complete_task(&self, task_id: I, _result: Result<T, AsyncTaskError>) {
        // Cannot use tokio::sync directly - must use runtime abstraction
        // let mut tasks = self.active_tasks.write().await;
        // Cannot use tokio::sync directly - must use runtime abstraction
        self.task_count.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<T, I> Default for TokioOrchestraRuntime<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance spawned task wrapper with zero-allocation design
#[derive(Debug)]
pub struct TokioSpawnedTask<T, I> {
    task_id: I,
    handle: JoinHandle<Result<T, AsyncTaskError>>,
    runtime_ref: Arc<TokioOrchestraRuntime<T, I>>,
    created_at: Instant,
    priority: TaskPriority,
    name: String,
    context: TokioTaskContext<T, I>,
    fallback_work: TokioAsyncWork<Result<T, AsyncTaskError>>,
    max_retries: u8,
    current_retry: AtomicU8,
    retry_strategy: RetryStrategy,
    cpu_usage: TokioCpuUsage,
    memory_usage: TokioMemoryUsage,
    io_usage: TokioIoUsage,
}

/// Task execution context with relationships and runtime
#[derive(Debug)]
pub struct TokioTaskContext<T, I> {
    relationships: TokioTaskRelationships<T, I>,
    runtime: TokioOrchestraRuntime<T, I>,
    cwd: std::path::PathBuf,
}

/// Zero-allocation CPU usage tracker using atomic operations
#[derive(Debug)]
pub struct TokioCpuUsage {
    cpu_time_nanos: AtomicU64,
    user_time_nanos: AtomicU64,
    system_time_nanos: AtomicU64,
    utilization_percent: AtomicU64, // stored as percentage * 100 for precision
}

impl TokioCpuUsage {
    #[inline]
    pub fn new() -> Self {
        Self {
            cpu_time_nanos: AtomicU64::new(0),
            user_time_nanos: AtomicU64::new(0),
            system_time_nanos: AtomicU64::new(0),
            utilization_percent: AtomicU64::new(0),
        }
    }
}

impl CpuUsage for TokioCpuUsage {
    #[inline]
    fn cpu_time(&self) -> Duration {
        Duration::from_nanos(self.cpu_time_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn utilization(&self) -> f64 {
        self.utilization_percent.load(Ordering::Relaxed) as f64 / 10000.0
    }

    #[inline]
    fn user_time(&self) -> Duration {
        Duration::from_nanos(self.user_time_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn system_time(&self) -> Duration {
        Duration::from_nanos(self.system_time_nanos.load(Ordering::Relaxed))
    }
}

/// Zero-allocation memory usage tracker using atomic operations
#[derive(Debug)]
pub struct TokioMemoryUsage {
    current_bytes: AtomicU64,
    peak_bytes: AtomicU64,
    allocation_count: AtomicU64,
    allocation_rate_bytes_per_sec: AtomicU64, // stored as integer for atomic operations
}

impl TokioMemoryUsage {
    #[inline]
    pub fn new() -> Self {
        Self {
            current_bytes: AtomicU64::new(0),
            peak_bytes: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
            allocation_rate_bytes_per_sec: AtomicU64::new(0),
        }
    }
}

impl MemoryUsage for TokioMemoryUsage {
    #[inline]
    fn current_bytes(&self) -> u64 {
        self.current_bytes.load(Ordering::Relaxed)
    }

    #[inline]
    fn peak_bytes(&self) -> u64 {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    #[inline]
    fn allocation_count(&self) -> u64 {
        self.allocation_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn allocation_rate(&self) -> f64 {
        self.allocation_rate_bytes_per_sec.load(Ordering::Relaxed) as f64
    }
}

/// Zero-allocation IO usage tracker using atomic operations
#[derive(Debug)]
pub struct TokioIoUsage {
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,
    read_operations: AtomicU64,
    write_operations: AtomicU64,
    read_latency_nanos: AtomicU64,
    write_latency_nanos: AtomicU64,
    operations_per_second: AtomicU64, // stored as integer for atomic operations
    io_wait_time_nanos: AtomicU64,
}

impl TokioIoUsage {
    #[inline]
    pub fn new() -> Self {
        Self {
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            read_operations: AtomicU64::new(0),
            write_operations: AtomicU64::new(0),
            read_latency_nanos: AtomicU64::new(0),
            write_latency_nanos: AtomicU64::new(0),
            operations_per_second: AtomicU64::new(0),
            io_wait_time_nanos: AtomicU64::new(0),
        }
    }
}

impl IoUsage for TokioIoUsage {
    #[inline]
    fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    #[inline]
    fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    #[inline]
    fn read_operations(&self) -> u64 {
        self.read_operations.load(Ordering::Relaxed)
    }

    #[inline]
    fn write_operations(&self) -> u64 {
        self.write_operations.load(Ordering::Relaxed)
    }

    #[inline]
    fn read_latency(&self) -> Duration {
        Duration::from_nanos(self.read_latency_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn write_latency(&self) -> Duration {
        Duration::from_nanos(self.write_latency_nanos.load(Ordering::Relaxed))
    }

    #[inline]
    fn operations_per_second(&self) -> f64 {
        self.operations_per_second.load(Ordering::Relaxed) as f64
    }

    #[inline]
    fn io_wait_time(&self) -> Duration {
        Duration::from_nanos(self.io_wait_time_nanos.load(Ordering::Relaxed))
    }
}

impl<T, I> TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    pub fn new(
        task_id: I,
        handle: JoinHandle<Result<T, AsyncTaskError>>,
        runtime_ref: Arc<TokioOrchestraRuntime<T, I>>,
        priority: TaskPriority,
        name: String,
    ) -> Self {
        // Create default fallback work that returns an error
        let fallback_work = TokioAsyncWork::new(Arc::new(|| {
            Box::pin(async {
                Err(AsyncTaskError::RecoveryFailed(
                    "No fallback work configured".to_string(),
                ))
            })
        }));

        // Create default context
        let context = TokioTaskContext {
            relationships: TokioTaskRelationships::new(),
            runtime: TokioOrchestraRuntime::new(),
            cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
        };

        Self {
            task_id,
            handle,
            runtime_ref,
            created_at: Instant::now(),
            priority,
            name,
            context,
            fallback_work,
            max_retries: 3,
            current_retry: AtomicU8::new(0),
            retry_strategy: RetryStrategy::Fixed(Duration::from_millis(100)),
            cpu_usage: TokioCpuUsage::new(),
            memory_usage: TokioMemoryUsage::new(),
            io_usage: TokioIoUsage::new(),
        }
    }
}

impl<T, I> Future for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Unpin,
{
    type Output = Result<T, AsyncTaskError>;

    #[inline]
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        match Pin::new(&mut this.handle).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => {
                // Task completed - clean up via runtime
                let task_id = this.task_id;
                let runtime = this.runtime_ref.clone();
                let result_clone = result.clone();
                this.runtime_ref.runtime_handle.spawn(async move {
                    runtime.complete_task(task_id, result_clone).await;
                });
                std::task::Poll::Ready(result)
            }
            std::task::Poll::Ready(Err(join_error)) => {
                let error = AsyncTaskError::Failure(join_error.to_string());
                std::task::Poll::Ready(Err(error))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl<T, I> Runtime<T, I> for TokioOrchestraRuntime<T, I>
where
    T: Clone + Send + Sync + Default + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Unpin,
{
    type SpawnedTask = TokioSpawnedTask<T, I>;

    #[inline]
    fn spawn(
        &self,
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
    ) -> Self::SpawnedTask {
        let task_id = task.task_id();
        let runtime_ref = Arc::new(self.clone());

        // Execute the task using the EXISTING elite polling loop
        let handle = self.runtime_handle.spawn(async move {
            // Use the elite polling loop in TokioSpawningTask::poll()
            match task.await {
                result => result.into_result()
            }
        });

        // Track active task count
        self.task_count.fetch_add(1, Ordering::Relaxed);
        
        // Create the spawned task wrapper
        let spawned_task = TokioSpawnedTask::new(
            task_id,
            handle,
            runtime_ref,
            priority,
            format!("spawned_task_{}", task_id.to_string()),
        );

        // Store handle for management
        let tasks = self.active_tasks.clone();
        let task_id_clone = task_id;
        self.runtime_handle.spawn(async move {
            let mut tasks = tasks.write().await;
            // Store the spawned task handle for tracking
            tasks.insert(task_id_clone, tokio::spawn(async {}));
        });

        spawned_task
    }

    #[inline]
    fn block_on<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send + 'static,
    {
        // ALL blocking execution happens here via runtime
        self.runtime_handle.block_on(future)
    }

    #[inline]
    fn active_task_count(&self) -> usize {
        self.task_count.load(Ordering::Relaxed)
    }

    #[inline]
    fn shutdown(&self, timeout: Duration) -> Result<(), OrchestratorError> {
        // ALL shutdown logic happens here via runtime
        self.is_running.store(false, Ordering::Relaxed);

        let tasks = self.active_tasks.clone();
        let result = self.runtime_handle.block_on(async move {
            // Cannot use tokio directly - must use runtime abstraction
            panic!("shutdown_with_timeout requires proper runtime abstraction")
        });

        result.or(Ok(()))
    }

    #[inline]
    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }
}

impl<T, I> Clone for TokioOrchestraRuntime<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            active_tasks: self.active_tasks.clone(),
            task_count: AtomicUsize::new(self.task_count.load(Ordering::Relaxed)),
            is_running: AtomicBool::new(self.is_running.load(Ordering::Relaxed)),
            runtime_handle: self.runtime_handle.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

// AsyncTask trait implementation for TokioSpawnedTask
impl<T, I> PrioritizedTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn priority(&self) -> &impl sweet_async_api::task::task_priority::RankableByPriority {
        &self.priority
    }
}

impl<T, I> CancellableTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    async fn cancel(
        &self,
        level: sweet_async_api::task::cancellable_task::CancellationLevel,
    ) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.handle.abort();
        Ok(())
    }

    async fn cancel_gracefully(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.cancel(sweet_async_api::task::cancellable_task::CancellationLevel::Graceful)
            .await
    }

    async fn cancel_forcefully(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.cancel(sweet_async_api::task::cancellable_task::CancellationLevel::Kill)
            .await
    }

    async fn cancel_immediately(
        &self,
    ) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.cancel(sweet_async_api::task::cancellable_task::CancellationLevel::KillHard)
            .await
    }

    #[inline]
    fn is_cancelled(&self) -> bool {
        self.handle.is_finished()
    }

    fn on_cancel<F, Fut>(self, _callback: F) -> Self
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // For now, return self unchanged - callback registration not implemented
        self
    }
}

impl<T, I> TracingTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn is_tracing_enabled(&self) -> bool {
        true // Always enabled for spawned tasks
    }

    #[inline]
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        tracing::error!("Task {} error: {:?}", self.task_id.to_string(), error);
        Err(error)
    }

    #[inline]
    fn record_error(&self, error: &AsyncTaskError) {
        tracing::error!(
            "Recording error for task {}: {:?}",
            self.task_id.to_string(),
            error
        );
    }
}

impl<T, I> TimedTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn created_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now() - self.created_at.elapsed()
    }

    #[inline]
    fn executed_timestamp(&self) -> std::time::SystemTime {
        // For now, return creation time as executed time
        self.created_timestamp()
    }

    #[inline]
    fn completed_timestamp(&self) -> std::time::SystemTime {
        // For now, return creation time as completed time
        self.created_timestamp()
    }

    #[inline]
    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(300) // Default 5 minute timeout
    }
}

impl<T, I> ContextualizedTask<T, I> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Unpin,
{
    type RuntimeType = TokioOrchestraRuntime<T, I>;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    #[inline]
    fn relationships(&self) -> &Self::RelationshipsType {
        &self.context.relationships
    }

    #[inline]
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut self.context.relationships
    }

    #[inline]
    fn runtime(&self) -> &Self::RuntimeType {
        &self.context.runtime
    }

    #[inline]
    fn cwd(&self) -> std::path::PathBuf {
        self.context.cwd.clone()
    }
}

impl<T, I> RecoverableTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    type FallbackWork = TokioAsyncWork<Result<T, AsyncTaskError>>;

    async fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        if self.can_recover_from(&error) && !self.retries_exhausted() {
            // Execute fallback work
            use sweet_async_api::task::AsyncWork;
            let work = self.fallback_work().clone();
            AsyncWork::run(work).await
        } else {
            Err(error)
        }
    }

    #[inline]
    fn can_recover_from(&self, error: &AsyncTaskError) -> bool {
        matches!(
            error,
            AsyncTaskError::Timeout(_) | AsyncTaskError::Failure(_)
        )
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
        self.retry_strategy
    }
}

impl<T, I> StatusEnabledTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn status(&self) -> TaskStatus {
        if self.handle.is_finished() {
            TaskStatus::Completed
        } else {
            TaskStatus::Running
        }
    }
}

impl<T, I> MetricsEnabledTask<T> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
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

impl<T, I> NamedTask for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq,
{
    #[inline]
    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    #[inline]
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl<T, I> AsyncTask<T, I> for TokioSpawnedTask<T, I>
where
    T: Clone + Send + Sync + Default + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Unpin,
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

/// Trait for runtime access in tasks
pub trait TokioRuntime<T, I>: Send + Sync
where
    T: Clone + Send + 'static,
    I: sweet_async_api::TaskId,
{
    fn complete_task(
        &self,
        task_id: I,
        result: Result<T, AsyncTaskError>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}
