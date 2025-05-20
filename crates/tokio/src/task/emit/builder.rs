//! Emitting task builder for Tokio implementation
//!
//! This module provides the implementation for creating and configuring stream-based tasks.

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{Stream, StreamExt};
use tokio::runtime::Handle;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, TaskId, TaskPriority
};
use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy,
    SenderBuilder as ApiSenderBuilder, ReceiverBuilder as ApiReceiverBuilder
};
use sweet_async_api::task::emit::{
    EmittingTask, EmittingTaskBuilder as ApiEmittingTaskBuilder
};

use crate::task::builder::TokioAsyncTaskBuilder;
use super::collector::TokioEventCollector;
use super::event::{create_event_channel, TokioEventSender, TokioEventReceiver};

/// Tokio implementation of the EmittingTaskBuilder
#[derive(Clone)]
pub struct TokioEmittingTaskBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Task priority
    priority: TaskPriority,
    /// Type markers
    _marker: PhantomData<(C, E)>,
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    TokioEmittingTaskBuilder<T, C, E, I> 
{
    /// Create a new emitting task builder
    pub fn new(runtime: Handle, active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>) -> Self {
        Self {
            base_builder: TokioAsyncTaskBuilder::new(runtime.clone(), active_tasks.clone()),
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            _marker: PhantomData,
        }
    }

    /// Set the task priority
    pub fn priority(self, priority: TaskPriority) -> Self {
        Self {
            priority,
            ..self
        }
    }
    
    /// Set the task name
    pub fn name(self, name: &str) -> Self {
        Self {
            base_builder: self.base_builder.name(name),
            ..self
        }
    }
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> AsyncTaskBuilder 
    for TokioEmittingTaskBuilder<T, C, E, I> 
{
    // Note: name is not part of the API trait, implement on struct directly

    fn timeout(self, duration: Duration) -> Self {
        Self {
            base_builder: self.base_builder.timeout(duration),
            ..self
        }
    }

    fn retry(self, attempts: u8) -> Self {
        Self {
            base_builder: self.base_builder.retry(attempts),
            ..self
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        Self {
            base_builder: self.base_builder.tracing(enabled),
            ..self
        }
    }

    fn new() -> Self {
        // Create a placeholder with default runtime
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        Self::new(runtime, active_tasks)
    }
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    ApiEmittingTaskBuilder<T, C, E, I> for TokioEmittingTaskBuilder<T, C, E, I> 
{
    type SenderBuilder = TokioSenderBuilder<T, C, E, I>;

    fn sender<F>(self, sender: F, strategy: SenderStrategy) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static
    {
        TokioSenderBuilder::new(
            self.base_builder,
            self.runtime,
            self.active_tasks,
            self.priority,
            sender,
            strategy,
        )
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId, E> sweet_async_api::orchestra::OrchestratorBuilder<T, Task, I> for TokioEmittingTaskBuilder<T, T, E, I>
where
    E: Send + 'static,
{
    type Next = TokioAsyncTaskBuilder<T, I>;
    
    fn orchestrator<O: sweet_async_api::orchestra::orchestrator::TaskOrchestrator<T, Task, I>>(
        self,
        orchestrator: &O,
    ) -> Self::Next {
        // Return the base task builder which implements AsyncTaskBuilder
        self.base_builder
    }
}

/// Tokio implementation of the SenderBuilder
pub struct TokioSenderBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Task priority
    priority: TaskPriority,
    /// Event sender work function
    sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
    /// Sender strategy
    sender_strategy: SenderStrategy,
    /// Type markers
    _marker: PhantomData<(C, E)>,
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    TokioSenderBuilder<T, C, E, I> 
{
    /// Create a new sender builder
    pub fn new<F>(
        base_builder: TokioAsyncTaskBuilder<T, I>,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        sender_work: F,
        sender_strategy: SenderStrategy,
    ) -> Self
    where
        F: AsyncWork<T> + Send + 'static,
    {
        Self {
            base_builder,
            runtime,
            active_tasks,
            priority,
            sender_work: Box::new(sender_work),
            sender_strategy,
            _marker: PhantomData,
        }
    }
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    ApiSenderBuilder<T, C, E, I> for TokioSenderBuilder<T, C, E, I> 
{
    type Receiver = TokioReceiverBuilder<T, C, E, I>;
    type StreamProcessorReceiver<Ch, Coll> = TokioReceiverBuilder<T, C, E, I>
    where
        Ch: Send,
        Coll: Send;
    type EmittingTask = TokioEmittingTask<T, C, E, I>;

    fn receiver(
        self,
        strategy: ReceiverStrategy,
        receiver_fn: fn(&T, &mut (), Uuid) -> C,
    ) -> Self::Receiver {
        TokioReceiverBuilder::new(
            self.base_builder,
            self.runtime,
            self.active_tasks,
            self.priority,
            self.sender_work,
            self.sender_strategy,
            receiver_fn,
            strategy,
        )
    }
    
    fn stream_processor<Ch, F, S, Coll>(
        self,
        f: F,
        strategy: ReceiverStrategy,
    ) -> Self::StreamProcessorReceiver<Ch, Coll>
    where
        Ch: Send,
        F: sweet_async_api::task::builder::AsyncWork<Ch> + Send + 'static,
        S: futures::Stream + Send + 'static,
        Coll: Send + Default + Send + 'static,
        <S as futures::Stream>::Item: sweet_async_api::task::emit::ReceiverEvent<T, C>,
    {
        // TODO: Implement stream processor functionality
        // For now, create a basic receiver that will need proper implementation
        let receiver_work = move |_t: T| Box::pin(async move {
            unimplemented!("stream processor not yet implemented")
        });
        
        TokioReceiverBuilder::new(
            self.base_builder,
            self.runtime,
            self.active_tasks,
            self.priority,
            self.sender_work,
            self.sender_strategy,
            Box::new(receiver_work),
            strategy,
        )
    }
    
    fn run(self) -> Self::EmittingTask {
        // TODO: Implement proper runner
        unimplemented!("run is not implemented yet")
    }
}

/// Tokio implementation of ReceiverBuilder
pub struct TokioReceiverBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Task priority
    priority: TaskPriority,
    /// Event sender work function
    sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
    /// Sender strategy
    sender_strategy: SenderStrategy,
    /// Event receiver function pointer
    receiver_work_fn: fn(&T, &mut (), Uuid) -> C,
    /// Receiver strategy
    receiver_strategy: ReceiverStrategy,
    /// Type markers
    _marker: PhantomData<E>,
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    TokioReceiverBuilder<T, C, E, I> 
{
    /// Create a new receiver builder
    pub fn new(
        base_builder: TokioAsyncTaskBuilder<T, I>,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
        sender_strategy: SenderStrategy,
        receiver_work_fn: fn(&T, &mut (), Uuid) -> C,
        receiver_strategy: ReceiverStrategy,
    ) -> Self 
    {
        Self {
            base_builder,
            runtime,
            active_tasks,
            priority,
            sender_work,
            sender_strategy,
            receiver_work_fn,
            receiver_strategy,
            _marker: PhantomData,
        }
    }
}

/// Tokio implementation of EmittingTask
pub struct TokioEmittingTask<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Task ID
    id: I,
    /// Task priority
    priority: TaskPriority,
    /// Sender task handle
    sender_handle: Arc<Mutex<Option<JoinHandle<Result<(), AsyncTaskError>>>>>,
    /// Receiver task handle
    receiver_handle: Arc<Mutex<Option<JoinHandle<Result<Vec<C>, AsyncTaskError>>>>>,
    /// Event sender
    event_sender: Arc<TokioEventSender<T>>,
    /// Cancellation sender
    cancel_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    /// Final result from task completion
    final_result: Arc<Mutex<Option<Result<Vec<C>, AsyncTaskError>>>>,
    /// Task metrics
    metrics: crate::task::async_task::TaskMetrics,
    /// Task timeout
    task_timeout: Duration,
    /// Type markers
    _marker: PhantomData<E>,
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, E: Send + 'static, I: TaskId + Copy + Clone + Send + Sync + 'static> 
    TokioEmittingTask<T, C, E, I> 
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: I,
        priority: TaskPriority,
        sender_work_produces_receiver: Box<dyn AsyncWork<tokio::sync::mpsc::Receiver<T>> + Send + 'static>,
        sender_strategy: SenderStrategy,
        receiver_work_fn: fn(&T, &mut (), Uuid) -> C,
        receiver_strategy: ReceiverStrategy,
        runtime: Handle,
        task_timeout_duration: Duration,
        task_retry_count: u8,
        _tracing_enabled: bool,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        _name: Option<String>,
    ) -> Self {
        let (internal_event_tx, internal_event_rx_for_collector) = create_event_channel::<T>(100);
        let (cancel_tx_main_oneshot, mut cancel_rx_for_sender) = oneshot::channel();
        let (_cancel_tx_for_receiver, cancel_rx_for_receiver_collector) = oneshot::channel();
        let sender_join_handle_arc = Arc::new(Mutex::new(None));
        let receiver_join_handle_arc = Arc::new(Mutex::new(None));
        let final_result_arc = Arc::new(Mutex::new(None));
        let runtime_for_sender = runtime.clone();
        let active_tasks_for_sender = active_tasks.clone();
        let sender_join_handle_for_storage = sender_join_handle_arc.clone();
        let internal_event_tx_for_sender_task = internal_event_tx.clone();

        let original_sender_jh: JoinHandle<Result<(), AsyncTaskError>> = runtime_for_sender.spawn(async move {
            let mut user_event_source_rx: tokio::sync::mpsc::Receiver<T> = sender_work_produces_receiver.run().await;
            let mut overall_sender_result: Result<(), AsyncTaskError> = Ok(());

            match sender_strategy {
                SenderStrategy::Serial { timeout_seconds } => {
                    let item_send_timeout = if timeout_seconds > 0 { Some(Duration::from_secs(timeout_seconds)) } else { None };
                    loop {
                        tokio::select! {
                            biased;
                            _ = &mut cancel_rx_for_sender => { return Err(AsyncTaskError::Cancelled); }
                            maybe_event_t = user_event_source_rx.recv() => {
                                if let Some(event_t) = maybe_event_t {
                                    let send_op = internal_event_tx_for_sender_task.send(event_t);
                                    let send_result = match item_send_timeout {
                                        Some(dur) => tokio::time::timeout(dur, send_op).await.map_err(|_| AsyncTaskError::Timeout(dur)),
                                        None => Ok(send_op.await),
                                    };
                                    if let Err(send_error) = send_result.and_then(|res| res.map_err(|_| AsyncTaskError::Failure("Internal channel send failed".to_string()))) {
                                        tracing::error!("SerialSender: Failed to send event to internal channel: {:?}", send_error);
                                        return Err(send_error);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                SenderStrategy::Parallel { workers, rate_limit: _ } => {
                    let semaphore = Arc::new(Semaphore::new(workers.min().max(1)));
                    let mut sender_join_handles: Vec<JoinHandle<Result<(), AsyncTaskError>>> = Vec::new();
                    let mut processing_error: Option<AsyncTaskError> = None;

                    loop {
                        if processing_error.is_some() { break; } // Stop processing if an error occurred
                        tokio::select! {
                            biased;
                            _ = &mut cancel_rx_for_sender => { 
                                processing_error = Some(AsyncTaskError::Cancelled);
                                break; 
                            }
                            maybe_event_t = user_event_source_rx.recv() => {
                                if let Some(event_t) = maybe_event_t {
                                    let permit = match semaphore.clone().acquire_owned().await { Ok(p) => p, Err(_) => { processing_error = Some(AsyncTaskError::Failure("Semaphore closed prematurely".into())); break; } };
                                    let tx_clone = internal_event_tx_for_sender_task.clone();
                                    let jh = tokio::spawn(async move {
                                        let send_result = tx_clone.send(event_t).await;
                                        drop(permit);
                                        if send_result.is_err() {
                                            tracing::warn!("ParallelSender: Failed to send event to internal channel.");
                                            return Err(AsyncTaskError::Failure("Internal channel send failed in parallel worker".to_string()));
                                        }
                                        Ok(())
                                    });
                                    sender_join_handles.push(jh);
                                } else {
                                    break; // User's channel closed
                                }
                            }
                        }
                    }
                    
                    if let Some(AsyncTaskError::Cancelled) = processing_error {
                        tracing::debug!("ParallelSender: Cancellation received. Aborting active send tasks.");
                        for handle in &sender_join_handles { handle.abort(); }
                    }

                    for handle in sender_join_handles {
                        match handle.await {
                            Ok(Ok(())) => { /* Send task success */ }
                            Ok(Err(e)) => { // Send task returned an error
                                tracing::error!("ParallelSender: A send task failed: {:?}", e);
                                if processing_error.is_none() { processing_error = Some(e); } // Capture first error
                            }
                            Err(join_error) => { // Tokio JoinError (panic or cancellation)
                                tracing::error!("ParallelSender: A send task panicked or was aborted: {:?}", join_error);
                                if processing_error.is_none() {
                                     if join_error.is_cancelled() {
                                        // If it was aborted by us, this is expected during cancellation flow.
                                        // Ensure overall_sender_result reflects cancellation if it was the trigger.
                                        if !matches!(processing_error, Some(AsyncTaskError::Cancelled)) {
                                            processing_error = Some(AsyncTaskError::Cancelled); // Or a more specific abort error
                                        }
                                     } else {
                                        processing_error = Some(AsyncTaskError::Failure(format!("Send task panic: {}", join_error)));
                                     }
                                }
                            }
                        }
                    }
                    overall_sender_result = match processing_error {
                        Some(e) => Err(e),
                        None => Ok(()),
                    };
                }
                SenderStrategy::Batched { batch_size, max_delay } => {
                    let mut batch = Vec::with_capacity(batch_size);
                    let mut interval = tokio::time::interval(max_delay);
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay); // Important for batching

                    loop {
                        tokio::select! {
                            biased;
                            _ = &mut cancel_rx_for_sender => { return Err(AsyncTaskError::Cancelled); }
                            _ = interval.tick() => {
                                if !batch.is_empty() {
                                    let current_batch = std::mem::take(&mut batch);
                                    tracing::debug!("BatchedSender: Sending batch of {} items due to timeout", current_batch.len());
                                    for event_t in current_batch {
                                        if internal_event_tx_for_sender_task.send(event_t).await.is_err() {
                                            tracing::error!("BatchedSender: Failed to send event to internal channel during batch send (timeout).");
                                            return Err(AsyncTaskError::Failure("Internal channel send failed in batched sender".to_string()));
                                        }
                                    }
                                }
                            }
                            maybe_event_t = user_event_source_rx.recv() => {
                                if let Some(event_t) = maybe_event_t {
                                    batch.push(event_t);
                                    if batch.len() >= batch_size {
                                        let current_batch = std::mem::take(&mut batch);
                                        tracing::debug!("BatchedSender: Sending batch of {} items due to size", current_batch.len());
                                        for event_t_in_batch in current_batch {
                                            if internal_event_tx_for_sender_task.send(event_t_in_batch).await.is_err() {
                                                tracing::error!("BatchedSender: Failed to send event to internal channel during batch send (size).");
                                                return Err(AsyncTaskError::Failure("Internal channel send failed in batched sender".to_string()));
                                            }
                                        }
                                        interval.reset(); // Reset timer after a full batch is sent
                                    }
                                } else {
                                    // Stream ended, process any remaining items in the batch
                                    if !batch.is_empty() {
                                        tracing::debug!("BatchedSender: Sending final batch of {} items from stream end", batch.len());
                                        for event_t_in_final_batch in batch {
                                            if internal_event_tx_for_sender_task.send(event_t_in_final_batch).await.is_err() {
                                                tracing::error!("BatchedSender: Failed to send event to internal channel during final batch send.");
                                                return Err(AsyncTaskError::Failure("Internal channel send failed in batched sender".to_string()));
                                            }
                                        }
                                    }
                                    break; // Exit loop as source stream ended
                                }
                            }
                        }
                    }
                }
                SenderStrategy::Adaptive { initial_capacity, max_concurrency, adaptation_window, use_rayon_for_cpu: _ } => {
                    tracing::debug!(
                        "AdaptiveSender: Init with initial_cap={}, max_con={}, window={:?}",
                        initial_capacity, max_concurrency, adaptation_window
                    );

                    let max_c = max_concurrency.max(1);
                    let min_c = 1; // Minimum concurrency is 1
                    let mut target_concurrency = initial_capacity.max(min_c).min(max_c);
                    
                    // Semaphore capacity is max_concurrency, actual dispatch is limited by target_concurrency
                    let semaphore = Arc::new(Semaphore::new(max_c)); 
                    
                    let mut active_send_subtasks: usize = 0;
                    let mut send_durations_micros: Vec<u128> = Vec::with_capacity(initial_capacity * 2);
                    let mut successful_sends_window: usize = 0;
                    let mut failed_sends_window: usize = 0;
                    let mut last_adaptation_time = Instant::now();
                    let adaptation_sample_min_count = initial_capacity.max(5); // Min samples before adapting

                    let mut sender_task_join_handles = Vec::new();
                    let (metric_tx, mut metric_rx) = mpsc::channel::<Result<Duration, AsyncTaskError>>(max_c * 2);
                    let mut processing_error: Option<AsyncTaskError> = None;
                    let mut user_stream_ended = false;

                    loop {
                        if processing_error.is_some() { break; }

                        tokio::select! {
                            biased;
                            _ = &mut cancel_rx_for_sender => { 
                                processing_error = Some(AsyncTaskError::Cancelled);
                                break; 
                            }
                            Some(metric_result) = metric_rx.recv() => {
                                active_send_subtasks = active_send_subtasks.saturating_sub(1);
                                match metric_result {
                                    Ok(duration) => {
                                        send_durations_micros.push(duration.as_micros());
                                        successful_sends_window += 1;
                                    }
                                    Err(e) => { 
                                        failed_sends_window += 1;
                                        // If a send worker fails, we might want to halt or be more cautious
                                        tracing::warn!("AdaptiveSender: Worker send error: {:?}", e);
                                        // For now, we log and let AIMD reduce concurrency if errors persist.
                                        // Could make this a hard stop by setting processing_error here.
                                    }
                                }
                            }
                            // Only try to receive from user_event_source_rx if not ended AND we have capacity based on target_concurrency
                            maybe_event_t = user_event_source_rx.recv(), if !user_stream_ended && active_send_subtasks < target_concurrency => {
                                if let Some(event_t) = maybe_event_t {
                                    // Must acquire from overall semaphore before incrementing active_send_subtasks
                                    let permit = match semaphore.clone().try_acquire_owned() { // Use try_acquire for non-blocking check with target_concurrency logic
                                        Ok(p) => p,
                                        Err(_) => { 
                                            // This means semaphore (at max_c) is full, even if target_concurrency allows more.
                                            // This branch should ideally not be hit often if target_concurrency <= max_c.
                                            // Or, we could .await acquire here, as the guard `active_send_subtasks < target_concurrency` already fired.
                                            // Let's use blocking acquire as we already decided we *want* to spawn.
                                            match semaphore.clone().acquire_owned().await {
                                                Ok(p_block) => p_block,
                                                Err(_) => { processing_error = Some(AsyncTaskError::Failure("Semaphore closed during blocking acquire".into())); break; }
                                            }
                                        }
                                    };
                                    active_send_subtasks += 1;

                                    let tx_clone = internal_event_tx_for_sender_task.clone();
                                    let metric_tx_clone = metric_tx.clone();
                                    let jh = tokio::spawn(async move {
                                        let send_start_time = Instant::now();
                                        let send_res = tx_clone.send(event_t).await;
                                        let send_duration = send_start_time.elapsed();
                                        drop(permit);
                                        let task_result = if send_res.is_err() {
                                            Err(AsyncTaskError::Failure("Internal channel send failed in adaptive worker".to_string()))
                                        } else {
                                            Ok(send_duration)
                                        };
                                        if metric_tx_clone.send(task_result).await.is_err() {
                                            tracing::warn!("AdaptiveSender: Failed to send metric.");
                                        }
                                    });
                                    sender_task_join_handles.push(jh);
                                } else {
                                    user_stream_ended = true; // User's channel closed
                                }
                            }
                        }
                        
                        // Adaptation Logic
                        if last_adaptation_time.elapsed() >= adaptation_window && 
                           (successful_sends_window + failed_sends_window) >= adaptation_sample_min_count &&
                           processing_error.is_none() {
                            
                            let total_samples = successful_sends_window + failed_sends_window;
                            let error_rate = if total_samples > 0 { failed_sends_window as f64 / total_samples as f64 } else { 0.0 };
                            let avg_latency_micros = if !send_durations_micros.is_empty() { 
                                send_durations_micros.iter().sum::<u128>() / send_durations_micros.len() as u128
                            } else { 0 };

                            let mut new_target_concurrency = target_concurrency;
                            const HIGH_ERROR_RATE_THRESHOLD: f64 = 0.1; // 10% error rate
                            const HIGH_LATENCY_MICROS: u128 = 50_000; // 50ms
                            const LOW_LATENCY_MICROS: u128 = 10_000;  // 10ms

                            if error_rate > HIGH_ERROR_RATE_THRESHOLD {
                                new_target_concurrency = (target_concurrency as f64 * 0.75).round() as usize; // Multiplicative Decrease
                                tracing::warn!("AdaptiveSender: High error rate ({:.2}%), reducing concurrency.", error_rate * 100.0);
                            } else if avg_latency_micros > HIGH_LATENCY_MICROS {
                                new_target_concurrency = (target_concurrency as f64 * 0.85).round() as usize; // Multiplicative Decrease
                            } else if avg_latency_micros < LOW_LATENCY_MICROS && error_rate == 0.0 {
                                new_target_concurrency += 1; // Additive Increase
                            }
                            new_target_concurrency = new_target_concurrency.max(min_c).min(max_c);

                            if new_target_concurrency != target_concurrency {
                                tracing::debug!(
                                    "AdaptiveSender: Adjusting target C from {} to {} (avg_lat: {}us, err_rate: {:.2}%)", 
                                    target_concurrency, new_target_concurrency, avg_latency_micros, error_rate * 100.0
                                );
                                target_concurrency = new_target_concurrency;
                                // Semaphore capacity is fixed at max_c. Target concurrency limits spawning.
                            }

                            send_durations_micros.clear();
                            successful_sends_window = 0;
                            failed_sends_window = 0;
                            last_adaptation_time = Instant::now();
                        }

                        // Exit condition: user stream ended, no active sub-tasks, and metric channel likely empty by now.
                        if user_stream_ended && active_send_subtasks == 0 && metric_rx.try_recv().is_err() { // try_recv is non-blocking
                            break;
                        }
                    }
                    // ... (Cleanup and joining handles as before, propagating processing_error) ...
                    drop(metric_tx);
                    while let Some(metric_result) = metric_rx.recv().await { // Drain any final metrics
                         match metric_result {
                            Ok(duration) => { send_durations_micros.push(duration.as_micros()); successful_sends_window += 1; }
                            Err(_) => { failed_sends_window += 1; }
                        }
                    }
                    if processing_error.is_some() && !matches!(processing_error, Some(AsyncTaskError::Cancelled)) {
                         // If loop broke due to error, ensure active tasks are handled (aborted if not already done by cancel path)
                         if !cancel_rx_for_sender.try_recv().is_ok() { // If not already in cancellation flow
                            for handle in &sender_task_join_handles { handle.abort(); }
                         }
                    } else if matches!(processing_error, Some(AsyncTaskError::Cancelled)) {
                         for handle in &sender_task_join_handles { handle.abort(); }
                    }

                    for handle in sender_task_join_handles {
                        match handle.await {
                            Ok(_) => {} // Metric processed via channel
                            Err(join_error) => {
                                if processing_error.is_none() {
                                    processing_error = Some(if join_error.is_cancelled() { AsyncTaskError::Cancelled } else { AsyncTaskError::Failure(format!("Send sub-task panic: {}", join_error)) });
                                }
                            }
                        }
                    }
                    overall_sender_result = match processing_error {
                        Some(e) => Err(e),
                        None => Ok(()),
                    };
                }
            }
            overall_sender_result
        });
        
        // ... (Storing sender JoinHandle and Spawning Receiver Task as before) ...
        // ... (Self struct initialization as before) ...
    }

    /// Waits for the emitting task to complete and returns a TokioFinalEvent or an error.
    /// TSummary for TokioFinalEvent is assumed to be () for now.
    pub async fn await_final_event(self) -> Result<crate::task::emit::TokioFinalEvent<(), C, I>, AsyncTaskError>
    {
        let task_id_for_final_event = self.id; // Use the TaskId I directly

        let processing_future = async {
            let receiver_jh_opt = {
                let mut handle_guard = self.receiver_handle.lock().await;
                handle_guard.take() 
            };

            if let Some(receiver_jh) = receiver_jh_opt {
                match receiver_jh.await { // receiver_jh is JoinHandle<Result<HashMap<Uuid, C>, AsyncTaskError>>
                    Ok(task_outcome_result) => { // task_outcome_result is Result<HashMap<Uuid, C>, AsyncTaskError>
                        match task_outcome_result {
                            Ok(collected_map) => {
                                let final_event = crate::task::emit::TokioFinalEvent::new(
                                    (), // TSummary is ()
                                    collected_map, 
                                    task_id_for_final_event // Pass I directly
                                );
                                Ok(final_event)
                            }
                            Err(async_task_error) => {
                                Err(async_task_error)
                            }
                        }
                    }
                    Err(join_error) => { // Tokio JoinError from awaiting the receiver task itself
                        let async_task_error = AsyncTaskError::Failure(format!("Receiver task join error: {}", join_error));
                        {
                            let mut fr_guard = self.final_result.lock().await;
                            if fr_guard.is_none() { // Only set if not already set (e.g., by cancellation)
                               *fr_guard = Some(Err(async_task_error.clone()));
                            }
                        }
                        Err(async_task_error)
                    }
                }
            } else {
                // Handle was already taken or task never started properly.
                // Check final_result which might have been set by cancellation or other early error.
                let final_result_outcome = {
                    self.final_result.lock().await.clone()
                };
                match final_result_outcome {
                    Some(Ok(collected_map)) => {
                        // This case might occur if await_final_event is called multiple times after success,
                        // though self is consumed. Or if logic error allows final_result to be Ok() but handle was None.
                        let final_event = crate::task::emit::TokioFinalEvent::new(
                            (), 
                            collected_map, 
                            task_id_for_final_event
                        );
                        Ok(final_event)
                    }
                    Some(Err(async_task_error)) => Err(async_task_error),
                    None => Err(AsyncTaskError::Failure("Emitting task result unavailable and handle already taken/not set".to_string())),
                }
            }
        };

        if self.task_timeout > Duration::ZERO {
            match tokio::time::timeout(self.task_timeout, processing_future).await {
                Ok(result_of_processing) => result_of_processing, // This is Result<TokioFinalEvent, AsyncTaskError>
                Err(_) => { // Overall task timeout occurred
                    let timeout_error = AsyncTaskError::Timeout(self.task_timeout);
                    {
                        let mut fr_guard = self.final_result.lock().await;
                        // Update final_result only if it wasn't an error already or not set.
                        // Prioritize earlier errors over a later overall timeout if an error was already recorded.
                        if fr_guard.is_none() || matches!(*fr_guard, Some(Ok(_))) { 
                            *fr_guard = Some(Err(timeout_error.clone()));
                        }
                    }
                    Err(timeout_error)
                }
            }
        } else {
            processing_future.await
        }
    }
}

// Implement core AsyncTask methods first (these are required by EmittingTask trait)
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    AsyncTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_spawning()
    }

    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_emitting()
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
{
    // FinalEvent will be implemented in a follow-up PR
    type Final = crate::task::emit::TokioFinalEvent<T, C>;
    
    fn is_complete(&self) -> bool {
        futures::executor::block_on(async {
            self.final_result.lock().await.unwrap().is_some()
        })
    }
    
    fn cancel(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        futures::executor::block_on(async {
            let cancel_tx = self.cancel_tx.clone();
            
            let sender = {
                let mut guard = cancel_tx.lock().await.unwrap();
                std::mem::replace(&mut *guard, None)
            };
            
            if let Some(tx) = sender {
                let _ = tx.send(());
            }
            
            Ok(())
        })
    }
}

// Implement all required AsyncTask supertraits
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::PrioritizedTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn priority(&self) -> &impl sweet_async_api::task::RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::CancellableTask<T> for TokioEmittingTask<T, C, E, I>
{
    async fn cancel(&self, level: sweet_async_api::task::CancellationLevel) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        let cancel_tx = self.cancel_tx.clone();
        
        let sender = {
            let mut guard = cancel_tx.lock().await.unwrap();
            std::mem::replace(&mut *guard, None)
        };
        
        if let Some(tx) = sender {
            let _ = tx.send(());
        }
        
        Ok(())
    }
    
    fn is_cancelled(&self) -> bool {
        futures::executor::block_on(async {
            self.final_result.lock().await.unwrap().is_none()
        })
    }
    
    fn on_cancel<F, Fut>(&self, callback: F)
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // TODO: Implement callback registration
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::TracingTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn handle_error(&self, error: sweet_async_api::task::AsyncTaskError) -> Result<T, sweet_async_api::task::AsyncTaskError> {
        // TODO: Implement proper error handling
        Err(error)
    }
    
    fn record_error(&self, error: &sweet_async_api::task::AsyncTaskError) {
        tracing::error!("Recording error: {:?}", error);
    }
    
    fn is_tracing_enabled(&self) -> bool {
        // TODO: Track tracing state
        false
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::TimedTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn created_timestamp(&self) -> std::time::SystemTime {
        // TODO: Track creation time
        std::time::SystemTime::now()
    }
    
    fn executed_timestamp(&self) -> std::time::SystemTime {
        // TODO: Track execution time
        std::time::SystemTime::now()
    }
    
    fn completed_timestamp(&self) -> std::time::SystemTime {
        // TODO: Track completion time
        std::time::SystemTime::now()
    }
    
    fn timeout(&self) -> std::time::Duration {
        // TODO: Track timeout
        std::time::Duration::from_secs(0)
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::ContextualizedTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    type RuntimeType = crate::runtime::TokioRuntime;
    
    fn child_tasks(&self) -> Vec<T> {
        // TODO: Implement child task tracking
        Vec::new()
    }
    
    fn parent(&self) -> Option<T> {
        // TODO: Implement parent tracking
        None
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        // TODO: Store runtime reference
        panic!("ContextualizedTask::runtime is not directly available")
    }
    
    fn cwd(&self) -> std::path::PathBuf {
        // TODO: Track current working directory
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::StatusEnabledTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn status(&self) -> sweet_async_api::task::TaskStatus {
        // TODO: Track proper status
        if self.is_complete() {
            sweet_async_api::task::TaskStatus::Completed
        } else {
            sweet_async_api::task::TaskStatus::Running
        }
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::RecoverableTask<T> for TokioEmittingTask<T, C, E, I>
{
    fn recover(&self, error: sweet_async_api::task::AsyncTaskError) -> Result<T, sweet_async_api::task::AsyncTaskError> {
        // TODO: Implement recovery logic
        Err(error)
    }
    
    fn can_recover_from(&self, _error: &sweet_async_api::task::AsyncTaskError) -> bool {
        // TODO: Implement recovery logic
        false
    }
    
    fn fallback_value(&self) -> Option<T> {
        // TODO: Implement fallback logic
        None
    }
}

impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::MetricsEnabledTask<T> for TokioEmittingTask<T, C, E, I>
{
    type Cpu = crate::task::async_task::TaskMetrics;
    type Memory = crate::task::async_task::TaskMetrics;
    type Io = crate::task::async_task::TaskMetrics;
    
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

impl<T: Send + 'static, C: Clone + Send + 'static, E: Send + Sync + 'static, I: TaskId> 
    ApiReceiverBuilder<T, C, E, I> for TokioReceiverBuilder<T, C, E, I> 
{
    type Task = TokioEmittingTask<T, C, E, I>;
    
    fn start_queue(self) -> Self::Task {
        let task_id_str = self.base_builder.get_name().unwrap_or_else(|| {
            format!("emitting-task-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
        });
        let task_id = I::from_string(&task_id_str).unwrap_or_else(|| {
            let fallback_id_str = format!("fallback-emitting-task-{}", uuid::Uuid::new_v4());
            I::from_string(&fallback_id_str).expect("Failed to create TaskId for emitting task")
        });
        
        TokioEmittingTask::new(
            task_id,
            self.priority,
            self.sender_work,
            self.sender_strategy,
            self.receiver_work_fn,
            self.receiver_strategy,
            self.runtime.clone(),
            self.base_builder.get_timeout(),
            self.base_builder.get_retry_attempts(),
            self.base_builder.is_tracing_enabled(),
            self.active_tasks,
            self.base_builder.get_name(),
        )
    }
}

// Helper function for metrics - replace with actual logic
fn crate_shared_metrics_instance() -> crate::task::async_task::TaskMetrics {
    crate::task::async_task::TaskMetrics::new()
}
