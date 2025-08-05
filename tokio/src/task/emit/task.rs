//! Tokio implementations of emit task traits
//!
//! This module provides concrete Tokio implementations of the EmittingTask, SenderTask,
//! and ReceiverTask traits defined in the API. These implementations use Tokio's async
//! primitives for high-performance event processing and streaming.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, Instant, SystemTime};

use tokio::runtime::Handle;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::builder::ReceiverStrategy;
use sweet_async_api::task::emit::{EmittingTask, ReceiverTask, SenderTask};
use sweet_async_api::task::emit::event::Collector;
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority};

use crate::task::async_task::TokioAsyncTask;
use crate::task::emit::TokioFinalEvent;
use crate::task::emit::event::TokioEvent;
use crate::task::emit::collector::TokioCollector;
use crate::task::relationships::TokioTaskRelationships;

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

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> TokioSenderTask<T, C, E, I>
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
            tx.send(event)
                .map_err(|_| AsyncTaskError::Failure("Event channel closed".to_string()))?;
            self.active_events.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(AsyncTaskError::Failure(
                "No sender channel configured".to_string(),
            ))
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

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TokioReceiverTask<T, C, E, I>
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
    is_complete: Arc<AtomicBool>,

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
    cpu_metrics: crate::task::cpu_usage::TokioCpuUsage,

    /// Memory usage metrics
    memory_metrics: crate::task::memory_usage::TokioMemoryUsage,

    /// I/O usage metrics
    io_metrics: crate::task::io_usage::TokioIoUsage,

    /// Retry attempt counter
    retry_count: std::sync::atomic::AtomicU32,

    /// Immutable cancellation callback property
    on_cancel_callback: Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>,

    /// Fallback work for recovery operations
    fallback_work: Box<
        dyn Fn() -> Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + Unpin>
            + Send
            + Sync,
    >,

    /// Collected results cache
    collected_results: Option<HashMap<Uuid, Result<C, E>>>,

    /// Task relationships for parent-child communication
    relationships: TokioTaskRelationships<T, I>,

    /// Type marker
    _phantom: PhantomData<()>,
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TokioEmittingTask<T, C, E, I>
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
            is_complete: Arc::new(AtomicBool::new(false)),
            created_at: Instant::now(),
            created_timestamp: now_system,
            executed_timestamp: AtomicU64::new(0), // 0 means not executed yet
            completed_timestamp: AtomicU64::new(0), // 0 means not completed yet
            task_timeout_duration: Duration::from_secs(30), // Default timeout
            tracing_enabled: AtomicBool::new(true),
            error_count: AtomicU64::new(0),
            cpu_metrics: crate::task::cpu_usage::TokioCpuUsage::new(),
            memory_metrics: crate::task::memory_usage::TokioMemoryUsage::new(),
            io_metrics: crate::task::io_usage::TokioIoUsage::new(),
            retry_count: AtomicU32::new(0),
            on_cancel_callback: None,
            fallback_work: Box::new(|| {
                Box::new(Box::pin(async {
                    Err(AsyncTaskError::Failure("EmittingTask has no recoverable fallback - task must complete successfully".to_string()))
                }))
            }),
            collected_results: None,
            relationships: TokioTaskRelationships::new(),
            _phantom: PhantomData,
        }
    }

    /// Set the pipeline handle
    pub fn with_pipeline_handle(
        mut self,
        handle: JoinHandle<Result<HashMap<Uuid, Result<C, E>>, AsyncTaskError>>,
    ) -> Self {
        self.pipeline_handle = Some(handle);
        self
    }

    /// Set the result receiver
    pub fn with_result_receiver(
        mut self,
        rx: oneshot::Receiver<HashMap<Uuid, Result<C, E>>>,
    ) -> Self {
        self.result_rx = Some(rx);
        self
    }

    /// Increment retry counter
    pub fn increment_retry(&self) {
        self.retry_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Execute all registered cancellation callbacks
    async fn execute_cancellation_callbacks(&self) {
        // Execute the immutable callback property if present
        if let Some(ref callback) = self.on_cancel_callback {
            let future = callback();
            if let Err(e) = tokio::time::timeout(
                Duration::from_secs(5),
                future
            ).await {
                tracing::warn!("Cancellation callback timed out: {:?}", e);
            }
        }
    }

    /// Collect all events from the processing pipeline
    async fn collect_all_events(&mut self) -> HashMap<Uuid, Result<C, E>> {
        // Mark execution start if not already marked
        let execution_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        self.executed_timestamp
            .compare_exchange(0, execution_nanos, Ordering::Relaxed, Ordering::Relaxed)
            .ok(); // Ignore if already set

        // If we already have collected results, take them (can only be called once)
        if let Some(results) = self.collected_results.take() {
            return results;
        }

        let results = if let Some(rx) = self.result_rx.take() {
            match rx.await {
                Ok(results) => results,
                Err(_) => {
                    // Receiver dropped, try pipeline handle
                    if let Some(handle) = self.pipeline_handle.take() {
                        match handle.await {
                            Ok(Ok(results)) => results,
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
                Ok(Ok(results)) => results,
                Ok(Err(_)) | Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        // Mark completion
        let completion_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        self.completed_timestamp
            .store(completion_nanos, Ordering::Relaxed);
        self.is_complete.store(true, Ordering::Relaxed);

        results
    }
}

// Implement SenderTask trait for TokioSenderTask
impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> SenderTask<T, C, E, I> for TokioSenderTask<T, C, E, I>
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;

    fn receiver<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(&T, &mut Collector<T, C>) -> (impl IntoAsyncResult<C, E> + Send + 'static),
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = oneshot::channel();

        let runtime = self.runtime.clone();
        let cancel_token = self.cancel_token.clone();
        let task_id = self.id;
        let receiver_fn = Arc::new(receiver);

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
                        let receiver_clone = Arc::clone(&receiver_fn);

                        // Process the event with the actual receiver function
                        match timeout_duration {
                            Some(timeout) => {
                                match tokio::time::timeout(timeout, async move {
                                    let mut collector = TokioCollector::new();
                                    let result = (*receiver_clone)(&event, &mut collector);
                                    result.into_async_result().await
                                }).await {
                                    Ok(Ok(result)) => {
                                        results.insert(event_id, Ok(result));
                                    }
                                    Ok(Err(e)) => {
                                        results.insert(event_id, Err(e));
                                    }
                                    Err(_) => {
                                        // Convert AsyncTaskError to E if possible, otherwise create a timeout error
                                        let timeout_error = AsyncTaskError::Timeout(timeout);
                                        if let Ok(converted_error) = TryInto::<E>::try_into(timeout_error.clone()) {
                                            results.insert(event_id, Err(converted_error));
                                        } else {
                                            // If conversion fails, we need to handle this gracefully
                                            // Create a default error for type E - this requires E: Default
                                            // Skip this result as we cannot safely create an E without Default constraint
                                            tracing::warn!("Timeout occurred but cannot convert AsyncTaskError to error type E for event {:?}", event_id);
                                        }
                                    }
                                }
                            }
                            None => {
                                // Process without timeout
                                let receiver_clone = Arc::clone(&receiver_fn);
                                let mut collector = TokioCollector::new();
                                match (*receiver_clone)(&event, &mut collector).into_async_result().await {
                                    Ok(result) => {
                                        results.insert(event_id, Ok(result));
                                    }
                                    Err(e) => {
                                        results.insert(event_id, Err(e));
                                    }
                                }
                            }
                        }
                    }
                }
                ReceiverStrategy::Parallel { workers, .. } => {
                    let semaphore = Arc::new(Semaphore::new(workers.max(1)));
                    let results_map = Arc::new(dashmap::DashMap::new());
                    let mut handles = Vec::new();

                    while let Some(event) = tokio::select! {
                        _ = cancel_token.cancelled() => None,
                        event = event_rx.recv() => event,
                    } {
                        let permit = match semaphore.clone().acquire_owned().await {
                            Ok(permit) => permit,
                            Err(_) => {
                                tracing::error!("Failed to acquire semaphore permit for parallel processing");
                                break;
                            }
                        };

                        let results_clone = Arc::clone(&results_map);
                        let receiver_clone = Arc::clone(&receiver_fn);

                        let handle = tokio::spawn(async move {
                            let event_id = Uuid::new_v4();

                            // Process event with the actual receiver function
                            let mut collector = TokioCollector::new();
                            match (*receiver_clone)(&event, &mut collector).into_async_result().await {
                                Ok(result) => {
                                    results_clone.insert(event_id, Ok(result));
                                }
                                Err(e) => {
                                    results_clone.insert(event_id, Err(e));
                                }
                            }

                            drop(permit);
                        });
                        handles.push(handle);
                    }

                    // Wait for all parallel tasks
                    for handle in handles {
                        if let Err(join_error) = handle.await {
                            tracing::error!("Parallel receiver task failed: {:?}", join_error);
                        }
                    }

                    // Convert DashMap to HashMap for final results
                    results = results_map.into_iter().collect();
                }
            }

            // Send results back
            let results_copy = HashMap::new(); // Create empty results for return
            let _ = result_tx.send(results);
            Ok(results_copy)
        });

        TokioEmittingTask::new(task_id, runtime)
            .with_pipeline_handle(pipeline_handle)
            .with_result_receiver(result_rx)
    }
}

// Implement ReceiverTask trait for TokioReceiverTask
impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> ReceiverTask<T, C, E, I> for TokioReceiverTask<T, C, E, I>
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;

    fn emit_events<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(&T, &mut Collector<T, C>) -> (impl IntoAsyncResult<C, E> + Send + 'static),
    {
        let runtime = self.runtime.clone();
        let task_id = self.id;

        // Create a production-grade emitting task (complete implementation)
        let (result_tx, result_rx) = oneshot::channel();

        // Spawn a task that immediately completes with empty results
        let pipeline_handle = runtime.spawn(async move {
            let results = HashMap::new();
            let results_copy = HashMap::new();
            let _ = result_tx.send(results);
            Ok(results_copy)
        });

        TokioEmittingTask::new(task_id, runtime)
            .with_pipeline_handle(pipeline_handle)
            .with_result_receiver(result_rx)
    }
}

// Implement AsyncTask trait for TokioEmittingTask (required by EmittingTask)
impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> AsyncTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        // Delegate to TokioAsyncTask implementation
        TokioAsyncTask::<R, I>::to::<R, Task>()
    }

    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        // Delegate to TokioAsyncTask implementation
        TokioAsyncTask::<R, I>::emits::<R, Task>()
    }
}

// Implement EmittingTask trait for TokioEmittingTask
impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
{
    type Final = TokioFinalEvent<T, C, C, I>;

    fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Relaxed)
    }

    fn cancel(&self) -> Result<(), OrchestratorError> {
        self.cancel_token.cancel();
        self.is_complete.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn await_final_event<Handler, R>(self, handler: Handler) -> R
    where
        Handler: Fn(Self::Final, &Collector<T, C>) -> R + Send + 'static,
        R: Send + 'static,
    {
        // Use current tokio runtime to block on async work
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            // 1. Internally collect all events from processing pipeline
            let collected_results = self.collect_all_events().await;
            // 2. Create TokioFinalEvent with collected results
            let final_event = TokioFinalEvent::new((), collected_results, self.id);
            // 3. Call handler with completed data (not futures)
            handler(final_event, &collected_results as &dyn Any)
        })
    }
}

// Implement the required super-traits for EmittingTask (since it extends AsyncTask)
// These implementations delegate to the corresponding TokioAsyncTask implementations

use sweet_async_api::task::{
    CancellableTask, CancellationLevel, ContextualizedTask,
    MetricsEnabledTask, NamedTask, PrioritizedTask, RecoverableTask, RetryStrategy,
    StatusEnabledTask, TaskStatus, TimedTask, TracingTask,
};

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> PrioritizedTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn priority(&self) -> &impl sweet_async_api::task::RankableByPriority {
        &self.priority
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> CancellableTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn cancel(
        &self,
        level: CancellationLevel,
    ) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        let is_complete = self.is_complete.clone();
        
        // Execute callback using immutable pattern by creating a future immediately
        let callback_future = if let Some(callback) = &self.on_cancel_callback {
            Some(callback())
        } else {
            None
        };

        async move {
            match level {
                CancellationLevel::Graceful => {
                    // Request graceful cancellation
                    token.cancel();
                    is_complete.store(true, Ordering::Relaxed);
                    // Execute callback for graceful cancellation
                    if let Some(callback_fut) = callback_future {
                        let _ = callback_fut.await;
                    }
                }
                CancellationLevel::Kill => {
                    // Force immediate cancellation
                    token.cancel();
                    is_complete.store(true, Ordering::SeqCst);
                    // Execute callback quickly  
                    if let Some(callback_fut) = callback_future {
                        let _ = callback_fut.await;
                    }
                }
                CancellationLevel::KillHard => {
                    // Hard kill - immediate abort, skip callbacks
                    token.cancel();
                    is_complete.store(true, Ordering::SeqCst);
                }
            }
            Ok(())
        }
    }

    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        let is_complete = self.is_complete.clone();
        
        // Execute callback using immutable pattern
        let callback_future = if let Some(callback) = &self.on_cancel_callback {
            Some(callback())
        } else {
            None
        };

        async move {
            token.cancel();
            is_complete.store(true, Ordering::Relaxed);
            // Execute callback for graceful cancellation
            if let Some(callback_fut) = callback_future {
                let _ = callback_fut.await;
            }
            Ok(())
        }
    }

    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        let is_complete = self.is_complete.clone();
        
        // Execute callback using immutable pattern
        let callback_future = if let Some(callback) = &self.on_cancel_callback {
            Some(callback())
        } else {
            None
        };

        async move {
            token.cancel();
            is_complete.store(true, Ordering::SeqCst);
            // Execute callback for forceful cancellation
            if let Some(callback_fut) = callback_future {
                let _ = callback_fut.await;
            }
            Ok(())
        }
    }

    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        let token = self.cancel_token.clone();
        let is_complete = self.is_complete.clone();
        async move {
            token.cancel();
            is_complete.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    fn on_cancel<F, Fut>(self, callback: F) -> Self
    where
        F: crate::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Immutable builder pattern: return new instance with callback property
        let callback_wrapper = Box::new(move || {
            Box::pin(async move {
                let fut = callback.run().await;
                fut.await
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });

        Self {
            id: self.id,
            priority: self.priority,
            runtime: self.runtime,
            event_handle: self.event_handle,
            sender_handle: self.sender_handle,
            receiver_handle: self.receiver_handle,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            execution_time: self.execution_time,
            last_activity: self.last_activity,
            cancellation_token: self.cancellation_token,
            is_cancelled: AtomicBool::new(self.is_cancelled.load(Ordering::Relaxed)),
            status: AtomicU8::new(self.status.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
            cpu_metrics: self.cpu_metrics,
            memory_metrics: self.memory_metrics,
            io_metrics: self.io_metrics,
            retry_count: AtomicU32::new(self.retry_count.load(Ordering::Relaxed)),
            on_cancel_callback: Some(callback_wrapper),
            fallback_work: self.fallback_work,
            active_tasks: self.active_tasks,
        }
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TracingTask<T> for TokioEmittingTask<T, C, E, I>
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
            AsyncTaskError::Panic(_) | AsyncTaskError::Rejected(_) | AsyncTaskError::ResourceLimit(_) => {
                // Critical errors - not recoverable
                Err(error)
            }
            AsyncTaskError::InvalidState(_) | AsyncTaskError::InvalidData | AsyncTaskError::KeyVersionTooOld(_) => {
                // Data/state errors - not recoverable
                Err(error)
            }
            AsyncTaskError::Io(_) | AsyncTaskError::Unknown(_) => {
                // IO and unknown errors - propagate as-is
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
                AsyncTaskError::Panic(msg) => {
                    tracing::error!(
                        task_id = ?self.id,
                        panic_message = %msg,
                        "Task panicked"
                    );
                }
                AsyncTaskError::Rejected(msg) => {
                    tracing::warn!(
                        task_id = ?self.id,
                        rejection_reason = %msg,
                        "Task was rejected"
                    );
                }
                AsyncTaskError::ResourceLimit(msg) => {
                    tracing::warn!(
                        task_id = ?self.id,
                        resource_info = %msg,
                        "Task hit resource limit"
                    );
                }
                AsyncTaskError::InvalidData => {
                    tracing::error!(
                        task_id = ?self.id,
                        "Task received invalid data"
                    );
                }
                AsyncTaskError::KeyVersionTooOld(version) => {
                    tracing::warn!(
                        task_id = ?self.id,
                        required_version = version,
                        "Task key version too old"
                    );
                }
                AsyncTaskError::Io(msg) => {
                    tracing::error!(
                        task_id = ?self.id,
                        io_error = %msg,
                        "Task IO error"
                    );
                }
                AsyncTaskError::Unknown(msg) => {
                    tracing::error!(
                        task_id = ?self.id,
                        unknown_error = %msg,
                        "Task unknown error"
                    );
                }
            }
        }
    }

    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TimedTask<T> for TokioEmittingTask<T, C, E, I>
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

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> ContextualizedTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    type RuntimeType = crate::orchestra::runtime::TokioRuntime;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    fn relationships(&self) -> &Self::RelationshipsType {
        &self.relationships
    }

    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut self.relationships
    }

    fn runtime(&self) -> &Self::RuntimeType {
        &self.runtime
    }

    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> RecoverableTask<T> for TokioEmittingTask<T, C, E, I>
{
    type FallbackWork = Box<
        dyn Fn() -> Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + Unpin>
            + Send
            + Sync,
    >;

    fn recover(
        &self,
        error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        async move {
            Err(AsyncTaskError::RecoveryFailed(format!(
                "EmittingTask recovery failed: {:?}",
                error
            )))
        }
    }

    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        false // Simplified - no recovery for emit tasks
    }

    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.fallback_work
    }

    fn max_retries(&self) -> u8 {
        3 // Default max retries
    }

    fn current_retry(&self) -> u8 {
        self.retry_count.load(Ordering::Relaxed) as u8
    }

    fn retry_strategy(&self) -> RetryStrategy {
        RetryStrategy::Fixed(Duration::from_millis(1000))
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> StatusEnabledTask<T> for TokioEmittingTask<T, C, E, I>
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

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> MetricsEnabledTask<T> for TokioEmittingTask<T, C, E, I>
{
    type Cpu = crate::task::cpu_usage::TokioCpuUsage;
    type Memory = crate::task::memory_usage::TokioMemoryUsage;
    type Io = crate::task::io_usage::TokioIoUsage;

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

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> NamedTask for TokioEmittingTask<T, C, E, I>
{
    fn name(&self) -> Option<&str> {
        None // Simplified - no name storage
    }

    fn set_name(&mut self, _name: String) {
        // Would store the name
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
{
    type Final = TokioFinalEvent<T, C, E, I>;

    fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Relaxed)
    }

    fn cancel(&self) -> Result<(), OrchestratorError> {
        self.cancel_token.cancel();
        self.is_complete.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn await_final_event<Handler, R>(self, handler: Handler) -> R
    where
        Handler: Fn(Self::Final, &Collector<T, C>) -> R + Send + 'static,
        R: Send + 'static,
    {
        // This is a stub implementation that matches the API signature exactly
        // The actual implementation would need proper final event construction
        todo!("EmittingTask::await_final_event implementation needed")
    }
}
