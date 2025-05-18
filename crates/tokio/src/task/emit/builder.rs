//! Emitting task builder for Tokio implementation
//!
//! This module provides the implementation for creating and configuring stream-based tasks.

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{Stream, StreamExt};
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use uuid::Uuid;

use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, TaskId, TaskPriority
};
use sweet_async_api::task::builder::{
    AsyncTaskBuilder, ReceiverStrategy, SenderStrategy
};
use sweet_async_api::task::emit::{
    EmittingTask, EmittingTaskBuilder as ApiEmittingTaskBuilder
};

use crate::task::builder::AsyncTaskBuilder;
use super::collector::TokioEventCollector;
use super::event::{create_event_channel, TokioEventSender, TokioEventReceiver};

/// Tokio implementation of the EmittingTaskBuilder
#[derive(Clone)]
pub struct TokioEmittingTaskBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Base builder with common configuration
    base_builder: AsyncTaskBuilder<T, I>,
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
            base_builder: AsyncTaskBuilder::new(),
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
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> AsyncTaskBuilder 
    for TokioEmittingTaskBuilder<T, C, E, I> 
{
    fn name(self, name: &str) -> Self {
        Self {
            base_builder: self.base_builder.name(name),
            ..self
        }
    }

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

/// Tokio implementation of the SenderBuilder
pub struct TokioSenderBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Base builder with common configuration
    base_builder: AsyncTaskBuilder<T, I>,
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
        base_builder: AsyncTaskBuilder<T, I>,
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
    type ReceiverBuilder = TokioReceiverBuilder<T, C, E, I>;

    fn receiver<F>(self, receiver: F, strategy: ReceiverStrategy) -> Self::ReceiverBuilder
    where
        F: AsyncWork<C> + Send + 'static,
    {
        TokioReceiverBuilder::new(
            self.base_builder,
            self.runtime,
            self.active_tasks,
            self.priority,
            self.sender_work,
            self.sender_strategy,
            receiver,
            strategy,
        )
    }
}

/// Tokio implementation of ReceiverBuilder
pub struct TokioReceiverBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    /// Base builder with common configuration
    base_builder: AsyncTaskBuilder<T, I>,
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
    /// Event receiver work function
    receiver_work: Box<dyn AsyncWork<C> + Send + 'static>,
    /// Receiver strategy
    receiver_strategy: ReceiverStrategy,
    /// Type markers
    _marker: PhantomData<E>,
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    TokioReceiverBuilder<T, C, E, I> 
{
    /// Create a new receiver builder
    pub fn new<F>(
        base_builder: AsyncTaskBuilder<T, I>,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
        sender_strategy: SenderStrategy,
        receiver_work: F,
        receiver_strategy: ReceiverStrategy,
    ) -> Self
    where
        F: AsyncWork<C> + Send + 'static,
    {
        Self {
            base_builder,
            runtime,
            active_tasks,
            priority,
            sender_work,
            sender_strategy,
            receiver_work: Box::new(receiver_work),
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
    /// Collector for task results
    collector: Arc<Mutex<TokioEventCollector<T, C, I>>>,
    /// Cancellation sender
    cancel_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    /// Final result from task completion
    final_result: Arc<Mutex<Option<Result<Vec<C>, AsyncTaskError>>>>,
    /// Type markers
    _marker: PhantomData<E>,
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    TokioEmittingTask<T, C, E, I> 
{
    /// Create a new emitting task
    pub fn new(
        id: I,
        priority: TaskPriority,
        sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
        sender_strategy: SenderStrategy,
        receiver_work: Box<dyn AsyncWork<C> + Send + 'static>,
        receiver_strategy: ReceiverStrategy,
        runtime: Handle,
        timeout: Duration,
        retry_count: u8,
        tracing_enabled: bool,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    ) -> Self {
        // Create channel for events
        let (event_sender, event_receiver) = create_event_channel::<T>(100); // Use appropriate buffer size
        
        // Create collector for results
        let collector = Arc::new(Mutex::new(TokioEventCollector::<T, C, I>::new()));
        
        // Create cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();
        
        // Set up shared state
        let sender_handle = Arc::new(Mutex::new(None));
        let receiver_handle = Arc::new(Mutex::new(None));
        let event_sender_arc = Arc::new(event_sender);
        let final_result = Arc::new(Mutex::new(None));
        
        // Clone for closures
        let event_sender_clone = event_sender_arc.clone();
        let collector_clone = collector.clone();
        let final_result_clone = final_result.clone();
        let runtime_clone = runtime.clone();
        
        // Spawn sender task
        let sender_task = runtime.spawn(async move {
            let sender_result = sender_work.run().await;
            
            // Send the event
            if let Err(_) = event_sender_clone.send(sender_result).await {
                return Err(AsyncTaskError::Failure("Failed to send event".to_string()));
            }
            
            Ok(())
        });
        
        // Store sender handle
        let sender_handle_clone = sender_handle.clone();
        futures::executor::block_on(async {
            *sender_handle_clone.lock().await = Some(sender_task.clone());
        });
        
        // Spawn receiver task
        let receiver_task = runtime.spawn(async move {
            // Set up receiver from event stream
            let mut collector = collector_clone.lock().await;
            
            // Start processing with the stream
            collector.start_processing(event_receiver, receiver_strategy);
            
            // Wait for cancellation or completion
            tokio::select! {
                _ = cancel_rx => {
                    // Task was cancelled
                    return Err(AsyncTaskError::Cancelled);
                }
                result = async {
                    // Process the receiver work and collect results
                    let processed = receiver_work.run().await;
                    
                    // Wait for collector to finish
                    let all_results = collector.join().await;
                    
                    // Store final result
                    *final_result_clone.lock().await = Some(Ok(all_results.clone()));
                    
                    Ok(all_results)
                } => {
                    result
                }
            }
        });
        
        // Store receiver handle
        let receiver_handle_clone = receiver_handle.clone();
        futures::executor::block_on(async {
            *receiver_handle_clone.lock().await = Some(receiver_task.clone());
        });
        
        // Register the tasks with active_tasks
        futures::executor::block_on(async {
            let mut tasks = active_tasks.lock().await;
            tasks.push(sender_task);
            tasks.push(receiver_task);
        });
        
        Self {
            id,
            priority,
            sender_handle,
            receiver_handle,
            event_sender: event_sender_arc,
            collector,
            cancel_tx: Arc::new(Mutex::new(Some(cancel_tx))),
            final_result,
            _marker: PhantomData,
        }
    }
}

// Implement core AsyncTask methods first (these are required by EmittingTask trait)
impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    AsyncTask<T, I> for TokioEmittingTask<T, C, E, I>
{
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::task::spawn::builder::TokioSpawningTaskBuilder;
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        TokioSpawningTaskBuilder::<R, AsyncTaskError, I>::new(runtime, active_tasks)
    }

    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::task::emit::TokioEmittingTaskBuilder;
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        TokioEmittingTaskBuilder::<R, R, E, I>::new(runtime, active_tasks)
    }
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>
    EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
{
    // FinalEvent will be implemented in a follow-up PR
    type Final = crate::task::emit::TokioFinalEvent<T, C>;
    
    fn is_complete(&self) -> bool {
        // Task is complete if final_result is Some
        futures::executor::block_on(async {
            self.final_result.lock().unwrap().is_some()
        })
    }
    
    fn cancel(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        // Convert the cancellation future to a synchronous result
        futures::executor::block_on(async {
            let cancel_tx = self.cancel_tx.clone();
            
            let sender = {
                let mut tx = cancel_tx.lock().unwrap();
                tx.take()
            };
            
            if let Some(tx) = sender {
                let _ = tx.send(());
            }
            
            Ok(())
        })
    }
    
    // The following methods are part of the AsyncTask trait that EmittingTask derives from
    // They would be implemented fully in a complete PR
}

impl<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> 
    ApiReceiverBuilder<T, C, E, I> for TokioReceiverBuilder<T, C, E, I> 
{
    type Task = TokioEmittingTask<T, C, E, I>;
    
    fn run(self) -> Self::Task {
        // Generate a unique task ID using timestamp
        let random_id = format!("task-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let task_id = I::from_string(&random_id).unwrap_or_else(|| {
            // Create a fallback ID string using a different format if the first attempt failed
            let fallback_id = format!("fallback-task-{}", uuid::Uuid::new_v4());
            I::from_string(&fallback_id).expect("Failed to create task ID even with fallback")
        });
        
        TokioEmittingTask::new(
            task_id,
            self.priority,
            self.sender_work,
            self.sender_strategy,
            self.receiver_work,
            self.receiver_strategy,
            self.runtime.clone(),
            self.base_builder.get_timeout(),
            self.base_builder.get_retry_attempts(),
            self.base_builder.is_tracing_enabled(),
            self.active_tasks,
        )
    }
    
    fn await_result(
        self,
    ) -> impl Future<Output = (C, <Self::Task as EmittingTask<T, C, E, I>>::Final)> + Send {
        async move {
            let task = self.run();
            let result = task.await_completion().await.unwrap_or_else(|e| {
                eprintln!("Task failed: {:?}", e);
                Vec::new()
            });
            
            // For simplicity, we're using a dummy value for C
            // In a real implementation, this would come from the task execution
            let default_c = futures::executor::block_on(async {
                self.receiver_work.run().await
            });
            
            (default_c, result)
        }
    }
}
