//! Emitting task implementation for Tokio
//!
//! This module provides the implementation for tasks that emit events with configurable processing strategies.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::builder::ReceiverStrategy;
use sweet_async_api::task::emit::{EmittingTask, FinalEvent, SenderTask, ReceiverTask};
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority};

use crate::task::tokio_task::TokioTask;
use crate::task::emit::{TokioEmittingTask, TokioFinalEvent};

/// Implementation of SenderTask for AsyncTask
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    SenderTask<T, C, E, I> for AsyncTask<T, I>
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;

    fn receiver<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn() -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static
    {
        // Create a new TokioEmittingTask based on this task as the sender
        
        // Create a runtime handle
        let runtime = tokio::runtime::Handle::current();
        
        // Create the active tasks registry
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        
        // Create a SenderStrategy (using simple Serial strategy for now)
        let sender_strategy = sweet_async_api::task::builder::SenderStrategy::Serial { 
            timeout_seconds: 30 
        };
        
        // Create sender work function that just returns the current task's value
        let task_id = self.task_id();
        let sender_work = Box::new(move || {
            // Create a clone of self to capture for the closure
            let task_id_clone = task_id;
            
            // Return a future that produces a value
            async move {
                // In a real implementation, this would fetch the actual value
                // For now, we create a basic implementation that just uses a default value
                T::default()
            }
        });
        
        // Create a receiver work function from the provided function
        let receiver_work = Box::new(receiver);
        
        // Create a unique task ID
        let random_id = format!("task-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let id = I::from_string(&random_id).unwrap_or_else(|| {
            // Create a fallback ID string using a different format
            let fallback_id = format!("fallback-task-{}", uuid::Uuid::new_v4());
            I::from_string(&fallback_id).expect("Failed to create task ID")
        });
        
        // Create the emitting task
        TokioEmittingTask::new(
            id,
            TaskPriority::Normal,
            sender_work,
            sender_strategy,
            receiver_work,
            strategy,
            runtime,
            std::time::Duration::from_secs(30), // Default timeout
            0, // No retries
            false, // No tracing
            active_tasks
        )
    }
}

/// Implementation of ReceiverTask for AsyncTask
impl<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    ReceiverTask<T, C, E, I> for AsyncTask<T, I>
{
    type EmittingTaskType = TokioEmittingTask<T, C, E, I>;

    fn emit_events<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn() -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static
    {
        // Create a new TokioEmittingTask with this task as an emitter of events
        
        // Create a runtime handle
        let runtime = tokio::runtime::Handle::current();
        
        // Create active tasks registry
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        
        // Create sender strategy (Parallel with modest concurrency)
        let sender_strategy = sweet_async_api::task::builder::SenderStrategy::Parallel {
            workers: crate::task::builder::MinMax(1, 4),
            rate_limit: 10.0, // 10 events per second limit
        };
        
        // Create sender work function that produces values for the task
        let task_id = self.task_id();
        let sender_work = Box::new(move || {
            let task_id_clone = task_id;
            
            async move {
                // In a real implementation, this would emit a series of events
                // For now, return a default value
                T::default()
            }
        });
        
        // Create receiver work from provided function
        let receiver_work = Box::new(receiver);
        
        // Generate a unique task ID
        let random_id = format!("emitter-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let id = I::from_string(&random_id).unwrap_or_else(|| {
            // Create a fallback ID string using a different format
            let fallback_id = format!("fallback-emitter-{}", uuid::Uuid::new_v4());
            I::from_string(&fallback_id).expect("Failed to create task ID")
        });
        
        // Create the emitting task
        TokioEmittingTask::new(
            id,
            TaskPriority::Normal,
            sender_work,
            sender_strategy,
            receiver_work,
            strategy,
            runtime,
            std::time::Duration::from_secs(60), // Longer timeout for stream processing
            1, // Allow one retry
            true, // Enable tracing
            active_tasks
        )
    }
}

/// Extension methods for TokioEmittingTask to implement the full EmittingTask trait
impl<T: Clone + Send + 'static, C: Send + Clone + 'static, E: Send + 'static, I: TaskId>
    TokioEmittingTask<T, C, E, I>
{
    /// Wait for the task to complete and get the final result
    pub async fn await_completion(&self) -> Result<Vec<C>, OrchestratorError> {
        // In a real implementation, this would wait for all events to be processed
        // and return the aggregated result
        
        // Get the result from the receiver task
        let receiver_handle = {
            let handle = self.receiver_handle.lock().await;
            handle.clone()
        };
        
        if let Some(handle) = receiver_handle {
            match handle.await {
                Ok(result) => match result {
                    Ok(values) => Ok(values),
                    Err(e) => Err(OrchestratorError::TaskFailure(format!("Task failed: {:?}", e))),
                },
                Err(e) => Err(OrchestratorError::TaskFailure(format!("Join error: {:?}", e))),
            }
        } else {
            Err(OrchestratorError::TaskFailure("Task not started".to_string()))
        }
    }
    
    /// Get task ID
    pub fn task_id(&self) -> I {
        self.id
    }
    
    /// Get task priority
    pub fn priority(&self) -> TaskPriority {
        self.priority
    }
    
    /// Send an event to this task
    pub async fn send_event(&self, event: T) -> Result<(), AsyncTaskError> {
        self.event_sender.send(event).await
            .map_err(|_| AsyncTaskError::Failure("Failed to send event".to_string()))
    }
}

// Implement AsyncTask for TokioEmittingTask to satisfy the EmittingTask trait requirements
impl<T: Clone + Send + 'static, C: Send + Clone + 'static, E: Send + 'static, I: TaskId>
    AsyncTask<T, I> for TokioEmittingTask<T, C, E, I> 
{
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        // Delegate to the existing implementation in AsyncTask
        AsyncTask::<R, I>::to::<R, Task>()
    }

    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::task::emit::TokioEmittingTaskBuilder;
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        TokioEmittingTaskBuilder::<R, R, E, I>::new(runtime, active_tasks)
    }
}

// Enhanced implementation of EmittingTask for TokioEmittingTask
impl<T: Clone + Send + 'static, C: Send + Clone + 'static, E: Send + 'static, I: TaskId>
    EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
{
    type Final = TokioFinalEvent<T, C>;
    
    fn is_complete(&self) -> bool {
        // Try to check without blocking
        match self.final_result.try_lock() {
            Ok(result) => result.is_some(),
            Err(_) => false, // If locked, assume not complete
        }
    }
    
    fn cancel(&self) -> Result<(), OrchestratorError> {
        // Try to cancel without blocking
        let cancel_tx = Arc::clone(&self.cancel_tx);
        
        match cancel_tx.try_lock() {
            Ok(mut tx) => {
                if let Some(sender) = tx.take() {
                    let _ = sender.send(());
                }
                Ok(())
            }
            Err(_) => {
                // Can't get lock, spawn async cancellation
                tokio::spawn(async move {
                    let mut tx = cancel_tx.lock().await;
                    if let Some(sender) = tx.take() {
                        let _ = sender.send(());
                    }
                });
                
                Ok(())
            }
        }
    }
}
