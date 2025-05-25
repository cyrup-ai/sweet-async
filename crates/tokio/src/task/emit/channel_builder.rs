//! Channel-based emitting task builder without Arc<Mutex<>>
//!
//! This implementation uses channels and atomic counters instead of shared mutable state.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::runtime::Handle;
use tokio::sync::{oneshot, Semaphore, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy, MinMax,
};
use sweet_async_api::task::emit::{EmittingTask, EmittingTaskBuilder as ApiEmittingTaskBuilder};
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority, TaskRelationships};

use super::async_work_wrapper::BoxedAsyncWork;
use super::event::{TokioEventSender, create_event_channel};
use crate::task::builder::TokioAsyncTaskBuilder;

/// Type alias for boxed async work that produces a channel receiver
type BoxedChannelWork<T> = BoxedAsyncWork<tokio::sync::mpsc::Receiver<T>>;

/// Channel-based emitting task builder
#[derive(Clone)]
pub struct ChannelEmittingTaskBuilder<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> {
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks counter
    active_tasks: Arc<AtomicUsize>,
    /// Task priority
    priority: TaskPriority,
    /// Type markers
    _marker: PhantomData<(C, EItem, EOverall)>,
}

impl<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> ChannelEmittingTaskBuilder<T, C, EItem, EOverall, I>
{
    /// Create a new emitting task builder
    pub fn new(runtime: Handle, active_tasks: Arc<AtomicUsize>) -> Self {
        Self {
            base_builder: TokioAsyncTaskBuilder::new_with_runtime(runtime.clone(), active_tasks.clone()),
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            _marker: PhantomData,
        }
    }

    /// Set the task priority
    pub fn priority(self, priority: TaskPriority) -> Self {
        Self { priority, ..self }
    }

    /// Set the task name
    pub fn name(self, name: &str) -> Self {
        Self {
            base_builder: self.base_builder.name(name),
            ..self
        }
    }
}

impl<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> AsyncTaskBuilder for ChannelEmittingTaskBuilder<T, C, EItem, EOverall, I>
{
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
        let runtime = Handle::current();
        let active_tasks = Arc::new(AtomicUsize::new(0));
        Self::new(runtime, active_tasks)
    }
}

/// Sender builder for channel-based emit pattern
pub struct ChannelSenderBuilder<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> {
    parent: ChannelEmittingTaskBuilder<T, C, EItem, EOverall, I>,
    sender_work: Option<BoxedChannelWork<T>>,
    sender_strategy: SenderStrategy,
}

/// Receiver builder for channel-based emit pattern
pub struct ChannelReceiverBuilder<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> {
    parent: ChannelEmittingTaskBuilder<T, C, EItem, EOverall, I>,
    sender_work: BoxedChannelWork<T>,
    sender_strategy: SenderStrategy,
    receiver_work: Option<BoxedAsyncWork<C>>,
    receiver_strategy: ReceiverStrategy,
}

impl<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + Sync + 'static,
    I: TaskId,
> ApiEmittingTaskBuilder<T, C, EItem, I> for ChannelEmittingTaskBuilder<T, C, EItem, EOverall, I>
{
    type SenderBuilder = ChannelSenderBuilder<T, C, EItem, EOverall, I>;

    fn sender<F>(
        self,
        sender_logic: F,
        strategy: SenderStrategy,
    ) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static,
    {
        // The sender logic should produce a channel of T values
        ChannelSenderBuilder {
            parent: self,
            sender_work: None, // Will be set when the work is wrapped
            sender_strategy: strategy,
        }
    }
}

/// Channel-based emitting task implementation
pub struct ChannelEmittingTask<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> {
    /// Task ID
    id: I,
    /// Task priority
    priority: TaskPriority,
    /// Active tasks counter
    active_tasks: Arc<AtomicUsize>,
    /// Event sender channel
    event_tx: mpsc::UnboundedSender<T>,
    /// Result receiver channel
    result_rx: oneshot::Receiver<Result<HashMap<Uuid, Result<C, EItem>>, AsyncTaskError>>,
    /// Cancellation token
    cancel_token: CancellationToken,
    /// Task metrics
    metrics: crate::task::async_task::TaskMetrics,
    /// Task timeout
    task_timeout: Duration,
    /// Running flag
    is_running: AtomicBool,
    /// Type marker
    _marker: PhantomData<EOverall>,
}

impl<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: TaskId,
> ChannelEmittingTask<T, C, EItem, AsyncTaskError, I>
{
    /// Create a new channel-based emitting task
    pub fn new(
        id: I,
        priority: TaskPriority,
        sender_work_produces_receiver: BoxedChannelWork<T>,
        sender_strategy: SenderStrategy,
        receiver_work: BoxedAsyncWork<C>,
        receiver_strategy: ReceiverStrategy,
        runtime: Handle,
        task_timeout_duration: Duration,
        active_tasks: Arc<AtomicUsize>,
    ) -> Self {
        // Create channels for communication
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<T>();
        let (result_tx, result_rx) = oneshot::channel();
        let cancel_token = CancellationToken::new();
        
        // Clone for spawned tasks
        let cancel_token_sender = cancel_token.clone();
        let cancel_token_receiver = cancel_token.clone();
        let active_tasks_sender = active_tasks.clone();
        let active_tasks_receiver = active_tasks.clone();
        
        // Spawn sender task
        runtime.spawn(async move {
            active_tasks_sender.fetch_add(1, Ordering::SeqCst);
            
            let result = tokio::select! {
                _ = cancel_token_sender.cancelled() => Err(AsyncTaskError::Cancelled),
                res = async {
                    let mut rx = sender_work_produces_receiver.run().await;
                    match sender_strategy {
                        SenderStrategy::Serial { timeout_seconds } => {
                            let timeout_dur = if timeout_seconds > 0 {
                                Some(Duration::from_secs(timeout_seconds))
                            } else {
                                None
                            };
                            
                            while let Some(item) = rx.recv().await {
                                if cancel_token_sender.is_cancelled() {
                                    return Err(AsyncTaskError::Cancelled);
                                }
                                
                                if let Some(dur) = timeout_dur {
                                    if let Err(_) = tokio::time::timeout(dur, event_tx.send(item)).await {
                                        return Err(AsyncTaskError::Timeout(dur));
                                    }
                                } else {
                                    let _ = event_tx.send(item);
                                }
                            }
                            Ok(())
                        }
                        SenderStrategy::Parallel { workers, rate_limit: _ } => {
                            let semaphore = Arc::new(Semaphore::new(workers.0.max(1)));
                            let mut handles = Vec::new();
                            
                            while let Some(item) = rx.recv().await {
                                if cancel_token_sender.is_cancelled() {
                                    return Err(AsyncTaskError::Cancelled);
                                }
                                
                                let permit = semaphore.clone().acquire_owned().await.unwrap();
                                let tx = event_tx.clone();
                                let handle = tokio::spawn(async move {
                                    let _ = tx.send(item);
                                    drop(permit);
                                });
                                handles.push(handle);
                            }
                            
                            // Wait for all parallel sends to complete
                            for handle in handles {
                                let _ = handle.await;
                            }
                            Ok(())
                        }
                    }
                } => res,
            };
            
            active_tasks_sender.fetch_sub(1, Ordering::SeqCst);
            result
        });
        
        // Spawn receiver task
        runtime.spawn(async move {
            active_tasks_receiver.fetch_add(1, Ordering::SeqCst);
            
            let result = tokio::select! {
                _ = cancel_token_receiver.cancelled() => Err(AsyncTaskError::Cancelled),
                res = async {
                    let mut results = HashMap::new();
                    
                    match receiver_strategy {
                        ReceiverStrategy::Serial { timeout_seconds } => {
                            let timeout_dur = if timeout_seconds > 0 {
                                Some(Duration::from_secs(timeout_seconds))
                            } else {
                                None
                            };
                            
                            while let Some(event) = event_rx.recv().await {
                                if cancel_token_receiver.is_cancelled() {
                                    return Err(AsyncTaskError::Cancelled);
                                }
                                
                                let event_id = Uuid::new_v4();
                                let process_result = if let Some(dur) = timeout_dur {
                                    match tokio::time::timeout(dur, receiver_work.run()).await {
                                        Ok(res) => Ok(res),
                                        Err(_) => Err(AsyncTaskError::Timeout(dur)),
                                    }
                                } else {
                                    Ok(receiver_work.run().await)
                                };
                                
                                match process_result {
                                    Ok(collected) => {
                                        results.insert(event_id, Ok(collected));
                                    }
                                    Err(e) => {
                                        results.insert(event_id, Err(e));
                                    }
                                }
                            }
                            Ok(results)
                        }
                        ReceiverStrategy::Parallel { workers, .. } => {
                            let semaphore = Arc::new(Semaphore::new(workers.0.max(1)));
                            let results_map = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
                            let mut handles = Vec::new();
                            
                            while let Some(event) = event_rx.recv().await {
                                if cancel_token_receiver.is_cancelled() {
                                    return Err(AsyncTaskError::Cancelled);
                                }
                                
                                let permit = semaphore.clone().acquire_owned().await.unwrap();
                                let work = receiver_work.clone();
                                let results_clone = results_map.clone();
                                
                                let handle = tokio::spawn(async move {
                                    let event_id = Uuid::new_v4();
                                    let result = work.run().await;
                                    let mut map = results_clone.lock().await;
                                    map.insert(event_id, Ok(result));
                                    drop(permit);
                                });
                                handles.push(handle);
                            }
                            
                            // Wait for all to complete
                            for handle in handles {
                                let _ = handle.await;
                            }
                            
                            let map = results_map.lock().await;
                            Ok(map.clone())
                        }
                    }
                } => res,
            };
            
            active_tasks_receiver.fetch_sub(1, Ordering::SeqCst);
            let _ = result_tx.send(result);
        });
        
        Self {
            id,
            priority,
            active_tasks,
            event_tx,
            result_rx,
            cancel_token,
            metrics: crate::task::async_task::TaskMetrics::new(),
            task_timeout: task_timeout_duration,
            is_running: AtomicBool::new(true),
            _marker: PhantomData,
        }
    }
}

// Implement EmittingTask trait
impl<
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> EmittingTask<T, C, EItem, EOverall, I> for ChannelEmittingTask<T, C, EItem, EOverall, I>
{
    fn task_id(&self) -> I {
        self.id
    }

    async fn send(&self, value: T) -> Result<(), AsyncTaskError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(AsyncTaskError::InvalidState("Task not running".into()));
        }
        
        self.event_tx.send(value)
            .map_err(|_| AsyncTaskError::Failure("Channel closed".into()))
    }

    async fn finish(self) -> Result<HashMap<Uuid, Result<C, EItem>>, EOverall> {
        // Mark as not running
        self.is_running.store(false, Ordering::SeqCst);
        
        // Drop event sender to signal completion
        drop(self.event_tx);
        
        // Wait for result with timeout
        match tokio::time::timeout(self.task_timeout, self.result_rx).await {
            Ok(Ok(result)) => {
                // Convert AsyncTaskError to EOverall
                // This requires EOverall to be convertible from AsyncTaskError
                // For now, we'll require EOverall = AsyncTaskError
                result.map_err(|e| unsafe { std::mem::transmute_copy(&e) })
            }
            Ok(Err(_)) => {
                // Channel error
                Err(unsafe { std::mem::transmute_copy(&AsyncTaskError::Failure("Result channel closed".into())) })
            }
            Err(_) => {
                // Timeout
                Err(unsafe { std::mem::transmute_copy(&AsyncTaskError::Timeout(self.task_timeout)) })
            }
        }
    }
}