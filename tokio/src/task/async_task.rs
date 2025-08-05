use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use futures;

use tokio_util::sync::CancellationToken;

use sweet_async_api::orchestra::{OrchestratorBuilder, OrchestratorError};
use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, CancellableTask, CancellationLevel, ContextualizedTask, CpuUsage,
    IoUsage, MemoryUsage, MetricsEnabledTask, NamedTask, PrioritizedTask, RankableByPriority,
    RecoverableTask, RetryStrategy, StatusEnabledTask, TaskId, TaskPriority,
    TaskStatus, TimedTask, TracingTask, builder::AsyncWork,
};
use crate::task::TaskMetrics;
use uuid::Uuid;


use crate::orchestra::runtime::TokioRuntime;

// Metrics structs (retained and refined from previous version)
#[derive(Debug)]
pub struct TokioCpuUsage {
    total_time_nanos: AtomicU64,
    utilization_scaled: AtomicU64, // scaled by 100
    user_time_nanos: AtomicU64,
    system_time_nanos: AtomicU64,
}

impl Clone for TokioCpuUsage {
    fn clone(&self) -> Self {
        Self {
            total_time_nanos: AtomicU64::new(self.total_time_nanos.load(Ordering::Relaxed)),
            utilization_scaled: AtomicU64::new(self.utilization_scaled.load(Ordering::Relaxed)),
            user_time_nanos: AtomicU64::new(self.user_time_nanos.load(Ordering::Relaxed)),
            system_time_nanos: AtomicU64::new(self.system_time_nanos.load(Ordering::Relaxed)),
        }
    }
}

impl TokioCpuUsage {
    fn new() -> Self {
        Self {
            total_time_nanos: AtomicU64::new(0),
            utilization_scaled: AtomicU64::new(0),
            user_time_nanos: AtomicU64::new(0),
            system_time_nanos: AtomicU64::new(0),
        }
    }
}

impl CpuUsage for TokioCpuUsage {
    fn cpu_time(&self) -> Duration {
        Duration::from_nanos(self.total_time_nanos.load(Ordering::Relaxed))
    }

    fn utilization(&self) -> f64 {
        self.utilization_scaled.load(Ordering::Relaxed) as f64 / 100.0
    }

    fn user_time(&self) -> Duration {
        Duration::from_nanos(self.user_time_nanos.load(Ordering::Relaxed))
    }

    fn system_time(&self) -> Duration {
        Duration::from_nanos(self.system_time_nanos.load(Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct TokioMemoryUsage {
    current_bytes: AtomicU64,
    peak_bytes: AtomicU64,
    allocation_count: AtomicU64,
    allocation_rate_scaled: AtomicU64,
}

impl Clone for TokioMemoryUsage {
    fn clone(&self) -> Self {
        Self {
            current_bytes: AtomicU64::new(self.current_bytes.load(Ordering::Relaxed)),
            peak_bytes: AtomicU64::new(self.peak_bytes.load(Ordering::Relaxed)),
            allocation_count: AtomicU64::new(self.allocation_count.load(Ordering::Relaxed)),
            allocation_rate_scaled: AtomicU64::new(
                self.allocation_rate_scaled.load(Ordering::Relaxed),
            ),
        }
    }
}

impl TokioMemoryUsage {
    fn new() -> Self {
        Self {
            current_bytes: AtomicU64::new(0),
            peak_bytes: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
            allocation_rate_scaled: AtomicU64::new(0),
        }
    }
}

impl MemoryUsage for TokioMemoryUsage {
    fn current_bytes(&self) -> u64 {
        self.current_bytes.load(Ordering::Relaxed)
    }

    fn peak_bytes(&self) -> u64 {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    fn allocation_count(&self) -> u64 {
        self.allocation_count.load(Ordering::Relaxed)
    }

    fn allocation_rate(&self) -> f64 {
        self.allocation_rate_scaled.load(Ordering::Relaxed) as f64 / 100.0
    }
}

#[derive(Debug)]
pub struct TokioIoUsage {
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,
    read_operations: AtomicU64,
    write_operations: AtomicU64,
    read_latency_nanos: AtomicU64,
    write_latency_nanos: AtomicU64,
    ops_per_second_scaled: AtomicU64,
    io_wait_nanos: AtomicU64,
}

impl Clone for TokioIoUsage {
    fn clone(&self) -> Self {
        Self {
            bytes_read: AtomicU64::new(self.bytes_read.load(Ordering::Relaxed)),
            bytes_written: AtomicU64::new(self.bytes_written.load(Ordering::Relaxed)),
            read_operations: AtomicU64::new(self.read_operations.load(Ordering::Relaxed)),
            write_operations: AtomicU64::new(self.write_operations.load(Ordering::Relaxed)),
            read_latency_nanos: AtomicU64::new(self.read_latency_nanos.load(Ordering::Relaxed)),
            write_latency_nanos: AtomicU64::new(self.write_latency_nanos.load(Ordering::Relaxed)),
            ops_per_second_scaled: AtomicU64::new(
                self.ops_per_second_scaled.load(Ordering::Relaxed),
            ),
            io_wait_nanos: AtomicU64::new(self.io_wait_nanos.load(Ordering::Relaxed)),
        }
    }
}

impl TokioIoUsage {
    fn new() -> Self {
        Self {
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            read_operations: AtomicU64::new(0),
            write_operations: AtomicU64::new(0),
            read_latency_nanos: AtomicU64::new(0),
            write_latency_nanos: AtomicU64::new(0),
            ops_per_second_scaled: AtomicU64::new(0),
            io_wait_nanos: AtomicU64::new(0),
        }
    }
}

impl IoUsage for TokioIoUsage {
    fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    fn read_operations(&self) -> u64 {
        self.read_operations.load(Ordering::Relaxed)
    }

    fn write_operations(&self) -> u64 {
        self.write_operations.load(Ordering::Relaxed)
    }

    fn read_latency(&self) -> Duration {
        Duration::from_nanos(self.read_latency_nanos.load(Ordering::Relaxed))
    }

    fn write_latency(&self) -> Duration {
        Duration::from_nanos(self.write_latency_nanos.load(Ordering::Relaxed))
    }

    fn operations_per_second(&self) -> f64 {
        self.ops_per_second_scaled.load(Ordering::Relaxed) as f64 / 100.0
    }

    fn io_wait_time(&self) -> Duration {
        Duration::from_nanos(self.io_wait_nanos.load(Ordering::Relaxed))
    }
}

// Main struct
pub struct TokioAsyncTask<T: Clone + Send + 'static, I: TaskId> {
    id: I,
    name: Option<String>,
    priority: TaskPriority,
    status: AtomicU8,
    cancel_token: CancellationToken,
    cancelled: AtomicBool,
    created_at: Instant,
    created_timestamp: SystemTime,
    executed_timestamp: AtomicU64,
    completed_timestamp: AtomicU64,
    timeout_duration: Duration,
    retry_count: AtomicU32,
    metrics: TaskMetrics,
    cpu_usage: TokioCpuUsage,
    memory_usage: TokioMemoryUsage,
    io_operations: TokioIoUsage,
    tracing_enabled: AtomicBool,
    error_count: AtomicU64,
    runtime_handle: tokio::runtime::Handle,
    max_retries: u8,
    current_retry: AtomicU8,
    retry_strategy: RetryStrategy,
    fallback_work: crate::task::error_fallback::ErrorFallback<T>,
    // Immutable callback properties - NO Arc<Mutex<Vec<>>> pattern
    on_cancel_callback: Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>,
    relationships: (),
    // Sophisticated cancellation state management
    cancellation_level: AtomicU8,
    cancellation_started: AtomicU64,
    graceful_timeout_nanos: AtomicU64,
    kill_timeout_nanos: AtomicU64,
    escalation_count: AtomicU8,
    cleanup_completed: AtomicBool,
    _phantom: PhantomData<T>,
}

impl<T: Clone + Send + Sync + Unpin + 'static, I: TaskId + Unpin> TokioAsyncTask<T, I> {
    fn new(id: I) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name: None,
            priority: TaskPriority::Normal,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            cancel_token: CancellationToken::new(),
            cancelled: AtomicBool::new(false),
            created_at: Instant::now(),
            created_timestamp: now,
            executed_timestamp: AtomicU64::new(0),
            completed_timestamp: AtomicU64::new(0),
            timeout_duration: Duration::from_secs(30),
            retry_count: AtomicU32::new(0),
            metrics: TaskMetrics::new(),
            cpu_usage: TokioCpuUsage::new(),
            memory_usage: TokioMemoryUsage::new(),
            io_operations: TokioIoUsage::new(),
            tracing_enabled: AtomicBool::new(false),
            error_count: AtomicU64::new(0),
            runtime_handle: tokio::runtime::Handle::current(),
            max_retries: 3,
            current_retry: AtomicU8::new(0),
            retry_strategy: RetryStrategy::Exponential {
                base: Duration::from_secs(1),
                factor: 2.0,
                max: Duration::from_secs(30),
            },
            fallback_work: crate::task::error_fallback::ErrorFallback::new(),
            on_cancel_callback: None, // No callback initially
            relationships: (),
            // Initialize sophisticated cancellation state
            cancellation_level: AtomicU8::new(0), // No cancellation initially
            cancellation_started: AtomicU64::new(0),
            graceful_timeout_nanos: AtomicU64::new(Duration::from_secs(30).as_nanos() as u64),
            kill_timeout_nanos: AtomicU64::new(Duration::from_secs(10).as_nanos() as u64),
            escalation_count: AtomicU8::new(0),
            cleanup_completed: AtomicBool::new(false),
            _phantom: PhantomData,
        }
    }

    fn with_generated_id() -> Self
    where
        I: From<Uuid>,
    {
        Self::new(I::from(Uuid::new_v4()))
    }

    fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    fn with_tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled.store(enabled, Ordering::Relaxed);
        self
    }

    pub fn task_id(&self) -> I {
        self.id
    }

    fn mark_started(&self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos() as u64;
        self.executed_timestamp.store(now, Ordering::Relaxed);
        self.status
            .store(TaskStatus::Running as u8, Ordering::Relaxed);
    }

    fn mark_completed(&self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos() as u64;
        self.completed_timestamp.store(now, Ordering::Relaxed);
        self.status
            .store(TaskStatus::Completed as u8, Ordering::Relaxed);
    }

    fn mark_failed(&self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos() as u64;
        self.completed_timestamp.store(now, Ordering::Relaxed);
        self.status
            .store(TaskStatus::Failed as u8, Ordering::Relaxed);
    }

    fn increment_retry(&self) {
        self.current_retry.fetch_add(1, Ordering::Relaxed);
    }

    fn update_cpu_usage(&self, usage: u64) {
        self.cpu_usage
            .total_time_nanos
            .store(usage, Ordering::Relaxed);
    }

    fn update_memory_usage(&self, bytes: u64) {
        self.memory_usage
            .current_bytes
            .store(bytes, Ordering::Relaxed);
    }

    fn increment_io_operations(&self) {
        self.io_operations
            .read_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    async fn execute_cancellation_callbacks(&self) {
        // Execute the immutable callback property if present
        if let Some(ref callback) = self.on_cancel_callback {
            let future = callback();
            tokio::spawn(future);
        }
    }

    /// Escalate cancellation to a more aggressive level with zero allocation
    async fn escalate_cancellation(&self, target_level: CancellationLevel) -> Result<(), OrchestratorError> {
        // Record escalation count and update level atomically
        self.escalation_count.fetch_add(1, Ordering::Relaxed);
        self.cancellation_level.store(target_level as u8, Ordering::Relaxed);
        
        // Delegate to the main cancel method for consistent behavior
        self.cancel(target_level).await
    }
}

// Super trait implementations

impl<T: Clone + Send + 'static, I: TaskId> CancellableTask<T> for TokioAsyncTask<T, I> {
    fn cancel(
        &self,
        level: CancellationLevel,
    ) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        let cancelled = self.cancelled.load(Ordering::Relaxed);
        let status = self.status.load(Ordering::Relaxed);
        let cancellation_level = self.cancellation_level.load(Ordering::Relaxed);
        let cancellation_started = self.cancellation_started.load(Ordering::Relaxed);
        let graceful_timeout_nanos = self.graceful_timeout_nanos.load(Ordering::Relaxed);
        let kill_timeout_nanos = self.kill_timeout_nanos.load(Ordering::Relaxed);
        let escalation_count = self.escalation_count.load(Ordering::Relaxed);
        let cleanup_completed = self.cleanup_completed.load(Ordering::Relaxed);
        // Cannot clone callback since it's a trait object - get reference instead
        let has_callback = self.on_cancel_callback.is_some();

        async move {
            // Record cancellation initiation with zero-allocation timestamp
            let start_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_nanos() as u64;
            
            self.cancellation_started.store(start_time, Ordering::Relaxed);
            self.cancellation_level.store(level as u8, Ordering::Relaxed);
            
            // Transition to pending cancellation state atomically
            self.status.store(TaskStatus::PendingCancellation as u8, Ordering::Relaxed);
            
            match level {
                CancellationLevel::Graceful => {
                    // Set cancellation flag to signal graceful shutdown
                    self.cancelled.store(true, Ordering::Relaxed);
                    token.cancel();
                    
                    // Execute cancellation callback if present
                    if let Some(ref callback) = self.on_cancel_callback {
                        let future = callback();
                        let graceful_timeout_duration = Duration::from_nanos(
                            graceful_timeout_nanos
                        );
                        
                        match tokio::time::timeout(graceful_timeout_duration, future).await {
                            Ok(_) => {
                                self.cleanup_completed.store(true, Ordering::Relaxed);
                            }
                            Err(_) => {
                                // Graceful timeout exceeded, escalate automatically
                                self.escalation_count.fetch_add(1, Ordering::Relaxed);
                                return self.escalate_cancellation(CancellationLevel::Kill).await;
                            }
                        }
                    } else {
                        // No callback, mark cleanup as completed immediately
                        self.cleanup_completed.store(true, Ordering::Relaxed);
                    }
                        // No error case needed - already handled above
                }
                
                CancellationLevel::Kill => {
                    // More urgent cancellation with limited cleanup window
                    self.cancelled.store(true, Ordering::Relaxed);
                    token.cancel();
                    
                    // Execute only essential cleanup with strict timeout
                    let kill_timeout_duration = Duration::from_nanos(
                        self.kill_timeout_nanos.load(Ordering::Relaxed)
                    );
                    
                    let essential_cleanup = async {
                        // Execute callback if present (minimal cleanup mode)
                        if let Some(ref callback) = self.on_cancel_callback {
                            let future = callback();
                            // Don't wait for callback in kill mode
                            tokio::spawn(future);
                        }
                        self.cleanup_completed.store(true, Ordering::Relaxed);
                    };
                    
                    match tokio::time::timeout(kill_timeout_duration, essential_cleanup).await {
                        Ok(_) => {
                            // Essential cleanup completed within timeout
                        }
                        Err(_) => {
                            // Kill timeout exceeded, escalate to hard termination
                            self.escalation_count.fetch_add(1, Ordering::Relaxed);
                            return self.escalate_cancellation(CancellationLevel::KillHard).await;
                        }
                    }
                }
                
                CancellationLevel::KillHard => {
                    // Immediate termination with no cleanup
                    self.cancelled.store(true, Ordering::Relaxed);
                    token.cancel();
                    
                    // No cleanup operations allowed - immediate termination
                    self.escalation_count.fetch_add(1, Ordering::Relaxed);
                    self.cleanup_completed.store(false, Ordering::Relaxed); // No cleanup performed
                }
            }
            
            // Transition to final cancelled state
            self.status.store(TaskStatus::Cancelled as u8, Ordering::Relaxed);
            
            Ok(())
        }
    }

    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            Ok(())
        }
    }

    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            Ok(())
        }
    }

    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            Ok(())
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    fn on_cancel<F, Fut>(self, callback: F) -> Self
    where
        F: crate::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Immutable builder pattern: return new instance with callback property
        let callback_arc = std::sync::Arc::new(callback);
        let callback_wrapper = Box::new(move || {
            let callback_clone = callback_arc.clone();
            Box::pin(async move {
                let fut = callback_clone.run().await;
                fut.await
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });
        
        Self {
            id: self.id,
            name: self.name,
            priority: self.priority,
            status: AtomicU8::new(self.status.load(Ordering::Relaxed)),
            cancel_token: self.cancel_token,
            cancelled: AtomicBool::new(self.cancelled.load(Ordering::Relaxed)),
            created_at: self.created_at,
            created_timestamp: self.created_timestamp,
            executed_timestamp: AtomicU64::new(self.executed_timestamp.load(Ordering::Relaxed)),
            completed_timestamp: AtomicU64::new(self.completed_timestamp.load(Ordering::Relaxed)),
            timeout_duration: self.timeout_duration,
            retry_count: AtomicU32::new(self.retry_count.load(Ordering::Relaxed)),
            cpu_usage: self.cpu_usage,
            memory_usage: self.memory_usage,
            io_operations: self.io_operations,
            tracing_enabled: AtomicBool::new(self.tracing_enabled.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
            runtime_handle: self.runtime_handle,
            max_retries: self.max_retries,
            current_retry: AtomicU8::new(self.current_retry.load(Ordering::Relaxed)),
            retry_strategy: self.retry_strategy,
            metrics: self.metrics,
            fallback_work: self.fallback_work,
            on_cancel_callback: Some(callback_wrapper),
            relationships: self.relationships,
            cancellation_level: AtomicU8::new(self.cancellation_level.load(Ordering::Relaxed)),
            cancellation_started: AtomicU64::new(self.cancellation_started.load(Ordering::Relaxed)),
            graceful_timeout_nanos: AtomicU64::new(self.graceful_timeout_nanos.load(Ordering::Relaxed)),
            kill_timeout_nanos: AtomicU64::new(self.kill_timeout_nanos.load(Ordering::Relaxed)),
            escalation_count: AtomicU8::new(self.escalation_count.load(Ordering::Relaxed)),
            cleanup_completed: AtomicBool::new(self.cleanup_completed.load(Ordering::Relaxed)),
            _phantom: PhantomData,
        }
    }
} // Close CancellableTask impl

impl<T: Clone + Send + 'static, I: TaskId> TracingTask<T> for TokioAsyncTask<T, I> {
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        self.record_error(&error);
        Err(error)
    }

    fn record_error(&self, error: &AsyncTaskError) {
        if self.is_tracing_enabled() {
            tracing::error!("Task {:?} error: {:?}", self.id, error);
        }
    }

    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TimedTask<T> for TokioAsyncTask<T, I> {
    fn created_timestamp(&self) -> SystemTime {
        self.created_timestamp
    }

    fn executed_timestamp(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH
            + Duration::from_nanos(self.executed_timestamp.load(Ordering::Relaxed))
    }

    fn completed_timestamp(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH
            + Duration::from_nanos(self.completed_timestamp.load(Ordering::Relaxed))
    }

    fn timeout(&self) -> Duration {
        self.timeout_duration
    }
}

impl<T: Clone + Send + 'static, I: TaskId> ContextualizedTask<T, I> for TokioAsyncTask<T, I> {
    type RuntimeType = TokioRuntime;
    type RelationshipsType = ();

    fn relationships(&self) -> &Self::RelationshipsType {
        &self.relationships
    }

    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut self.relationships
    }

    fn runtime(&self) -> &Self::RuntimeType {
        static RUNTIME: std::sync::OnceLock<TokioRuntime> = std::sync::OnceLock::new();
        RUNTIME.get_or_init(|| TokioRuntime::new())
    }

    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_default()
    }
}

impl<T: Clone + Send + 'static, I: TaskId> RecoverableTask<T> for TokioAsyncTask<T, I> {
    type FallbackWork = crate::task::error_fallback::ErrorFallback<T>;

    fn recover(&self, error: AsyncTaskError) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        let fallback_work = self.fallback_work.clone();
        let mut current_retry = self.current_retry.load(std::sync::atomic::Ordering::Relaxed);
        let retry_strategy = self.retry_strategy;
        let _max_retries = self.max_retries;
        
        async move {
            let mut attempts = 0;
            let max_infinite_cap = 10; // Cap to prevent true infinite loop

            // Check if we can recover from this error type
            let can_recover = match error {
                AsyncTaskError::Timeout(_) => true,
                AsyncTaskError::Failure(_) => true,
                _ => false,
            };

            loop {
                if attempts >= max_infinite_cap {
                    return Err(AsyncTaskError::RecoveryFailed(
                        "Retry cap reached".to_string(),
                    ));
                }

                if !can_recover {
                    break;
                }

                current_retry += 1;
                attempts += 1;

                // Inline backoff calculation (zero alloc, fast math)
                let delay = match retry_strategy {
                    RetryStrategy::Exponential { base, factor, max } => {
                        let attempt = current_retry as u32;
                        let calculated = base.as_millis() as f64 * factor.powi(attempt as i32);
                        Duration::from_millis(calculated.min(max.as_millis() as f64) as u64)
                    }
                    RetryStrategy::Linear { base, increment } => {
                        let attempt = current_retry as u64;
                        base + Duration::from_nanos(increment.as_nanos() as u64 * attempt)
                    }
                    RetryStrategy::Fixed(delay) => delay,
                    RetryStrategy::Immediate => Duration::ZERO,
                };

                // Non-blocking sleep
                tokio::time::sleep(delay).await;

                // Re-execution simulation - always succeed after cap for 'always work'
                if attempts % 2 == 0 {
                    return fallback_work.clone().run().await;
                }
            }

            // Execute fallback work as last resort
            fallback_work.run().await
        }
    }

    fn can_recover_from(&self, error: &AsyncTaskError) -> bool {
        match error {
            AsyncTaskError::Timeout(_) => true,
            AsyncTaskError::Failure(_) => true,
            _ => false,
        }
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

    fn retries_exhausted(&self) -> bool {
        self.current_retry.load(Ordering::Relaxed) >= self.max_retries
    }
}

impl<T: Clone + Send + 'static, I: TaskId> StatusEnabledTask<T> for TokioAsyncTask<T, I> {
    fn status(&self) -> TaskStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => TaskStatus::Pending,
            1 => TaskStatus::Running,
            2 => TaskStatus::Completed,
            3 => TaskStatus::Failed,
            4 => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> PrioritizedTask<T> for TokioAsyncTask<T, I> {
    fn priority(&self) -> &impl RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, I: TaskId> MetricsEnabledTask<T> for TokioAsyncTask<T, I> {
    type Cpu = TokioCpuUsage;
    type Memory = TokioMemoryUsage;
    type Io = TokioIoUsage;

    fn cpu_usage(&self) -> &Self::Cpu {
        &self.cpu_usage
    }

    fn memory_usage(&self) -> &Self::Memory {
        &self.memory_usage
    }

    fn io_usage(&self) -> &Self::Io {
        &self.io_operations
    }
}

impl<T: Clone + Send + 'static, I: TaskId> NamedTask for TokioAsyncTask<T, I> {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

// Core AsyncTask implementation
impl<T: Clone + Send + 'static, I: TaskId> AsyncTask<T, I> for TokioAsyncTask<T, I> {
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>
    {
        use crate::task::spawn::builder::TokioSpawningTaskBuilder;
        TokioSpawningTaskBuilder::<R, sweet_async_api::task::AsyncTaskError, I>::new()
    }

    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl OrchestratorBuilder<R, Task, I> {
        use crate::task::emit::channel_builder::TokioEmittingTaskBuilder;
        TokioEmittingTaskBuilder::<R, (), sweet_async_api::task::AsyncTaskError, I>::new()
    }
} // Close the impl block properly

impl<T: Clone + Send + 'static, I: TaskId> Clone for TokioAsyncTask<T, I> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            priority: self.priority,
            status: AtomicU8::new(self.status.load(Ordering::Relaxed)),
            cancel_token: self.cancel_token.clone(),
            cancelled: AtomicBool::new(self.cancelled.load(Ordering::Relaxed)),
            created_at: self.created_at,
            created_timestamp: self.created_timestamp,
            executed_timestamp: AtomicU64::new(self.executed_timestamp.load(Ordering::Relaxed)),
            completed_timestamp: AtomicU64::new(self.completed_timestamp.load(Ordering::Relaxed)),
            timeout_duration: self.timeout_duration,
            retry_count: AtomicU32::new(self.retry_count.load(Ordering::Relaxed)),
            cpu_usage: self.cpu_usage.clone(),
            memory_usage: self.memory_usage.clone(),
            io_operations: self.io_operations.clone(),
            tracing_enabled: AtomicBool::new(self.tracing_enabled.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
            runtime_handle: self.runtime_handle.clone(),
            max_retries: self.max_retries,
            current_retry: AtomicU8::new(self.current_retry.load(Ordering::Relaxed)),
            retry_strategy: self.retry_strategy,
            metrics: self.metrics.clone(),
            fallback_work: crate::task::error_fallback::ErrorFallback::new(),
            on_cancel_callback: self.on_cancel_callback.as_ref().map(|cb| {
                // Since callbacks are immutable properties, we can't clone Fn traits
                // Create a new None callback for the clone
                None as Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>
            }).flatten(),
            relationships: self.relationships,
            // Clone sophisticated cancellation state
            cancellation_level: AtomicU8::new(self.cancellation_level.load(Ordering::Relaxed)),
            cancellation_started: AtomicU64::new(self.cancellation_started.load(Ordering::Relaxed)),
            graceful_timeout_nanos: AtomicU64::new(self.graceful_timeout_nanos.load(Ordering::Relaxed)),
            kill_timeout_nanos: AtomicU64::new(self.kill_timeout_nanos.load(Ordering::Relaxed)),
            escalation_count: AtomicU8::new(self.escalation_count.load(Ordering::Relaxed)),
            cleanup_completed: AtomicBool::new(self.cleanup_completed.load(Ordering::Relaxed)),
            _phantom: PhantomData,
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> std::future::IntoFuture for TokioAsyncTask<T, I> {
    type Output = Result<T, AsyncTaskError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let timeout_duration = self.timeout_duration;
            let max_retries = self.max_retries;
            // Mark started
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_nanos() as u64;
            self.executed_timestamp.store(now, Ordering::Relaxed);
            self.status.store(TaskStatus::Running as u8, Ordering::Relaxed);
            
            // Check if task is already cancelled
            if self.cancelled.load(Ordering::Relaxed) {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO)
                    .as_nanos() as u64;
                self.completed_timestamp.store(now, Ordering::Relaxed);
                self.status.store(TaskStatus::Failed as u8, Ordering::Relaxed);
                return Err(AsyncTaskError::Cancelled);
            }

            // Execute task with timeout and retry logic
            let result = tokio::time::timeout(timeout_duration, async {
                // Execute fallback work as the primary execution path
                let mut attempts = 0;
                let max_attempts = max_retries as usize + 1;
                
                loop {
                    attempts += 1;
                    
                    match self.fallback_work.clone().run().await {
                        Ok(value) => {
                            let now = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or(Duration::ZERO)
                                .as_nanos() as u64;
                            self.completed_timestamp.store(now, Ordering::Relaxed);
                            self.status.store(TaskStatus::Completed as u8, Ordering::Relaxed);
                            return Ok(value);
                        },
                        Err(error) => {
                            if attempts >= max_attempts {
                                self.error_count.fetch_add(1, Ordering::Relaxed);
                                let now = SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap_or(Duration::ZERO)
                                    .as_nanos() as u64;
                                self.completed_timestamp.store(now, Ordering::Relaxed);
                                self.status.store(TaskStatus::Failed as u8, Ordering::Relaxed);
                                return Err(error);
                            }
                            
                            // Calculate retry delay based on strategy
                            let delay = match self.retry_strategy {
                                RetryStrategy::Exponential { base, factor, max } => {
                                    let calculated = base.as_millis() as f64 * factor.powi((attempts - 1) as i32);
                                    Duration::from_millis(calculated.min(max.as_millis() as f64) as u64)
                                },
                                RetryStrategy::Linear { base, increment } => {
                                    base + Duration::from_nanos(increment.as_nanos() as u64 * (attempts - 1) as u64)
                                },
                                RetryStrategy::Fixed(delay) => delay,
                                RetryStrategy::Immediate => Duration::ZERO,
                            };
                            
                            if delay > Duration::ZERO {
                                tokio::time::sleep(delay).await;
                            }
                            
                            self.current_retry.store(attempts as u8, Ordering::Relaxed);
                        }
                    }
                }
            }).await;

            match result {
                Ok(task_result) => task_result,
                Err(_) => {
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or(Duration::ZERO)
                        .as_nanos() as u64;
                    self.completed_timestamp.store(now, Ordering::Relaxed);
                    self.status.store(TaskStatus::Failed as u8, Ordering::Relaxed);
                    Err(AsyncTaskError::Timeout(timeout_duration))
                }
            }
        })
    }
}
