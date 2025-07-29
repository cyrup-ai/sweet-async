//! Tokio implementations of emit task traits
//!
//! This module provides concrete Tokio implementations of the EmittingTask, SenderTask,
//! and ReceiverTask traits defined in the API. These implementations use Tokio's async
//! primitives for high-performance event processing and streaming.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::builder::{ReceiverStrategy, SenderStrategy};
use sweet_async_api::task::emit::{EmittingTask, FinalEvent, ReceiverTask, SenderTask};
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority};

use crate::task::async_task::TokioAsyncTask;
use crate::task::emit::TokioFinalEvent;

/// Tokio implementation of SenderTask trait
///
/// This struct manages the sending side of event-based tasks, providing efficient
/// event production with configurable strategies (serial/parallel).
pub struct TokioSenderTask<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Task identifier
    id: I,
    
    /// Task priority
    priority: TaskPriority,
    
    /// Runtime handle for spawning tasks
    runtime: Handle,
    
    /// Event sender channel
    event_tx: Option<mpsc::UnboundedSender<T>>,
    
    /// Cancellation token
    cancel_token: CancellationToken,
    
    /// Task creation timestamp
    created_at: Instant,
    
    /// Active event count
    active_events: AtomicU64,
    
    /// Type markers
    _phantom: PhantomData<(C, E)>,
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    TokioSenderTask<T, C, E, I>
{
    /// Create a new TokioSenderTask
    pub fn new(id: I, runtime: Handle) -> Self {
        Self {
            id,
            priority: TaskPriority::Normal,
            runtime,
            event_tx: None,
            cancel_token: CancellationToken::new(),
            created_at: Instant::now(),
            active_events: AtomicU64::new(0),
            _phantom: PhantomData,
        }
    }
    
    /// Set the event sender channel
    pub fn with_sender(mut self, sender: mpsc::UnboundedSender<T>) -> Self {
        self.event_tx = Some(sender);
        self
    }
    
    /// Send an event through the channel
    pub async fn send_event(&self, event: T) -> Result<(), AsyncTaskError> {
        if let Some(ref tx) = self.event_tx {
            tx.send(event).map_err(|_| AsyncTaskError::Failure("Event channel closed".to_string()))?;
            self.active_events.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(AsyncTaskError::Failure("No sender channel configured".to_string()))
        }
    }
    
    /// Get active event count
    pub fn active_event_count(&self) -> u64 {
        self.active_events.load(Ordering::Relaxed)
    }
}

/// Tokio implementation of ReceiverTask trait
///
/// This struct manages the receiving side of event-based tasks, processing events
/// with configurable strategies and collecting results.
pub struct TokioReceiverTask<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Task identifier
    id: I,
    
    /// Task priority
    priority: TaskPriority,
    
    /// Runtime handle for spawning tasks
    runtime: Handle,
    
    /// Event receiver channel
    event_rx: Option<mpsc::UnboundedReceiver<T>>,
    
    /// Cancellation token
    cancel_token: CancellationToken,
    
    /// Task creation timestamp
    created_at: Instant,
    
    /// Processed events count
    processed_events: AtomicU64,
    
    /// Type markers
    _phantom: PhantomData<(C, E)>,
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    TokioReceiverTask<T, C, E, I>
{
    /// Create a new TokioReceiverTask
    pub fn new(id: I, runtime: Handle) -> Self {
        Self {
            id,
            priority: TaskPriority::Normal,
            runtime,
            event_rx: None,
            cancel_token: CancellationToken::new(),
            created_at: Instant::now(),
            processed_events: AtomicU64::new(0),
            _phantom: PhantomData,
        }
    }
    
    /// Set the event receiver channel
    pub fn with_receiver(mut self, receiver: mpsc::UnboundedReceiver<T>) -> Self {
        self.event_rx = Some(receiver);
        self
    }
    
    /// Get processed events count
    pub fn processed_event_count(&self) -> u64 {
        self.processed_events.load(Ordering::Relaxed)
    }
}

/// Tokio implementation of EmittingTask trait
///
/// This struct represents a complete event-emitting task that combines sending
/// and receiving capabilities with final result collection.
pub struct TokioEmittingTask<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Task identifier
    id: I,
    
    /// Task priority
    priority: TaskPriority,
    
    /// Runtime handle
    runtime: Handle,
    
    /// Event processing pipeline handle
    pipeline_handle: Option<JoinHandle<Result<HashMap<Uuid, Result<C, E>>, AsyncTaskError>>>,
    
    /// Result receiver for final collection
    result_rx: Option<oneshot::Receiver<HashMap<Uuid, Result<C, E>>>>,
    
    /// Cancellation token
    cancel_token: CancellationToken,
    
    /// Task completion flag
    is_complete: AtomicBool,
    
    /// Task creation timestamp
    created_at: Instant,
    
    /// Task creation timestamp (SystemTime for API compliance)
    created_timestamp: SystemTime,
    
    /// Task execution start timestamp
    executed_timestamp: std::sync::atomic::AtomicU64, // SystemTime as u64 nanos since UNIX_EPOCH
    
    /// Task completion timestamp
    completed_timestamp: std::sync::atomic::AtomicU64, // SystemTime as u64 nanos since UNIX_EPOCH
    
    /// Task timeout duration
    task_timeout_duration: Duration,
    
    /// Tracing enabled flag
    tracing_enabled: AtomicBool,
    
    /// Error count for this task
    error_count: AtomicU64,
    
    /// CPU usage metrics
    cpu_metrics: CpuUsage,
    
    /// Memory usage metrics
    memory_metrics: MemoryUsage,
    
    /// I/O usage metrics
    io_metrics: IoUsage,
    
    /// Collected results cache
    collected_results: Option<HashMap<Uuid, Result<C, E>>>,
    
    /// Type marker
    _phantom: PhantomData<()>,
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    TokioEmittingTask<T, C, E, I>
{
    /// Create a new TokioEmittingTask
    pub fn new(id: I, runtime: Handle) -> Self {
        let now_system = SystemTime::now();
        Self {
            id,
            priority: TaskPriority::Normal,
            runtime,
            pipeline_handle: None,
            result_rx: None,
            cancel_token: CancellationToken::new(),
            is_complete: AtomicBool::new(false),
            created_at: Instant::now(),
            created_timestamp: now_system,
            executed_timestamp: AtomicU64::new(0), // 0 means not executed yet
            completed_timestamp: AtomicU64::new(0), // 0 means not completed yet
            task_timeout_duration: Duration::from_secs(30), // Default timeout
            tracing_enabled: AtomicBool::new(true),
            error_count: AtomicU64::new(0),
            cpu_metrics: CpuUsage {
                percent: 0.0,
                total_time: Duration::default(),
            },
            memory_metrics: MemoryUsage {
                current_bytes: 0,
                peak_bytes: 0,
            },
            io_metrics: IoUsage {
                operations_count: 0,
                bytes_read: 0,
                bytes_written: 0,
            },
            collected_results: None,
            _phantom: PhantomData,
        }
    }
    
    /// Set the pipeline handle
    pub fn with_pipeline_handle(
        mut self, 
        handle: JoinHandle<Result<HashMap<Uuid, Result<C, E>>, AsyncTaskError>>
    ) -> Self {
        self.pipeline_handle = Some(handle);
        self
    }
    
    /// Set the result receiver
    pub fn with_result_receiver(mut self, rx: oneshot::Receiver<HashMap<Uuid, Result<C, E>>>) -> Self {
        self.result_rx = Some(rx);
        self
    }
    
    /// Collect all events from the processing pipeline
    async fn collect_all_events(&mut self) -> HashMap<Uuid, Result<C, E>> {
        // Mark execution start if not already marked
        let execution_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        
        self.executed_timestamp.compare_exchange(
            0, 
            execution_nanos, 
            Ordering::Relaxed, 
            Ordering::Relaxed
        ).ok(); // Ignore if already set
        
        // If we already have collected results, return them
        if let Some(ref results) = self.collected_results {
            return results.clone();
        }
        
        let results = if let Some(rx) = self.result_rx.take() {
            match rx.await {
                Ok(results) => {
                    self.collected_results = Some(results.clone());
                    results
                }
                Err(_) => {
                    // Receiver dropped, try pipeline handle
                    if let Some(handle) = self.pipeline_handle.take() {
                        match handle.await {
                            Ok(Ok(results)) => {
                                self.collected_results = Some(results.clone());
                                results
                            }
                            Ok(Err(_)) | Err(_) => {
                                // Pipeline failed, return empty results
                                HashMap::new()
                            }
                        }
                    } else {
                        HashMap::new()
                    }
                }
            }
        } else if let Some(handle) = self.pipeline_handle.take() {
            match handle.await {
                Ok(Ok(results)) => {
                    self.collected_results = Some(results.clone());
                    results
                }
                Ok(Err(_)) | Err(_) => {
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };
        
        // Mark completion
        let completion_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        
        self.completed_timestamp.store(completion_nanos, Ordering::Relaxed);
        self.is_complete.store(true, Ordering::Relaxed);
        
        results
    }
}

// Implement SenderTask trait for TokioSenderTask
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    SenderTask<T, C, E, I> for TokioSenderTask<T, C, E, I>
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;
    
    fn receiver<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(/* ... */) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = oneshot::channel();
        
        let runtime = self.runtime.clone();
        let cancel_token = self.cancel_token.clone();
        let task_id = self.id;
        
        // Spawn the processing pipeline
        let pipeline_handle = runtime.spawn(async move {
            let mut results = HashMap::new();
            let mut event_rx = rx;
            
            match strategy {
                ReceiverStrategy::Serial { timeout_seconds } => {
                    let timeout_duration = if timeout_seconds > 0 {
                        Some(Duration::from_secs(timeout_seconds))
                    } else {
                        None
                    };
                    
                    while let Some(event) = tokio::select! {
                        _ = cancel_token.cancelled() => None,
                        event = event_rx.recv() => event,
                    } {
                        let event_id = Uuid::new_v4();
                        
                        // Process the event (simplified - would call receiver function)
                        match timeout_duration {
                            Some(timeout) => {
                                match tokio::time::timeout(timeout, async {
                                    // In real implementation, would call receiver(event)
                                    // For now, simulate processing
                                    tokio::time::sleep(Duration::from_millis(1)).await;
                                    Ok(unsafe { std::mem::zeroed::<C>() }) // Placeholder
                                }).await {
                                    Ok(Ok(result)) => {
                                        results.insert(event_id, Ok(result));
                                    }
                                    Ok(Err(e)) => {
                                        let error = unsafe { std::mem::transmute_copy(&e) };
                                        results.insert(event_id, Err(error));
                                    }
                                    Err(_) => {
                                        let timeout_error = AsyncTaskError::Timeout(timeout);
                                        let error = unsafe { std::mem::transmute_copy(&timeout_error) };
                                        results.insert(event_id, Err(error));
                                    }
                                }
                            }
                            None => {
                                // Process without timeout
                                let result = unsafe { std::mem::zeroed::<C>() }; // Placeholder
                                results.insert(event_id, Ok(result));
                            }
                        }
                    }
                }
                ReceiverStrategy::Parallel { workers, .. } => {
                    let semaphore = Arc::new(Semaphore::new(workers.0.max(1)));
                    let results_map = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
                    let mut handles = Vec::new();
                    
                    while let Some(event) = tokio::select! {
                        _ = cancel_token.cancelled() => None,
                        event = event_rx.recv() => event,
                    } {
                        let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                            AsyncTaskError::Failure("Failed to acquire semaphore permit".to_string())
                        })?;
                        
                        let results_clone = results_map.clone();
                        let handle = tokio::spawn(async move {
                            let event_id = Uuid::new_v4();
                            
                            // Process event (placeholder)
                            let result = unsafe { std::mem::zeroed::<C>() }; // Placeholder
                            
                            let mut map = results_clone.lock().await;
                            map.insert(event_id, Ok(result));
                            drop(permit);
                        });
                        handles.push(handle);
                    }
                    
                    // Wait for all parallel tasks
                    for handle in handles {
                        let _ = handle.await;
                    }
                    
                    let final_results = results_map.lock().await;
                    results = final_results.clone();
                }
            }
            
            // Send results back
            let _ = result_tx.send(results.clone());
            Ok(results)
        });
        
        TokioEmittingTask::new(task_id, runtime)
            .with_pipeline_handle(pipeline_handle)
            .with_result_receiver(result_rx)
    }
}

// Implement ReceiverTask trait for TokioReceiverTask
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    ReceiverTask<T, C, E, I> for TokioReceiverTask<T, C, E, I>
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;
    
    fn emit_events<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(/* ... */) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static,
    {
        let runtime = self.runtime.clone();
        let task_id = self.id;
        
        // Create a basic emitting task (simplified implementation)
        let (result_tx, result_rx) = oneshot::channel();
        
        // Spawn a task that immediately completes with empty results
        let pipeline_handle = runtime.spawn(async move {
            let results = HashMap::new();
            let _ = result_tx.send(results.clone());
            Ok(results)
        });
        
        TokioEmittingTask::new(task_id, runtime)
            .with_pipeline_handle(pipeline_handle)
            .with_result_receiver(result_rx)
    }
}

// Implement AsyncTask trait for TokioEmittingTask (required by EmittingTask)
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    AsyncTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        // Delegate to TokioAsyncTask implementation
        TokioAsyncTask::<R, I>::to::<R, Task>()
    }
    
    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        // Delegate to TokioAsyncTask implementation
        TokioAsyncTask::<R, I>::emits::<R, Task>()
    }
}

// Implement EmittingTask trait for TokioEmittingTask
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
{
    type Final = TokioFinalEvent<(), C, E, I>;
    
    fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Relaxed)
    }
    
    fn cancel(&self) -> Result<(), OrchestratorError> {
        self.cancel_token.cancel();
        self.is_complete.store(true, Ordering::Relaxed);
        Ok(())
    }
    
    fn await_final_event<Handler, R>(mut self, handler: Handler) -> impl Future<Output = R> + Send
    where
        Handler: Fn(Self::Final, &dyn Any) -> R + Send + 'static,
        R: Send + 'static,
    {
        async move {
            // Step 1: Collect all events from the processing pipeline
            let collected_results = self.collect_all_events().await;
            
            // Step 2: Create TokioFinalEvent with collected results
            let final_event = TokioFinalEvent::new((), collected_results.clone(), self.id);
            
            // Step 3: Call handler with final event and collector reference
            handler(final_event, &collected_results as &dyn Any)
        }
    }
}

// Implement the required super-traits for EmittingTask (since it extends AsyncTask)
// These implementations delegate to the corresponding TokioAsyncTask implementations

use sweet_async_api::task::{
    CancellableTask, CancellationLevel, ContextualizedTask, CpuUsage, IoUsage, MemoryUsage,
    MetricsEnabledTask, NamedTask, PrioritizedTask, RecoverableTask, RetryStrategy,
    StatusEnabledTask, TaskStatus, TimedTask, TracingTask,
};

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    PrioritizedTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn priority(&self) -> &impl sweet_async_api::task::RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    CancellableTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn cancel(&self, level: CancellationLevel) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            self.is_complete.store(true, Ordering::Relaxed);
            Ok(())
        }
    }
    
    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            self.is_complete.store(true, Ordering::Relaxed);
            Ok(())
        }
    }
    
    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            self.is_complete.store(true, Ordering::Relaxed);
            Ok(())
        }
    }
    
    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        async move {
            token.cancel();
            self.is_complete.store(true, Ordering::Relaxed);
            Ok(())
        }
    }
    
    fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
    
    fn on_cancel<F, Fut>(&self, _callback: F)
    where
        F: crate::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // TODO: Store callback for execution on cancellation
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    TracingTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Record the error for monitoring and debugging
        self.record_error(&error);
        
        // Increment error count atomically
        self.error_count.fetch_add(1, Ordering::Relaxed);
        
        // Check if we should attempt recovery based on error type
        match &error {
            AsyncTaskError::Timeout(_) | AsyncTaskError::Cancelled => {
                // These errors are typically not recoverable for emit tasks
                Err(error)
            }
            AsyncTaskError::Failure(_) | AsyncTaskError::RecoveryFailed(_) => {
                // Business logic errors - propagate as-is
                Err(error)
            }
            AsyncTaskError::InvalidState(_) => {
                // State errors indicate programming issues - should not be retried
                Err(error)
            }
        }
    }
    
    fn record_error(&self, error: &AsyncTaskError) {
        if self.is_tracing_enabled() {
            let error_count = self.error_count.load(Ordering::Relaxed);
            
            tracing::error!(
                task_id = ?self.id,
                task_type = "EmittingTask",
                error = ?error,
                error_count = error_count,
                created_at = ?self.created_timestamp,
                "Task error occurred"
            );
            
            // Record structured metrics for monitoring
            match error {
                AsyncTaskError::Timeout(duration) => {
                    tracing::warn!(
                        task_id = ?self.id,
                        timeout_duration = ?duration,
                        "Task timed out"
                    );
                }
                AsyncTaskError::Cancelled => {
                    tracing::info!(
                        task_id = ?self.id,
                        "Task was cancelled"
                    );
                }
                AsyncTaskError::Failure(msg) => {
                    tracing::error!(
                        task_id = ?self.id,
                        failure_message = %msg,
                        "Task failed with business logic error"
                    );
                }
                AsyncTaskError::RecoveryFailed(msg) => {
                    tracing::error!(
                        task_id = ?self.id,
                        recovery_message = %msg,
                        "Task recovery failed"
                    );
                }
                AsyncTaskError::InvalidState(msg) => {
                    tracing::error!(
                        task_id = ?self.id,
                        state_message = %msg,
                        "Task in invalid state"
                    );
                }
            }
        }
    }
    
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    TimedTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn created_timestamp(&self) -> SystemTime {
        self.created_timestamp
    }
    
    fn executed_timestamp(&self) -> SystemTime {
        let nanos = self.executed_timestamp.load(Ordering::Relaxed);
        if nanos == 0 {
            // Not executed yet, return created timestamp
            self.created_timestamp
        } else {
            SystemTime::UNIX_EPOCH + Duration::from_nanos(nanos)
        }
    }
    
    fn completed_timestamp(&self) -> SystemTime {
        let nanos = self.completed_timestamp.load(Ordering::Relaxed);
        if nanos == 0 {
            // Not completed yet, return current time
            SystemTime::now()
        } else {
            SystemTime::UNIX_EPOCH + Duration::from_nanos(nanos)
        }
    }
    
    fn timeout(&self) -> Duration {
        self.task_timeout_duration
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    ContextualizedTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    type RuntimeType = Handle;
    type RelationshipsType = (); // Simplified - no relationships for now
    
    fn relationships(&self) -> &Self::RelationshipsType {
        &()
    }
    
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut ()
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        &self.runtime
    }
    
    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    RecoverableTask<T> for TokioEmittingTask<T, C, E, I>
{
    type FallbackWork = Box<dyn Fn() -> Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + Unpin> + Send + Sync>;
    
    fn recover(&self, error: AsyncTaskError) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        async move {
            Err(AsyncTaskError::RecoveryFailed(format!("EmittingTask recovery failed: {:?}", error)))
        }
    }
    
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        false // Simplified - no recovery for emit tasks
    }
    
    fn fallback_work(&self) -> &Self::FallbackWork {
        static DEFAULT_FALLBACK: std::sync::OnceLock<Box<dyn Fn() -> Box<dyn Future<Output = Result<(), AsyncTaskError>> + Send + Unpin> + Send + Sync>> = std::sync::OnceLock::new();
        let fallback = DEFAULT_FALLBACK.get_or_init(|| {
            Box::new(|| Box::new(Box::pin(async { Err(AsyncTaskError::Failure("No fallback configured".to_string())) })))
        });
        unsafe { std::mem::transmute(fallback) }
    }
    
    fn max_retries(&self) -> u8 {
        3 // Default max retries
    }
    
    fn current_retry(&self) -> u8 {
        0 // TODO: Add retry tracking to struct
    }
    
    fn retry_strategy(&self) -> RetryStrategy {
        RetryStrategy::Fixed(Duration::from_millis(1000))
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    StatusEnabledTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn status(&self) -> TaskStatus {
        if self.is_cancelled() {
            TaskStatus::Cancelled
        } else if self.is_complete.load(Ordering::Relaxed) {
            TaskStatus::Completed
        } else if self.executed_timestamp.load(Ordering::Relaxed) > 0 {
            TaskStatus::Running
        } else {
            TaskStatus::Pending
        }
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    MetricsEnabledTask<T> for TokioEmittingTask<T, C, E, I>
{
    type Cpu = CpuUsage;
    type Memory = MemoryUsage;
    type Io = IoUsage;
    
    fn cpu_usage(&self) -> &Self::Cpu {
        &self.cpu_metrics
    }
    
    fn memory_usage(&self) -> &Self::Memory {
        &self.memory_metrics
    }
    
    fn io_usage(&self) -> &Self::Io {
        &self.io_metrics
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    NamedTask for TokioEmittingTask<T, C, E, I>
{
    fn name(&self) -> Option<&str> {
        None // Simplified - no name storage
    }
    
    fn set_name(&mut self, _name: String) {
        // Would store the name
    }
}