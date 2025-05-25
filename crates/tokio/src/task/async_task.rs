use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::{
    AsyncTask as ApiAsyncTask, AsyncTaskError, CancellableTask, CancellationLevel,
    ContextualizedTask, CpuUsage, DistributedTask, IoUsage, MemoryUsage, MetricsEnabledTask, 
    NamedTask, PrioritizedTask, RecoverableTask, RetryStrategy, StatusEnabledTask, TaskId, 
    TaskPriority, TaskStatus, TimedTask, TracingTask, TaskRelationships, TaskEnvelope, 
    TaskMessage as ApiTaskMessage,
};
use sweet_async_api::task::builder::AsyncWork;

/// Default fallback that just returns the error
#[derive(Clone)]
pub struct ErrorFallback<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + 'static> AsyncWork<Result<T, AsyncTaskError>> for ErrorFallback<T> {
    fn run(self) -> Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>> {
        Box::pin(async move {
            Err(AsyncTaskError::Failure("Task failed with no recovery".to_string()))
        })
    }
}

impl<T> Default for ErrorFallback<T> {
    fn default() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

/// Task metrics implementation
#[cfg(feature = "metrics")]
#[derive(Clone, Default)]
pub struct TaskMetrics {
    cpu_time_nanos: AtomicU64,
    memory_current: AtomicU64,
    memory_peak: AtomicU64,
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,
}

#[cfg(feature = "metrics")]
impl TaskMetrics {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "metrics")]
impl CpuUsage for TaskMetrics {
    fn cpu_time(&self) -> Duration {
        Duration::from_nanos(self.cpu_time_nanos.load(Ordering::Relaxed))
    }
    
    fn utilization(&self) -> f64 {
        0.0 // Would need more sophisticated tracking
    }
    
    fn user_time(&self) -> Duration {
        self.cpu_time()
    }
    
    fn system_time(&self) -> Duration {
        Duration::ZERO
    }
}

#[cfg(feature = "metrics")]
impl MemoryUsage for TaskMetrics {
    fn current_bytes(&self) -> u64 {
        self.memory_current.load(Ordering::Relaxed)
    }
    
    fn peak_bytes(&self) -> u64 {
        self.memory_peak.load(Ordering::Relaxed)
    }
    
    fn allocation_count(&self) -> u64 {
        0
    }
    
    fn allocation_rate(&self) -> f64 {
        0.0
    }
}

#[cfg(feature = "metrics")]
impl IoUsage for TaskMetrics {
    fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }
    
    fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }
    
    fn read_operations(&self) -> u64 {
        0
    }
    
    fn write_operations(&self) -> u64 {
        0
    }
    
    fn read_latency(&self) -> Duration {
        Duration::ZERO
    }
    
    fn write_latency(&self) -> Duration {
        Duration::ZERO
    }
    
    fn operations_per_second(&self) -> f64 {
        0.0
    }
    
    fn io_wait_time(&self) -> Duration {
        Duration::ZERO
    }
}

// Empty metrics for when feature is disabled
#[cfg(not(feature = "metrics"))]
#[derive(Clone, Default)]
pub struct TaskMetrics;

#[cfg(not(feature = "metrics"))]
impl CpuUsage for TaskMetrics {
    fn cpu_time(&self) -> Duration { Duration::ZERO }
    fn utilization(&self) -> f64 { 0.0 }
    fn user_time(&self) -> Duration { Duration::ZERO }
    fn system_time(&self) -> Duration { Duration::ZERO }
}

#[cfg(not(feature = "metrics"))]
impl MemoryUsage for TaskMetrics {
    fn current_bytes(&self) -> u64 { 0 }
    fn peak_bytes(&self) -> u64 { 0 }
    fn allocation_count(&self) -> u64 { 0 }
    fn allocation_rate(&self) -> f64 { 0.0 }
}

#[cfg(not(feature = "metrics"))]
impl IoUsage for TaskMetrics {
    fn bytes_read(&self) -> u64 { 0 }
    fn bytes_written(&self) -> u64 { 0 }
    fn read_operations(&self) -> u64 { 0 }
    fn write_operations(&self) -> u64 { 0 }
    fn read_latency(&self) -> Duration { Duration::ZERO }
    fn write_latency(&self) -> Duration { Duration::ZERO }
    fn operations_per_second(&self) -> f64 { 0.0 }
    fn io_wait_time(&self) -> Duration { Duration::ZERO }
}

/// Helper to convert TaskStatus to u8 for atomic storage
fn status_to_u8(status: &TaskStatus) -> u8 {
    match status {
        TaskStatus::Pending => 0,
        TaskStatus::Running => 1,
        TaskStatus::PendingCancellation => 2,
        TaskStatus::Cancelled => 3,
        TaskStatus::Completed => 4,
    }
}

/// Helper to convert u8 to TaskStatus
fn u8_to_status(val: u8) -> TaskStatus {
    match val {
        0 => TaskStatus::Pending,
        1 => TaskStatus::Running,
        2 => TaskStatus::PendingCancellation,
        3 => TaskStatus::Cancelled,
        4 => TaskStatus::Completed,
        _ => TaskStatus::Pending,
    }
}

/// Vector clock stub when distributed feature is disabled
#[cfg(not(feature = "distributed"))]
#[derive(Clone, Default)]
pub struct VectorClock<I> {
    _phantom: std::marker::PhantomData<I>,
}

/// Type alias for convenience
pub type TokioAsyncTask<T, I> = AsyncTask<T, I, ErrorFallback<T>>;

/// Clean async task implementation with only trait-required fields
pub struct AsyncTask<T: Clone + Send + 'static, I: TaskId, F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static = ErrorFallback<T>> {
    // Core identity (required by traits)
    id: I,
    name: String,
    
    // Status (StatusEnabledTask)
    atomic_status: AtomicU8,
    
    // Priority (PrioritizedTask)
    priority: TaskPriority,
    
    // Timing (TimedTask)
    created_time: SystemTime,
    atomic_start_time: AtomicU64,
    atomic_end_time: AtomicU64,
    timeout: Duration,
    
    // Recovery (RecoverableTask)
    fallback_work: F,
    max_retries: u8,
    current_retry: AtomicU8,
    retry_strategy: RetryStrategy,
    
    // Metrics (MetricsEnabledTask)
    metrics: TaskMetrics,
    
    // Tracing (TracingTask)  
    tracing_enabled: bool,
    
    // Context (ContextualizedTask)
    cwd: PathBuf,
    relationships: TaskRelationships<T, I>,
    
    // Distribution (DistributedTask)
    #[cfg(feature = "distributed")]
    vector_clock: crate::task::vector_clock::VectorClock<I>,
    
    #[cfg(not(feature = "distributed"))]
    vector_clock: VectorClock<I>,
    
    // Cancellation (CancellableTask)
    atomic_cancelled: AtomicBool,
}

impl<T: Clone + Send + 'static, I: TaskId, F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static> AsyncTask<T, I, F> {
    /// Create a new task
    pub fn new(id: I, fallback_work: F) -> Self {
        Self {
            id: id.clone(),
            name: format!("Task #{}", id),
            atomic_status: AtomicU8::new(status_to_u8(&TaskStatus::Pending)),
            priority: TaskPriority::Normal,
            created_time: SystemTime::now(),
            atomic_start_time: AtomicU64::new(0),
            atomic_end_time: AtomicU64::new(0),
            timeout: Duration::from_secs(90),
            fallback_work,
            max_retries: 1,
            current_retry: AtomicU8::new(0),
            retry_strategy: RetryStrategy::Fixed(Duration::from_secs(5)),
            metrics: TaskMetrics::default(),
            tracing_enabled: false,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            relationships: TaskRelationships::default(),
            vector_clock: Default::default(),
            atomic_cancelled: AtomicBool::new(false),
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTask<T, I, ErrorFallback<T>> {
    /// Create a new task with default error fallback
    pub fn with_defaults(id: I) -> Self {
        Self::new(id, ErrorFallback::default())
    }
}

// Trait implementations...

impl<T: Clone + Send + 'static, I: TaskId, F> NamedTask for AsyncTask<T, I, F> 
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static 
{
    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }
    
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> StatusEnabledTask<T> for AsyncTask<T, I, F> 
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    fn status(&self) -> TaskStatus {
        u8_to_status(self.atomic_status.load(Ordering::Relaxed))
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> PrioritizedTask<T> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    fn priority(&self) -> &impl sweet_async_api::task::RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> TimedTask<T> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    fn created_timestamp(&self) -> SystemTime {
        self.created_time
    }
    
    fn executed_timestamp(&self) -> SystemTime {
        let nanos = self.atomic_start_time.load(Ordering::Relaxed);
        if nanos == 0 {
            self.created_time
        } else {
            UNIX_EPOCH + Duration::from_nanos(nanos)
        }
    }
    
    fn completed_timestamp(&self) -> SystemTime {
        let nanos = self.atomic_end_time.load(Ordering::Relaxed);
        if nanos == 0 {
            self.created_time
        } else {
            UNIX_EPOCH + Duration::from_nanos(nanos)
        }
    }
    
    fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> MetricsEnabledTask<T> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    type Cpu = TaskMetrics;
    type Memory = TaskMetrics;
    type Io = TaskMetrics;
    
    fn cpu_usage(&self) -> &Self::Cpu {
        &self.metrics
    }
    
    fn memory_usage(&self) -> &Self::Memory {
        &self.metrics
    }
    
    fn io_usage(&self) -> &Self::Io {
        &self.metrics
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> TracingTask<T> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        if self.tracing_enabled {
            tracing::error!("Task error: {:?}", error);
        }
        Err(error)
    }
    
    fn record_error(&self, error: &AsyncTaskError) {
        if self.tracing_enabled {
            tracing::error!("Recording error: {:?}", error);
        }
    }
    
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled
    }
}

impl<T: Clone + Send + Sync + 'static, I: TaskId, F> CancellableTask<T> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    async fn cancel(&self, _level: CancellationLevel) -> Result<(), OrchestratorError> {
        self.atomic_status.store(status_to_u8(&TaskStatus::PendingCancellation), Ordering::SeqCst);
        self.atomic_cancelled.store(true, Ordering::SeqCst);
        
        // Send cancellation message through relationships
        if let Some(parent) = &self.relationships.parent {
            // Send cancel acknowledgment to parent
            let _envelope = TaskEnvelope {
                sender_id: self.id,
                sender_hostname: hostname::get()
                    .ok()
                    .and_then(|name| name.into_string().ok())
                    .unwrap_or_else(|| "unknown".to_string()),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                message: ApiTaskMessage::CancelAck,
                is_encrypted: false,
                correlation_id: None,
            };
            // parent.sender.try_send(envelope).ok();
        }
        
        Ok(())
    }
    
    fn is_cancelled(&self) -> bool {
        self.atomic_cancelled.load(Ordering::Relaxed)
    }
    
    fn on_cancel<F2, Fut>(&self, _callback: F2)
    where
        F2: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // In channel-based design, cleanup happens via task chaining
    }
    
    async fn cancel_gracefully(&self) -> Result<(), OrchestratorError> {
        self.cancel(CancellationLevel::Graceful).await
    }
    
    async fn cancel_forcefully(&self) -> Result<(), OrchestratorError> {
        self.cancel(CancellationLevel::Kill).await
    }
    
    async fn cancel_immediately(&self) -> Result<(), OrchestratorError> {
        self.cancel(CancellationLevel::KillHard).await
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> ContextualizedTask<T, I> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    type RuntimeType = super::super::runtime::TokioRuntime;
    
    fn relationships(&self) -> &TaskRelationships<T, I> {
        &self.relationships
    }
    
    fn relationships_mut(&mut self) -> &mut TaskRelationships<T, I> {
        &mut self.relationships
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        panic!("Runtime should be accessed through orchestrator")
    }
    
    fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }
}

impl<T: Clone + Send + 'static, I: TaskId, F> RecoverableTask<T> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static + Clone
{
    type FallbackWork = F;
    
    async fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        if self.current_retry.load(Ordering::Relaxed) < self.max_retries {
            return Err(error); // Orchestrator handles retry
        }
        
        // Execute fallback
        let fallback = self.fallback_work.clone();
        fallback.run().await
    }
    
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        self.current_retry.load(Ordering::Relaxed) < self.max_retries
    }
    
    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.fallback_work
    }
    
    fn max_retries(&self) -> u8 {
        self.max_retries
    }
    
    fn current_retry(&self) -> u8 {
        self.current_retry.load(Ordering::Relaxed)
    }
    
    fn retry_strategy(&self) -> RetryStrategy {
        self.retry_strategy
    }
}

#[cfg(feature = "distributed")]
impl<T: Clone + Send + 'static, I: TaskId, F> DistributedTask<I> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    type VectorClock = crate::task::vector_clock::VectorClock<I>;
    
    fn vector_clock(&self) -> &Self::VectorClock {
        &self.vector_clock
    }
    
    fn tick_clock(&mut self) {
        let hostname = hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        self.vector_clock.tick(&self.id, &hostname);
    }
    
    fn update_clock_from(&mut self, other: &Self::VectorClock) {
        self.vector_clock.merge(other);
        self.tick_clock();
    }
}

#[cfg(not(feature = "distributed"))]
impl<T: Clone + Send + 'static, I: TaskId, F> DistributedTask<I> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static
{
    type VectorClock = VectorClock<I>;
    
    fn vector_clock(&self) -> &Self::VectorClock {
        &self.vector_clock
    }
    
    fn tick_clock(&mut self) {
        // No-op when distributed feature is disabled
    }
    
    fn update_clock_from(&mut self, _other: &Self::VectorClock) {
        // No-op when distributed feature is disabled
    }
}

impl<T: Clone + Send + Sync + 'static, I: TaskId, F> ApiAsyncTask<T, I> for AsyncTask<T, I, F>
where F: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync + 'static + Clone
{
    fn to<R: Clone + Send + 'static, Task: ApiAsyncTask<R, I> + 'static>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_spawning()
    }
    
    fn emits<R: Clone + Send + 'static, Task: ApiAsyncTask<R, I> + 'static>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_emitting()
    }
}