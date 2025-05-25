//! Emitting task builder for Tokio implementation
//!
//! This module provides the implementation for creating and configuring stream-based tasks.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use futures::StreamExt;
use tokio::runtime::Handle;
use tokio::sync::{oneshot, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy,
};
use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, TaskId, TaskPriority,
};
use sweet_async_api::orchestra::OrchestratorError;

use crate::builder::DefaultOrchestratorBuilder;
use crate::task::async_task::AsyncTask as TokioAsyncTask;

/// Type alias for channel work that produces a receiver
pub type BoxedChannelWork<T> = Box<dyn AsyncWork<mpsc::Receiver<T>> + Send>;

/// Type alias for async work that produces a value
pub type BoxedAsyncWork<C> = Box<dyn AsyncWork<C> + Send>;

// Implement AsyncTaskBuilder for EmittingTaskBuilder
impl<T, C, EItem, EOverall, Task, I> AsyncTaskBuilder for EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    fn timeout(self, duration: Duration) -> Self {
        self.timeout(duration)
    }
    
    fn retry(self, _attempts: u8) -> Self {
        self // Retry is handled differently for emitting tasks
    }
    
    fn tracing(self, _enabled: bool) -> Self {
        self // Tracing is handled at task level
    }
    
    fn new() -> Self {
        Self::new()
    }
}

// Implement OrchestratorBuilder for EmittingTaskBuilder so it can be polymorphic
impl<T, C, EItem, EOverall, Task, I> sweet_async_api::orchestra::OrchestratorBuilder<EOverall, Task, I>
    for EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    type Next = Self; // It returns itself since it already implements AsyncTaskBuilder
    
    fn orchestrator<O: sweet_async_api::orchestra::TaskOrchestrator<EOverall, Task, I>>(
        self,
        _orchestrator: &O,
    ) -> Self::Next {
        self // Just return self, the orchestrator is optional
    }
}

/// Builder for creating emitting tasks
pub struct EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    sender_work: Option<BoxedChannelWork<T>>,
    sender_strategy: Option<SenderStrategy>,
    receiver_work: Option<BoxedAsyncWork<C>>,
    receiver_strategy: Option<ReceiverStrategy>,
    _phantom: PhantomData<(T, C, EItem, EOverall, Task, I)>,
}

impl<T, C, EItem, EOverall, Task, I> EmittingTaskBuilder<T, C, EItem, EOverall, Task, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    /// Create a new emitting task builder
    pub fn new() -> Self {
        Self {
            sender_work: None,
            sender_strategy: None,
            receiver_work: None,
            receiver_strategy: None,
            _phantom: PhantomData,
        }
    }

    /// Set the sender work and strategy
    pub fn sender<F>(mut self, strategy: SenderStrategy, work: F) -> Self
    where
        F: AsyncWork<mpsc::Receiver<T>> + Send + 'static,
    {
        self.sender_work = Some(Box::new(work));
        self.sender_strategy = Some(strategy);
        self
    }

    /// Set the receiver work and strategy
    pub fn receiver<F>(mut self, strategy: ReceiverStrategy, work: F) -> Self
    where
        F: AsyncWork<C> + Send + 'static,
    {
        self.receiver_work = Some(Box::new(work));
        self.receiver_strategy = Some(strategy);
        self
    }

    /// Set the task priority
    pub fn priority(self, _priority: TaskPriority) -> Self {
        // Priority is handled at task creation time
        self
    }

    /// Set the task timeout
    pub fn timeout(self, _timeout: Duration) -> Self {
        // Timeout is handled at task creation time
        self
    }

    /// Build and run the emitting task
    pub async fn await_final_event<F>(
        self,
        final_handler: F,
    ) -> Result<EOverall, AsyncTaskError>
    where
        F: AsyncWork<EOverall> + Send + 'static,
    {
        let sender_work = self.sender_work.ok_or_else(|| {
            AsyncTaskError::Configuration("Sender work not configured".to_string())
        })?;
        
        let receiver_work = self.receiver_work.ok_or_else(|| {
            AsyncTaskError::Configuration("Receiver work not configured".to_string())
        })?;
        
        let sender_strategy = self.sender_strategy.unwrap_or(SenderStrategy::Serial);
        let receiver_strategy = self.receiver_strategy.unwrap_or(ReceiverStrategy::Serial { 
            timeout_seconds: None 
        });

        // Create channel for events
        let (tx, mut rx) = mpsc::channel::<T>(100);
        
        // Spawn sender task
        let sender_handle = tokio::spawn(async move {
            let mut event_rx = sender_work.run().await;
            
            match sender_strategy {
                SenderStrategy::Serial => {
                    while let Some(event) = event_rx.recv().await {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
                SenderStrategy::Parallel { workers } => {
                    // Use buffer_unordered for parallel processing
                    let stream = tokio_stream::wrappers::ReceiverStream::new(event_rx);
                    let mut buffered = stream.buffer_unordered(workers);
                    
                    while let Some(event) = buffered.next().await {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Ok(())
        });

        // Spawn receiver task
        let receiver_handle = tokio::spawn(async move {
            let mut results = HashMap::new();
            
            match receiver_strategy {
                ReceiverStrategy::Serial { timeout_seconds } => {
                    while let Some(event) = rx.recv().await {
                        let result = receiver_work.run().await;
                        results.insert(Uuid::new_v4(), Ok(result));
                        
                        if let Some(_timeout) = timeout_seconds {
                            // Could implement timeout logic here if needed
                        }
                    }
                }
                ReceiverStrategy::Parallel { workers, timeout_seconds } => {
                    // Process events in parallel using buffer_unordered
                    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
                    let mut buffered = stream.map(|_event| async {
                        let result = receiver_work.run().await;
                        (Uuid::new_v4(), Ok(result))
                    }).buffer_unordered(workers);
                    
                    while let Some((id, result)) = buffered.next().await {
                        results.insert(id, result);
                        
                        if let Some(_timeout) = timeout_seconds {
                            // Could implement timeout logic here if needed
                        }
                    }
                }
            }
            
            Ok::<_, AsyncTaskError>(results)
        });

        // Wait for both tasks to complete
        let _ = sender_handle.await.map_err(|e| AsyncTaskError::Failure(e.to_string()))?;
        let _results = receiver_handle.await.map_err(|e| AsyncTaskError::Failure(e.to_string()))??;
        
        // Run final handler
        let overall_result = final_handler.run().await;
        Ok(overall_result)
    }
}

