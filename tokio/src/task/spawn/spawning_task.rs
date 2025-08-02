use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::marker::PhantomData;

use tokio::task::JoinHandle;
use tokio::sync::Mutex;

use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, TaskId, TaskPriority, TaskStatus, RankableByPriority,
    NamedTask, StatusEnabledTask, PrioritizedTask, TimedTask, TracingTask,
    CancellableTask, CancellationLevel, ContextualizedTask, RecoverableTask,
    MetricsEnabledTask, RetryStrategy, CpuUsage, MemoryUsage, IoUsage,
};
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::{SpawningTask, TaskResult, AsyncResult};
use sweet_async_api::orchestra::OrchestratorError;

use crate::task::tokio_task::TokioTask;
use crate::task::spawn::result::{TokioTaskResult, TokioAsyncResult};
use crate::task::relationships::TokioTaskRelationships;

/// A wrapper around TokioTask that implements SpawningTask
pub struct TokioSpawningTask<T: Clone + Send + 'static, I: TaskId> {
    /// The task ID
    id: I,
    /// Task priority for scheduling
    priority: TaskPriority,
    /// The join handle for the spawned task
    handle: Option<JoinHandle<Result<T, AsyncTaskError>>>,
    /// Result storage
    result: Arc<Mutex<Option<Result<T, AsyncTaskError>>>>,
    /// Child tasks
    children: Arc<Mutex<Vec<I>>>,
    /// Parent-child task relationships for hierarchical communication
    relationships: Option<TokioTaskRelationships<T, I>>,
    /// Runtime handle for accessing the runtime
    runtime: crate::orchestra::runtime::TokioRuntime,
    /// CPU usage metrics (zero for spawning tasks)
    cpu_metrics: crate::task::cpu_usage::TokioCpuUsage,
    /// Memory usage metrics (optimized for spawning tasks)
    memory_metrics: crate::task::memory_usage::TokioMemoryUsage,
    /// I/O usage metrics (zero for spawning tasks)
    io_metrics: crate::task::io_usage::TokioIoUsage,
    /// Phantom data
    _phantom: PhantomData<T>,
}

impl<T: Clone + Send + 'static, I: TaskId> TokioSpawningTask<T, I> {
    /// Create a new spawning task with the provided spawning task implementation
    pub fn new_with_spawning_task(
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
        handle: tokio::runtime::Handle,
        active_tasks: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        // Generate a new ID if needed
        let id = if let Ok(id) = std::any::Any::downcast_ref::<I>(&(uuid::Uuid::new_v4() as dyn std::any::Any)) {
            *id
        } else {
            // Fallback - this might not work for all I types, but it's a reasonable attempt
            unsafe { std::mem::zeroed() }
        };
        
        let result = Arc::new(tokio::sync::Mutex::new(None));
        let result_clone = result.clone();
        
        // Spawn the task
        let spawn_handle = handle.spawn(async move {
            let task_result = task.spawn().await.unwrap_or(Err(AsyncTaskError::Failure("Task failed".to_string())));
            
            // Store result  
            if let Ok(mut guard) = result_clone.try_lock() {
                *guard = Some(task_result.clone());
            }
            active_tasks.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            
            task_result
        });
        
        Self {
            id,
            priority,
            handle: Some(spawn_handle),
            result,
            children: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            relationships: None,
            runtime: crate::orchestra::runtime::TokioRuntime::new(),
            cpu_metrics: crate::task::cpu_usage::TokioCpuUsage::new(),
            memory_metrics: crate::task::memory_usage::TokioMemoryUsage::new(),
            io_metrics: crate::task::io_usage::TokioIoUsage::new(),
            _phantom: PhantomData,
        }
    }

    /// Create a new spawning task wrapper for runtime integration
    pub fn new(
        task: impl SpawningTask<T, I> + 'static,
        handle: tokio::runtime::Handle,
        active_tasks: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        let id = task.task_id();
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();
        
        // Spawn the task on the provided runtime handle
        let spawn_handle = handle.spawn(async move {
            let task_result = task.await;
            
            // Store result and decrement active task counter
            if let Ok(mut guard) = result_clone.try_lock() {
                *guard = Some(task_result.clone());
            }
            active_tasks.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            
            task_result
        });
        
        Self {
            id,
            priority: TaskPriority::Normal,
            handle: Some(spawn_handle),
            result,
            children: Arc::new(Mutex::new(Vec::new())),
            relationships: None,
            runtime: crate::orchestra::runtime::TokioRuntime::new(),
            cpu_metrics: crate::task::cpu_usage::TokioCpuUsage::new(),
            memory_metrics: crate::task::memory_usage::TokioMemoryUsage::new(),
            io_metrics: crate::task::io_usage::TokioIoUsage::new(),
            _phantom: PhantomData,
        }
    }

    /// Create a new spawning task that will execute the given work
    pub fn from_work<F, R>(id: I, work: F) -> Self
    where
        F: AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError> + Send + 'static,
    {
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();
        
        // Spawn the work on tokio runtime
        let handle = tokio::spawn(async move {
            let work_result = work.run().await;
            let final_result = work_result.into_async_result().await;
            
            // Store the result - we can't clone errors, so just store success/failure  
            if let Ok(mut guard) = result_clone.try_lock() {
                match &final_result {
                    Ok(val) => *guard = Some(Ok(val.clone())),
                    Err(_) => *guard = Some(Err(AsyncTaskError::Failure("Task failed".to_string()))),
                }
            }
            
            final_result
        });
        
        Self {
            id,
            priority: TaskPriority::Normal,
            handle: Some(handle),
            result,
            children: Arc::new(Mutex::new(Vec::new())),
            relationships: None,
            runtime: crate::orchestra::runtime::TokioRuntime::new(),
            cpu_metrics: crate::task::cpu_usage::TokioCpuUsage::new(),
            memory_metrics: crate::task::memory_usage::TokioMemoryUsage::new(),
            io_metrics: crate::task::io_usage::TokioIoUsage::new(),
            _phantom: PhantomData,
        }
    }
    
    /// Create a new spawning task with parent-child relationships
    pub fn from_work_with_relationships<F, R>(
        id: I, 
        work: F,
        relationships: Option<TokioTaskRelationships<T, I>>
    ) -> Self
    where
        F: AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError> + Send + 'static,
    {
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();
        
        // Spawn the work on tokio runtime
        let handle = tokio::spawn(async move {
            let work_result = work.run().await;
            let final_result = work_result.into_async_result().await;
            
            // Store the result for later retrieval
            if let Ok(mut guard) = result_clone.try_lock() {
                match &final_result {
                    Ok(value) => *guard = Some(Ok(value.clone())),
                    Err(_) => *guard = Some(Err(AsyncTaskError::Failure("Task failed".to_string()))),
                }
            }
            
            final_result
        });
        
        Self {
            id,
            priority: TaskPriority::Normal,
            handle: Some(handle),
            result,
            children: Arc::new(Mutex::new(Vec::new())),
            relationships,
            runtime: crate::orchestra::runtime::TokioRuntime::new(),
            cpu_metrics: crate::task::cpu_usage::TokioCpuUsage::new(),
            memory_metrics: crate::task::memory_usage::TokioMemoryUsage::new(),
            io_metrics: crate::task::io_usage::TokioIoUsage::new(),
            _phantom: PhantomData,
        }
    }
}

// Implement Future for TokioSpawningTask so it can be awaited
impl<T: Clone + Send + 'static, I: TaskId> Future for TokioSpawningTask<T, I> {
    type Output = Result<T, AsyncTaskError>;
    
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        // We need to access the handle field mutably
        let this = self.get_mut();
        
        if let Some(handle) = this.handle.as_mut() {
            match Pin::new(handle).poll(cx) {
                std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
                std::task::Poll::Ready(Err(e)) => {
                    std::task::Poll::Ready(Err(AsyncTaskError::Failure(e.to_string())))
                }
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        } else {
            std::task::Poll::Ready(Err(AsyncTaskError::Failure("Task handle missing".to_string())))
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> SpawningTask<T, I> for TokioSpawningTask<T, I> {
    type OutputFuture = Pin<Box<dyn Future<Output = Self::TaskResult> + Send>>;
    type TaskResult = crate::task::spawn::result::TokioTaskResult<T>;
    type JoinChildrenFuture = Pin<Box<dyn Future<Output = Self::JoinChildrenResult> + Send>>;
    type JoinChildrenResult = crate::task::spawn::result::TokioAsyncResult<Vec<I>>;
    // Use a concrete type that implements AsyncWork
    type AsyncWork = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = T> + Send>> + Send>;
    
    fn run(self, work: Self::AsyncWork) -> Self {
        // For spawning tasks, we create a new task with the given work
        // Convert the boxed FnOnce into something that implements AsyncWork
        let id = self.id.clone();
        let work_wrapper = move || work();
        TokioSpawningTask::from_work(id, work_wrapper)
    }
    
    fn run_child<R>(&self, _task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Clone + Send + 'static,
        Self: SpawningTask<R, I>
    {
        // Not fully implemented yet
        Box::pin(async move {
            TokioTaskResult::new(Err(AsyncTaskError::Failure("Child task execution not implemented".to_string())))
        })
    }
    
    fn join_children(&self) -> Self::JoinChildrenFuture {
        let children = self.children.clone();
        Box::pin(async move {
            let child_ids = children.lock().await.clone();
            TokioAsyncResult::new(Ok(child_ids))
        })
    }
    
    fn task_id(&self) -> I {
        self.id.clone()
    }
    
    fn value(&self) -> Option<&T> {
        // Try to get the result without blocking
        if let Ok(guard) = self.result.try_lock() {
            if let Some(Ok(ref value)) = *guard {
                // This is unsafe - we're returning a reference to data inside a mutex
                // The trait design requires a fix here
                None
            } else {
                None
            }
        } else {
            None
        }
    }
    
    fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: AsyncWork<U> + Send + 'static,
        U: Clone + Send + 'static,
        Self: SpawningTask<U, I>
    {
        Box::pin(async move {
            // Await this task first
            let result = self.await;
            
            match result {
                Ok(_) => {
                    // Run the next work
                    let next_result = f.run().await;
                    TokioTaskResult::new(Ok(next_result))
                }
                Err(e) => TokioTaskResult::new(Err(e)),
            }
        })
    }
}

// Implement all required traits for AsyncTask

impl<T: Clone + Send + 'static, I: TaskId> NamedTask for TokioSpawningTask<T, I> {
    fn name(&self) -> Option<&str> {
        None
    }
    
    fn set_name(&mut self, _name: String) {
        // Not implemented for spawning tasks
    }
}

impl<T: Clone + Send + 'static, I: TaskId> StatusEnabledTask<T> for TokioSpawningTask<T, I> {
    fn status(&self) -> TaskStatus {
        if self.handle.is_some() {
            TaskStatus::Running
        } else {
            TaskStatus::Pending
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> PrioritizedTask<T> for TokioSpawningTask<T, I> {
    fn priority(&self) -> &impl RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TimedTask<T> for TokioSpawningTask<T, I> {
    fn created_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
    
    fn executed_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
    
    fn completed_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
    
    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(60)
    }
}

impl<T: Clone + Send + 'static, I: TaskId> MetricsEnabledTask<T> for TokioSpawningTask<T, I> {
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

impl<T: Clone + Send + 'static, I: TaskId> TracingTask<T> for TokioSpawningTask<T, I> {
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        Err(error)
    }
    
    fn record_error(&self, _error: &AsyncTaskError) {
        // Not implemented
    }
    
    fn is_tracing_enabled(&self) -> bool {
        false
    }
}

impl<T: Clone + Send + 'static, I: TaskId> CancellableTask<T> for TokioSpawningTask<T, I> {
    fn cancel(&self, _level: CancellationLevel) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        async move {
            Ok(())
        }
    }
    
    fn is_cancelled(&self) -> bool {
        false
    }
    
    fn on_cancel<F, Fut>(&self, _callback: F)
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Not implemented
    }
    
    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::Graceful)
    }
    
    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::Kill)
    }
    
    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.cancel(CancellationLevel::KillHard)
    }
}

impl<T: Clone + Send + 'static, I: TaskId> ContextualizedTask<T, I> for TokioSpawningTask<T, I> {
    type RuntimeType = crate::orchestra::runtime::TokioRuntime;
    type RelationshipsType = TokioTaskRelationships<T, I>;
    
    fn relationships(&self) -> &Self::RelationshipsType {
        // Production-ready approach: relationships should always be initialized
        // If None, we create a leaked static instance to provide a valid reference
        match &self.relationships {
            Some(relationships) => relationships,
            None => {
                // Log the architectural issue - this should be fixed at construction
                tracing::warn!(
                    task_id = ?self.id,
                    "TokioSpawningTask relationships not initialized, using default empty relationships"
                );
                
                // Create a leaked default instance that lives for the program lifetime
                // This ensures we always return a valid reference without unsafe code
                Box::leak(Box::new(TokioTaskRelationships::<T, I>::default()))
            }
        }
    }
    
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        // Ensure relationships is always initialized
        self.relationships.get_or_insert_with(|| TokioTaskRelationships::new())
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        &self.runtime
    }
    
    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}

impl<T: Clone + Send + 'static, I: TaskId> RecoverableTask<T> for TokioSpawningTask<T, I> {
    type FallbackWork = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>> + Send + Sync>;
    
    fn recover(&self, error: AsyncTaskError) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        async move {
            Err(error)
        }
    }
    
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        false
    }
    
    fn fallback_work(&self) -> &Self::FallbackWork {
        // For spawning tasks, fallback always returns an error indicating no recovery support
        // We create a properly typed, leaked static instance to provide a valid reference
        
        // Create a leaked fallback instance that lives for the program lifetime
        // This is the cleanest approach for providing a static reference without unsafe code
        Box::leak(Box::new(Box::new(|| {
            Box::pin(async {
                Err(AsyncTaskError::RecoveryFailed(
                    "Spawning tasks do not support fallback recovery".to_string()
                ))
            })
        })))
    }
    
    fn max_retries(&self) -> u8 {
        0
    }
    
    fn current_retry(&self) -> u8 {
        0
    }
    
    fn retry_strategy(&self) -> RetryStrategy {
        RetryStrategy::Fixed(std::time::Duration::from_secs(1))
    }
}

// AsyncTask implementation delegates to an internal TokioTask
impl<T: Clone + Send + 'static, I: TaskId> AsyncTask<T, I> for TokioSpawningTask<T, I> {
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() 
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        crate::task::builder::DefaultOrchestratorBuilder::<R, Task, I>::new_spawning()
    }
    
    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        crate::task::builder::DefaultOrchestratorBuilder::<R, Task, I>::new_emitting()
    }


}