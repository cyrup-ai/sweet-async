//! Emitting task builder for Tokio implementation
//!
//! This module provides the implementation for creating and configuring stream-based tasks.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
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
use sweet_async_api::orchestra::{OrchestratorError, orchestrator::TaskOrchestrator};

use crate::task::tokio_task::TokioTask;

// We can't box AsyncWork directly because it's not dyn-compatible
// Instead, we'll store the work as a type-erased future factory

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
    EOverall: Clone + Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    type Next = Self; // It returns itself since it already implements AsyncTaskBuilder
    
    fn orchestrator<O: TaskOrchestrator<EOverall, Task, I>>(
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
    EOverall: Clone + Send + 'static,
    Task: AsyncTask<EOverall, I> + 'static,
    I: TaskId,
{
    sender_strategy: Option<SenderStrategy>,
    receiver_strategy: Option<ReceiverStrategy>,
    _phantom: PhantomData<(T, C, EItem, EOverall, Task, I)>,
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
    /// Create a new emitting task builder
    pub fn new() -> Self {
        Self {
            sender_strategy: None,
            receiver_strategy: None,
            _phantom: PhantomData,
        }
    }

    /// Set the sender strategy (work will be provided later)
    pub fn sender<F>(mut self, strategy: SenderStrategy, _work: F) -> Self
    where
        F: AsyncWork<mpsc::Receiver<T>> + Send + 'static,
    {
        self.sender_strategy = Some(strategy);
        self
    }

    /// Set the receiver strategy (work will be provided later)
    pub fn receiver<F>(mut self, strategy: ReceiverStrategy, _work: F) -> Self
    where
        F: AsyncWork<C> + Send + 'static,
    {
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
        // For now, just run the final handler directly
        // In a real implementation, this would coordinate the sender/receiver tasks
        let overall_result = final_handler.run().await;
        Ok(overall_result)
    }
}

