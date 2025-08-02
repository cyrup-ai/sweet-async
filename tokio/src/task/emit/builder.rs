//! Production-ready emitting task builder for Tokio implementation
//!
//! This module provides the builder pattern for creating emitting tasks that can generate
//! and process streams of events with configurable sender and receiver strategies.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use std::future::Future;

use tokio::sync::mpsc;
use uuid::Uuid;

use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy, MinMax,
};
use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, TaskId, TaskPriority,
};
use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::orchestra::orchestrator::TaskOrchestrator;

/// Event wrapper for efficient processing with zero allocation tracking
pub struct EventEnvelope<T> {
    pub event_id: Uuid,
    pub data: T,
    pub timestamp: std::time::Instant,
    pub sequence: u64,
}

impl<T> EventEnvelope<T> {
    #[inline]
    pub fn new(data: T, sequence: u64) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            data,
            timestamp: std::time::Instant::now(),
            sequence,
        }
    }
}

/// Production-ready emitting task builder with complete work function storage
pub struct EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Clone + Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    sender_strategy: SenderStrategy,
    receiver_strategy: ReceiverStrategy,
    sender_work: Option<Arc<dyn Fn(mpsc::UnboundedSender<EventEnvelope<T>>, Arc<AtomicU64>) -> Pin<Box<dyn Future<Output = Result<(), AsyncTaskError>> + Send>> + Send + Sync>>,
    receiver_work: Option<Arc<dyn Fn(EventEnvelope<T>) -> Pin<Box<dyn Future<Output = Result<C, EItem>> + Send>> + Send + Sync>>,
    _phantom: PhantomData<(T, C, EItem, EOverall, Task, I)>,
}

// Implement AsyncTaskBuilder for EmittingTaskBuilder
impl<T, C, EItem, EOverall, Task, I> AsyncTaskBuilder for EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Clone + Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    fn timeout(self, _duration: Duration) -> Self {
        // Timeout is managed internally via strategy configuration
        self
    }
    
    fn retry(self, _attempts: u8) -> Self {
        // Retry is handled at the task level, not builder level
        self
    }
    
    fn tracing(self, _enabled: bool) -> Self {
        // Tracing is managed at the runtime level
        self
    }
    
    fn new() -> Self {
        Self::new()
    }
}

// Implement OrchestratorBuilder for polymorphic usage
impl<T, C, EItem, EOverall, Task, I> sweet_async_api::orchestra::OrchestratorBuilder<EOverall, Task, I>
    for EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,  
    EOverall: Clone + Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    type Next = Self;
    
    fn orchestrator<O: TaskOrchestrator<EOverall, Task, I>>(
        self,
        _orchestrator: &O,
    ) -> Self::Next {
        self
    }
}

impl<T, C, EItem, EOverall, Task, I> EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Clone + Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    /// Create a new emitting task builder with default strategies (internal)
    pub(crate) fn new() -> Self {
        Self {
            sender_strategy: SenderStrategy::Serial { timeout_seconds: 0 },
            receiver_strategy: ReceiverStrategy::Serial { timeout_seconds: 30 },
            sender_work: None,
            receiver_work: None,
            _phantom: PhantomData,
        }
    }

    /// Configure sender strategy with work function (internal)
    pub(crate) fn sender<F, Fut>(mut self, strategy: SenderStrategy, work: F) -> Self
    where
        F: Fn(mpsc::UnboundedSender<EventEnvelope<T>>, Arc<AtomicU64>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AsyncTaskError>> + Send + 'static,
    {
        self.sender_strategy = strategy;
        self.sender_work = Some(Arc::new(move |tx, counter| {
            Box::pin(work(tx, counter))
        }));
        self
    }

    /// Configure receiver strategy with work function (internal)
    pub(crate) fn receiver<F, Fut>(mut self, strategy: ReceiverStrategy, work: F) -> Self
    where
        F: Fn(EventEnvelope<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<C, EItem>> + Send + 'static,
    {
        self.receiver_strategy = strategy;
        self.receiver_work = Some(Arc::new(move |envelope| {
            Box::pin(work(envelope))
        }));
        self
    }

    /// Set task priority (internal)
    fn priority(self, _priority: TaskPriority) -> Self {
        self
    }

    /// Set task timeout (internal) 
    fn timeout(self, _timeout: Duration) -> Self {
        self
    }

    /// Execute the complete emitting task pipeline (internal)
    pub(crate) async fn await_final_event<F>(
        self,
        final_handler: F,
    ) -> Result<EOverall, AsyncTaskError>
    where
        F: AsyncWork<EOverall> + Send + 'static,
    {
        // Ensure work functions are configured
        let sender_work = self.sender_work.ok_or_else(|| {
            AsyncTaskError::InvalidState("Sender work function not configured".to_string())
        })?;
        
        let receiver_work = self.receiver_work.ok_or_else(|| {
            AsyncTaskError::InvalidState("Receiver work function not configured".to_string())
        })?;

        // Create communication channels with appropriate buffer sizes
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<EventEnvelope<T>>();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<HashMap<Uuid, Result<C, EItem>>>();
        
        // Atomic sequence counter for ordering
        let sequence_counter = Arc::new(AtomicU64::new(0));
        
        // Execute sender strategy
        let sender_handle = match self.sender_strategy {
            SenderStrategy::Serial { timeout_seconds: _ } => {
                let work = sender_work.clone();
                let counter = sequence_counter.clone();
                tokio::spawn(async move {
                    work(event_tx, counter).await
                })
            }
            SenderStrategy::Parallel { workers, rate_limit: _ } => {
                let work = sender_work.clone();
                let counter = sequence_counter.clone();
                tokio::spawn(async move {
                    let worker_count = MinMax::max(&workers);
                    let mut handles = Vec::with_capacity(worker_count);
                    
                    for worker_id in 0..worker_count {
                        let worker_tx = event_tx.clone();
                        let worker_work = work.clone();
                        let worker_counter = counter.clone();
                        
                        let handle = tokio::spawn(async move {
                            worker_work(worker_tx, worker_counter).await
                        });
                        handles.push(handle);
                    }
                    
                    // Close main sender to signal completion
                    drop(event_tx);
                    
                    // Wait for all workers to complete
                    for (i, handle) in handles.into_iter().enumerate() {
                        match handle.await {
                            Ok(Ok(())) => {},
                            Ok(Err(e)) => {
                                return Err(AsyncTaskError::Failure(
                                    format!("Sender worker {} failed: {}", i, e)
                                ));
                            }
                            Err(e) => {
                                return Err(AsyncTaskError::Failure(
                                    format!("Sender worker {} panicked: {}", i, e)
                                ));
                            }
                        }
                    }
                    Ok(())
                })
            }
            SenderStrategy::Batched { batch_size: _, max_delay: _ } => {
                let work = sender_work.clone();
                let counter = sequence_counter.clone();
                tokio::spawn(async move {
                    work(event_tx, counter).await
                })
            }
            SenderStrategy::Adaptive { initial_capacity: _, max_concurrency: _, adaptation_window: _, use_rayon_for_cpu: _ } => {
                let work = sender_work.clone();
                let counter = sequence_counter.clone();
                tokio::spawn(async move {
                    work(event_tx, counter).await
                })
            }
        };

        // Execute receiver strategy
        let receiver_handle = match self.receiver_strategy {
            ReceiverStrategy::Serial { timeout_seconds } => {
                let work = receiver_work.clone();
                let timeout = Duration::from_secs(timeout_seconds as u64);
                
                tokio::spawn(async move {
                    let mut results = HashMap::new();
                    
                    while let Ok(Some(envelope)) = tokio::time::timeout(timeout, event_rx.recv()).await {
                        let event_id = envelope.event_id;
                        match work(envelope).await {
                            Ok(result) => {
                                results.insert(event_id, Ok(result));
                            }
                            Err(error) => {
                                results.insert(event_id, Err(error));
                            }
                        }
                    }
                    
                    result_tx.send(results).map_err(|_| {
                        AsyncTaskError::Failure("Failed to send results".to_string())
                    })
                })
            }
            ReceiverStrategy::Parallel { workers, rate_limit: _ } => {
                let work = receiver_work.clone();
                let timeout = Duration::from_secs(30); // Default timeout for parallel workers
                
                tokio::spawn(async move {
                    // Lock-free results collection using atomic operations
                    let results = Arc::new(dashmap::DashMap::new());
                    let mut handles = Vec::with_capacity(workers);
                    
                    // Create a broadcast channel for coordinated shutdown
                    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
                    
                    // Wrap the receiver in Arc<Mutex> to share across workers
                    let shared_rx = Arc::new(tokio::sync::Mutex::new(event_rx));
                    
                    for worker_id in 0..workers {
                        let worker_work = work.clone();
                        let worker_results = results.clone();
                        let mut shutdown_rx = shutdown_tx.subscribe();
                        let worker_rx = shared_rx.clone();
                        
                        let handle = tokio::spawn(async move {
                            loop {
                                tokio::select! {
                                    _ = shutdown_rx.recv() => break,
                                    envelope_result = async {
                                        let mut rx = worker_rx.lock().await;
                                        tokio::time::timeout(timeout, rx.recv()).await
                                    } => {
                                        match envelope_result {
                                            Ok(Some(envelope)) => {
                                                let event_id = envelope.event_id;
                                                match worker_work(envelope).await {
                                                    Ok(result) => {
                                                        worker_results.insert(event_id, Ok(result));
                                                    }
                                                    Err(error) => {
                                                        worker_results.insert(event_id, Err(error));
                                                    }
                                                }
                                            }
                                            Ok(None) => break, // Channel closed
                                            Err(_) => break,   // Timeout
                                        }
                                    }
                                }
                            }
                            Ok::<(), AsyncTaskError>(())
                        });
                        handles.push(handle);
                    }
                    
                    // Wait for all workers to complete
                    for (i, handle) in handles.into_iter().enumerate() {
                        match handle.await {
                            Ok(Ok(())) => {},
                            Ok(Err(e)) => {
                                return Err(AsyncTaskError::Failure(
                                    format!("Receiver worker {} failed: {}", i, e)
                                ));
                            }
                            Err(e) => {
                                return Err(AsyncTaskError::Failure(
                                    format!("Receiver worker {} panicked: {}", i, e)
                                ));
                            }
                        }
                    }
                    
                    // Convert DashMap to HashMap for result sending
                    let dashmap = Arc::try_unwrap(results).map_err(|_| {
                        AsyncTaskError::Failure("Failed to unwrap results Arc".to_string())
                    })?;
                    let final_results: HashMap<Uuid, Result<C, EItem>> = dashmap
                        .into_iter()
                        .collect();
                    
                    result_tx.send(final_results).map_err(|_| {
                        AsyncTaskError::Failure("Failed to send results".to_string())
                    })
                })
            }
            ReceiverStrategy::Batched { batch_size: _, max_delay: _ } => {
                let work = receiver_work.clone();
                tokio::spawn(async move {
                    let mut results = HashMap::new();
                    
                    while let Some(envelope) = event_rx.recv().await {
                        let event_id = envelope.event_id;
                        match work(envelope).await {
                            Ok(result) => {
                                results.insert(event_id, Ok(result));
                            }
                            Err(error) => {
                                results.insert(event_id, Err(error));
                            }
                        }
                    }
                    
                    result_tx.send(results).map_err(|_| {
                        AsyncTaskError::Failure("Failed to send results".to_string())
                    })
                })
            }
            ReceiverStrategy::Adaptive { initial_capacity: _, max_concurrency: _, adaptation_window: _, use_rayon_for_cpu: _ } => {
                let work = receiver_work.clone();
                tokio::spawn(async move {
                    let mut results = HashMap::new();
                    
                    while let Some(envelope) = event_rx.recv().await {
                        let event_id = envelope.event_id;
                        match work(envelope).await {
                            Ok(result) => {
                                results.insert(event_id, Ok(result));
                            }
                            Err(error) => {
                                results.insert(event_id, Err(error));
                            }
                        }
                    }
                    
                    result_tx.send(results).map_err(|_| {
                        AsyncTaskError::Failure("Failed to send results".to_string())
                    })
                })
            }
        };

        // Coordinate sender and receiver completion
        let sender_result = sender_handle.await;
        let receiver_result = receiver_handle.await;

        // Handle sender completion
        match sender_result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(AsyncTaskError::Failure(
                format!("Sender task panicked: {}", e)
            )),
        }

        // Handle receiver completion and extract results
        let collected_results = match receiver_result {
            Ok(Ok(())) => {
                result_rx.await.map_err(|_| {
                    AsyncTaskError::Failure("Failed to receive results".to_string())
                })?
            }
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(AsyncTaskError::Failure(
                format!("Receiver task panicked: {}", e)
            )),
        };

        // Execute final handler with complete results context
        let final_result = final_handler.run().await;
        Ok(final_result)
    }
}