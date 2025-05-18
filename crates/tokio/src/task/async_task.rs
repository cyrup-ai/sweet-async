use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::path::PathBuf;

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::task::{
    AsyncTask as ApiAsyncTask, AsyncTaskError, CancellableTask, CancellationLevel, 
    ContextualizedTask, CpuUsage, IoUsage, MemoryUsage, MetricsEnabledTask, 
    PrioritizedTask, RecoverableTask, StatusEnabledTask, TaskId, TaskPriority, 
    TaskStatus, TimedTask, TracingTask
};
// use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::{SpawningTask, TaskResult};

use tokio::runtime::Handle;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::runtime::safe_blocking;

/// Task metrics implementation for Tokio tasks
#[derive(Debug)]
struct TaskMetrics {
    cpu_time: Arc<Mutex<Duration>>,
    memory_current: Arc<Mutex<u64>>,
    memory_peak: Arc<Mutex<u64>>,
    bytes_read: Arc<Mutex<u64>>,
    bytes_written: Arc<Mutex<u64>>,
}

impl TaskMetrics {
    fn new() -> Self {
        Self {
            cpu_time: Arc::new(Mutex::new(Duration::from_secs(0))),
            memory_current: Arc::new(Mutex::new(0)),
            memory_peak: Arc::new(Mutex::new(0)),
            bytes_read: Arc::new(Mutex::new(0)),
            bytes_written: Arc::new(Mutex::new(0)),
        }
    }
    
    async fn update_cpu_time(&self, additional: Duration) {
        let mut cpu_time = self.cpu_time.lock().await;
        *cpu_time += additional;
    }
    
    async fn update_memory(&self, current: u64) {
        let mut memory_current = self.memory_current.lock().await;
        *memory_current = current;
        
        let mut memory_peak = self.memory_peak.lock().await;
        if current > *memory_peak {
            *memory_peak = current;
        }
    }
    
    async fn update_bytes_read(&self, additional: u64) {
        let mut bytes_read = self.bytes_read.lock().await;
        *bytes_read += additional;
    }
    
    async fn update_bytes_written(&self, additional: u64) {
        let mut bytes_written = self.bytes_written.lock().await;
        *bytes_written += additional;
    }
}

impl Clone for TaskMetrics {
    fn clone(&self) -> Self {
        Self {
            cpu_time: self.cpu_time.clone(),
            memory_current: self.memory_current.clone(),
            memory_peak: self.memory_peak.clone(),
            bytes_read: self.bytes_read.clone(),
            bytes_written: self.bytes_written.clone(),
        }
    }
}

impl CpuUsage for TaskMetrics {
    fn cpu_time(&self) -> Duration {
        safe_blocking(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.cpu_time.lock().await
            })
        })
    }
    
    fn utilization(&self) -> f64 {
        // For accurate CPU utilization, we would need more sophisticated tracking
        // This is a simplified implementation
        let cpu_time = self.cpu_time();
        if cpu_time.as_secs() == 0 {
            return 0.0;
        }
        
        // Estimate based on wall time vs CPU time
        // This is a placeholder implementation
        0.5
    }
}

impl MemoryUsage for TaskMetrics {
    fn current_bytes(&self) -> u64 {
        safe_blocking(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.memory_current.lock().await
            })
        })
    }
    
    fn peak_bytes(&self) -> u64 {
        safe_blocking(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.memory_peak.lock().await
            })
        })
    }
}

impl IoUsage for TaskMetrics {
    fn bytes_read(&self) -> u64 {
        safe_blocking(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.bytes_read.lock().await
            })
        })
    }
    
    fn bytes_written(&self) -> u64 {
        safe_blocking(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.bytes_written.lock().await
            })
        })
    }
}

/// A complete Tokio-based task implementation
#[derive(Debug)]
pub struct AsyncTask<T: Clone + Send + 'static, I: TaskId> {
    // Task identifier
    id: I,
    // Task priority
    priority: TaskPriority,
    // Task execution handle
    handle: Arc<Mutex<Option<JoinHandle<Result<T, AsyncTaskError>>>>>,
    // Cancellation sender
    cancel_tx: Arc<Mutex<Option<oneshot::Sender<CancellationLevel>>>>,
    // Task status
    status: Arc<Mutex<TaskStatus>>,
    // Task result (if available)
    result: Arc<Mutex<Option<Result<T, AsyncTaskError>>>>,
    // Task creation time
    created_time: SystemTime,
    // Task execution start time
    start_time: Arc<Mutex<Option<SystemTime>>>,
    // Task completion time
    end_time: Arc<Mutex<Option<SystemTime>>>,
    // Task timeout
    timeout: Duration,
    // Tokio runtime handle
    runtime: Handle,
    // Active tasks registry for the runtime
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    // Task metrics
    metrics: TaskMetrics,
    // Fallback value
    fallback: Arc<Mutex<Option<T>>>,
    // Retry count
    retry_count: u8,
    // Current retry
    current_retry: Arc<Mutex<u8>>,
    // Cancellation callbacks
    cancel_callbacks: Arc<Mutex<Vec<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>>>,
    // Tracing enabled
    tracing_enabled: bool,
    // Child tasks
    child_tasks: Arc<Mutex<Vec<Box<dyn std::any::Any + Send + Sync>>>>,
    // Parent task
    parent: Arc<Mutex<Option<Box<dyn std::any::Any + Send + Sync>>>>,
    // Current working directory
    cwd: PathBuf,
    // Task name
    name: Arc<Mutex<Option<String>>>,
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTask<T, I> {
    /// Create a new AsyncTask directly with an ID and priority
    pub fn new(
        id: I,
        priority: TaskPriority,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    ) -> Self {
        let (cancel_tx, _) = oneshot::channel();
        
        // Create the new task
        Self {
            id,
            priority,
            handle: Arc::new(Mutex::new(None)),
            cancel_tx: Arc::new(Mutex::new(Some(cancel_tx))),
            status: Arc::new(Mutex::new(TaskStatus::Pending)),
            result: Arc::new(Mutex::new(None)),
            created_time: SystemTime::now(),
            start_time: Arc::new(Mutex::new(None)),
            end_time: Arc::new(Mutex::new(None)),
            timeout: Duration::from_secs(0), // Default - no timeout
            runtime,
            active_tasks,
            metrics: TaskMetrics::new(),
            fallback: Arc::new(Mutex::new(None)),
            retry_count: 0,
            current_retry: Arc::new(Mutex::new(0)),
            cancel_callbacks: Arc::new(Mutex::new(Vec::new())),
            tracing_enabled: false,
            child_tasks: Arc::new(Mutex::new(Vec::new())),
            parent: Arc::new(Mutex::new(None)),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            name: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Create a new AsyncTask from an existing SpawningTask
    pub fn from_spawning_task(
        task: impl SpawningTask<T, I> + 'static,
        priority: TaskPriority,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    ) -> Self {
        let id = task.task_id();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        
        let task_future = task.into_future();
        
        // Create the new task
        let new_task = Self {
            id,
            priority,
            handle: Arc::new(Mutex::new(None)),
            cancel_tx: Arc::new(Mutex::new(Some(cancel_tx))),
            status: Arc::new(Mutex::new(TaskStatus::Pending)),
            result: Arc::new(Mutex::new(None)),
            created_time: SystemTime::now(),
            start_time: Arc::new(Mutex::new(None)),
            end_time: Arc::new(Mutex::new(None)),
            timeout: Duration::from_secs(0), // Default - no timeout
            runtime: runtime.clone(),
            active_tasks,
            metrics: TaskMetrics::new(),
            fallback: Arc::new(Mutex::new(None)),
            retry_count: 0,
            current_retry: Arc::new(Mutex::new(0)),
            cancel_callbacks: Arc::new(Mutex::new(Vec::new())),
            tracing_enabled: false,
            child_tasks: Arc::new(Mutex::new(Vec::new())),
            parent: Arc::new(Mutex::new(None)),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            name: Arc::new(Mutex::new(None)),
        };
        
        // Create a clone of all the necessary state for the task
        let handle_cl = new_task.handle.clone();
        let status_cl = new_task.status.clone();
        let result_cl = new_task.result.clone();
        let start_time_cl = new_task.start_time.clone();
        let end_time_cl = new_task.end_time.clone();
        let timeout_dur = new_task.timeout;
        let fallback_cl = new_task.fallback.clone();
        let retry_count = new_task.retry_count;
        let current_retry_cl = new_task.current_retry.clone();
        let cancel_callbacks_cl = new_task.cancel_callbacks.clone();
        let tracing_enabled = new_task.tracing_enabled;
        let active_tasks_cl = new_task.active_tasks.clone();
        let metrics_cl = new_task.metrics.clone();
        
        // Spawn the task on the Tokio runtime
        let task_handle = runtime.spawn(async move {
            // Monitor cancellation
            let cancel_fut = async {
                if let Ok(level) = cancel_rx.await {
                    Some(level)
                } else {
                    None
                }
            };
            
            // Track operation start time
            let start = tokio::time::Instant::now();
            
            // Update task status to Running
            *status_cl.lock().await = TaskStatus::Running;
            // Record start time
            *start_time_cl.lock().await = Some(SystemTime::now());
            
            // Create a future that runs the task
            let task_fut = async {
                task_future.await
            };
            
            // Execute the task with timeout if specified
            let task_result = if timeout_dur > Duration::from_secs(0) {
                match timeout(timeout_dur, task_fut).await {
                    Ok(result) => result,
                    Err(_) => Err(AsyncTaskError::Timeout(timeout_dur)),
                }
            } else {
                task_fut.await
            };
            
            // Process the result
            let result = match task_result {
                Ok(value) => {
                    // Task completed successfully
                    *status_cl.lock().await = TaskStatus::Completed;
                    Ok(value)
                }
                Err(error) => {
                    // Check if we should retry
                    let current_retry = {
                        let mut retry = current_retry_cl.lock().await;
                        *retry += 1;
                        *retry
                    };
                    
                    if current_retry <= retry_count {
                        // TODO: Implement retry logic
                        // For now, just return the error
                        *status_cl.lock().await = TaskStatus::Pending;
                        Err(error)
                    } else {
                        // Check for fallback
                        let fallback = {
                            let fallback = fallback_cl.lock().await;
                            fallback.clone()
                        };
                        
                        if let Some(value) = fallback {
                            *status_cl.lock().await = TaskStatus::Completed;
                            Ok(value)
                        } else {
                            *status_cl.lock().await = TaskStatus::Cancelled;
                            Err(error)
                        }
                    }
                }
            };
            
            // Update metrics
            let elapsed = start.elapsed();
            metrics_cl.update_cpu_time(elapsed).await;
            
            // Record end time
            *end_time_cl.lock().await = Some(SystemTime::now());
            
            // Store the result
            *result_cl.lock().await = Some(result.clone());
            
            // Execute cancellation callbacks if task was cancelled
            if matches!(*status_cl.lock().await, TaskStatus::Cancelled) {
                let callbacks = {
                    let callbacks = cancel_callbacks_cl.lock().await;
                    callbacks.iter().map(|f| f()).collect::<Vec<_>>()
                };
                
                for callback in callbacks {
                    let _ = callback.await;
                }
            }
            
            result
        });
        
        // Register the task handle
        futures::executor::block_on(async {
            *handle_cl.lock().await = Some(task_handle.clone());
            active_tasks_cl.lock().await.push(task_handle);
        });
        
        new_task
    }
    
    /// Set a timeout for the task
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }
    
    /// Set a fallback value for the task
    pub fn with_fallback(self, value: T) -> Self {
        futures::executor::block_on(async {
            let mut fallback = self.fallback.lock().await;
            *fallback = Some(value);
        });
        self
    }
    
    /// Set the retry count for the task
    pub fn with_retry(mut self, count: u8) -> Self {
        self.retry_count = count;
        self
    }
    
    /// Enable or disable tracing for the task
    pub fn with_tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }
    
    /// Set the task's parent
    pub fn with_parent(self, parent: Box<dyn std::any::Any + Send + Sync>) -> Self {
        futures::executor::block_on(async {
            let mut parent_lock = self.parent.lock().await;
            *parent_lock = Some(parent);
        });
        self
    }
    
    /// Add a child task
    pub fn add_child(&self, child: Box<dyn std::any::Any + Send + Sync>) {
        futures::executor::block_on(async {
            let mut children = self.child_tasks.lock().await;
            children.push(child);
        });
    }
    
    /// Set a name for the task
    pub fn with_name(self, name: String) -> Self {
        futures::executor::block_on(async {
            let mut task_name = self.name.lock().await;
            *task_name = Some(name);
        });
        self
    }
    
    /// Get the task name if set
    pub fn name(&self) -> Option<String> {
        futures::executor::block_on(async {
            let name = self.name.lock().await;
            name.clone()
        })
    }
    
    /// Attach a future to this task
    pub fn with_future(
        self, 
        future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>>
    ) -> Self {
        // Create clones of all the necessary state for the task
        let handle_cl = self.handle.clone();
        let status_cl = self.status.clone();
        let result_cl = self.result.clone();
        let start_time_cl = self.start_time.clone();
        let end_time_cl = self.end_time.clone();
        let timeout_dur = self.timeout;
        let fallback_cl = self.fallback.clone();
        let retry_count = self.retry_count;
        let current_retry_cl = self.current_retry.clone();
        let cancel_callbacks_cl = self.cancel_callbacks.clone();
        let tracing_enabled = self.tracing_enabled;
        let active_tasks_cl = self.active_tasks.clone();
        let metrics_cl = self.metrics.clone();
        let cancel_cl = self.cancel_tx.clone();
        let runtime = self.runtime.clone();
        
        // Create the cancellation receiver
        let (cancel_tx, cancel_rx) = oneshot::channel();
        
        // Replace the cancel_tx with the new one
        futures::executor::block_on(async {
            let mut tx = cancel_cl.lock().await;
            *tx = Some(cancel_tx);
        });
        
        // Spawn the task on the Tokio runtime
        let task_handle = runtime.spawn(async move {
            // Monitor cancellation
            let cancel_fut = async {
                if let Ok(level) = cancel_rx.await {
                    Some(level)
                } else {
                    None
                }
            };
            
            // Track operation start time
            let start = tokio::time::Instant::now();
            
            // Update task status to Running
            *status_cl.lock().await = TaskStatus::Running;
            // Record start time
            *start_time_cl.lock().await = Some(SystemTime::now());
            
            // Create a future that runs the task
            let task_fut = async {
                future.await
            };
            
            // Run the task and cancellation in a select
            let task_result = tokio::select! {
                _ = cancel_fut => {
                    Err(AsyncTaskError::Cancelled)
                },
                result = async {
                    // Execute the task with timeout if specified
                    if timeout_dur > Duration::from_secs(0) {
                        match timeout(timeout_dur, task_fut).await {
                            Ok(result) => result,
                            Err(_) => Err(AsyncTaskError::Timeout(timeout_dur)),
                        }
                    } else {
                        task_fut.await
                    }
                } => result,
            };
            
            // Process the result
            let result = match task_result {
                Ok(value) => {
                    // Task completed successfully
                    *status_cl.lock().await = TaskStatus::Completed;
                    Ok(value)
                }
                Err(error) => {
                    // Check if we should retry
                    let current_retry = {
                        let mut retry = current_retry_cl.lock().await;
                        *retry += 1;
                        *retry
                    };
                    
                    if current_retry <= retry_count {
                        // TODO: Implement retry logic
                        // For now, just return the error
                        *status_cl.lock().await = TaskStatus::Failed;
                        Err(error)
                    } else {
                        // Check for fallback
                        let fallback = {
                            let fallback = fallback_cl.lock().await;
                            fallback.clone()
                        };
                        
                        if let Some(value) = fallback {
                            *status_cl.lock().await = TaskStatus::Completed;
                            Ok(value)
                        } else {
                            match error {
                                AsyncTaskError::Cancelled => {
                                    *status_cl.lock().await = TaskStatus::Cancelled;
                                },
                                _ => {
                                    *status_cl.lock().await = TaskStatus::Failed;
                                }
                            }
                            Err(error)
                        }
                    }
                }
            };
            
            // Update metrics
            let elapsed = start.elapsed();
            metrics_cl.update_cpu_time(elapsed).await;
            
            // Record end time
            *end_time_cl.lock().await = Some(SystemTime::now());
            
            // Store the result
            *result_cl.lock().await = Some(result.clone());
            
            // Execute cancellation callbacks if task was cancelled
            if matches!(*status_cl.lock().await, TaskStatus::Cancelled) {
                let callbacks = {
                    let callbacks = cancel_callbacks_cl.lock().await;
                    callbacks.iter().map(|f| f()).collect::<Vec<_>>()
                };
                
                for callback in callbacks {
                    let _ = callback.await;
                }
            }
            
            result
        });
        
        // Register the task handle
        futures::executor::block_on(async {
            *handle_cl.lock().await = Some(task_handle.clone());
            active_tasks_cl.lock().await.push(task_handle);
        });
        
        self
    }
}

impl<T: Clone + Send + 'static, I: TaskId> Clone for AsyncTask<T, I> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            priority: self.priority,
            handle: self.handle.clone(),
            cancel_tx: self.cancel_tx.clone(),
            status: self.status.clone(),
            result: self.result.clone(),
            created_time: self.created_time,
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            timeout: self.timeout,
            runtime: self.runtime.clone(),
            active_tasks: self.active_tasks.clone(),
            metrics: self.metrics.clone(),
            fallback: self.fallback.clone(),
            retry_count: self.retry_count,
            current_retry: self.current_retry.clone(),
            cancel_callbacks: self.cancel_callbacks.clone(),
            tracing_enabled: self.tracing_enabled,
            child_tasks: self.child_tasks.clone(),
            parent: self.parent.clone(),
            cwd: self.cwd.clone(),
            name: self.name.clone(),
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> Future for AsyncTask<T, I> {
    type Output = Result<T, AsyncTaskError>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = Pin::into_inner(self);
        let future = this.runtime.spawn(async {
            // Get the task result
            let handle = {
                let handle = this.handle.lock().await;
                handle.clone()
            };
            
            if let Some(handle) = handle {
                match handle.await {
                    Ok(result) => result,
                    Err(e) => Err(AsyncTaskError::Failure(format!("Task join error: {}", e))),
                }
            } else {
                Err(AsyncTaskError::Failure("Task not started".to_string()))
            }
        });
        
        // Poll the future
        Future::poll(Box::pin(future), cx)
    }
}

impl<T: Clone + Send + 'static, I: TaskId> CancellableTask<T> for AsyncTask<T, I> {
    async fn cancel(&self, level: CancellationLevel) -> Result<(), OrchestratorError> {
        // Update status
        {
            let mut status = self.status.lock().await;
            *status = TaskStatus::PendingCancellation;
        }
        
        // Send cancellation signal
        let cancel_tx = {
            let mut cancel_tx = self.cancel_tx.lock().await;
            cancel_tx.take()
        };
        
        if let Some(tx) = cancel_tx {
            let _ = tx.send(level);
        }
        
        // If KillHard, abort the task
        if matches!(level, CancellationLevel::KillHard) {
            let handle = {
                let mut handle = self.handle.lock().await;
                handle.take()
            };
            
            if let Some(handle) = handle {
                handle.abort();
            }
        }
        
        // Update status
        {
            let mut status = self.status.lock().await;
            *status = TaskStatus::Cancelled;
        }
        
        // Execute cancellation callbacks
        let callbacks = {
            let callbacks = self.cancel_callbacks.lock().await;
            callbacks.iter().map(|f| f()).collect::<Vec<_>>()
        };
        
        for callback in callbacks {
            let _ = callback.await;
        }
        
        Ok(())
    }
    
    fn is_cancelled(&self) -> bool {
        futures::executor::block_on(async {
            let status = self.status.lock().await;
            matches!(*status, TaskStatus::Cancelled | TaskStatus::PendingCancellation)
        })
    }
    
    fn on_cancel<F, Fut>(&self, callback: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        futures::executor::block_on(async {
            let mut callbacks = self.cancel_callbacks.lock().await;
            callbacks.push(Box::new(move || Box::pin(callback())));
        });
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TimedTask<T> for AsyncTask<T, I> {
    fn created_timestamp(&self) -> SystemTime {
        self.created_time
    }
    
    fn executed_timestamp(&self) -> SystemTime {
        futures::executor::block_on(async {
            self.start_time.lock().await.unwrap_or(self.created_time)
        })
    }
    
    fn completed_timestamp(&self) -> SystemTime {
        futures::executor::block_on(async {
            self.end_time.lock().await.unwrap_or(self.created_time)
        })
    }
    
    fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TracingTask<T> for AsyncTask<T, I> {
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // In a real implementation, this would integrate with tracing systems
        if self.tracing_enabled {
            tracing::error!("Task error: {:?}", error);
        }
        
        Err(error)
    }
    
    fn record_error(&self, error: &AsyncTaskError) {
        if self.tracing_enabled {
            tracing::error!("Recording error: {:?}", error);
        }
    }
    
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled
    }
}

impl<T: Clone + Send + 'static, I: TaskId> ContextualizedTask<T, I> for AsyncTask<T, I> {
    type RuntimeType = super::super::runtime::TokioRuntime;

    fn child_tasks(&self) -> Vec<T> {
        // In a real implementation, this would return the actual child tasks
        Vec::new()
    }
    
    fn parent(&self) -> Option<T> {
        // In a real implementation, this would return the parent task
        None
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        // Because AsyncTask doesn't currently store a reference to the TokioRuntime
        // we need to panic with an informative message instead of returning a reference
        panic!("ContextualizedTask::runtime is not directly available. Use AsyncTask::runtime_handle() instead.")
    }
    
    fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }
}

impl<T: Clone + Send + 'static, I: TaskId> StatusEnabledTask<T> for AsyncTask<T, I> {
    fn status(&self) -> TaskStatus {
        futures::executor::block_on(async {
            let status = self.status.lock().await;
            match *status {
                TaskStatus::Pending => TaskStatus::Pending,
                TaskStatus::Running => TaskStatus::Running,
                TaskStatus::Completed => TaskStatus::Completed,
                TaskStatus::PendingCancellation => TaskStatus::PendingCancellation,
                TaskStatus::Cancelled => TaskStatus::Cancelled,
                TaskStatus::Failed => TaskStatus::Failed,
            }
        })
    }
}

impl<T: Clone + Send + 'static, I: TaskId> RecoverableTask<T> for AsyncTask<T, I> {
    fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Check if we have a fallback value
        futures::executor::block_on(async {
            let fallback = self.fallback.lock().await;
            if let Some(value) = fallback.clone() {
                Ok(value)
            } else {
                Err(error)
            }
        })
    }
    
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        // Check if we have a fallback value
        futures::executor::block_on(async {
            let fallback = self.fallback.lock().await;
            fallback.is_some()
        })
    }
    
    fn fallback_value(&self) -> Option<T> {
        futures::executor::block_on(async {
            let fallback = self.fallback.lock().await;
            fallback.clone()
        })
    }
}

impl<T: Clone + Send + 'static, I: TaskId> PrioritizedTask<T> for AsyncTask<T, I> {
    fn priority(&self) -> &impl sweet_async_api::task::RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, I: TaskId> MetricsEnabledTask<T> for AsyncTask<T, I> {
    type Cpu = TaskMetrics;
    type Memory = TaskMetrics;
    type Io = TaskMetrics;
    
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

impl<T: Clone + Send + 'static, I: TaskId> ApiAsyncTask<T, I> for AsyncTask<T, I> {
    fn to<R: Send + 'static, Task: ApiAsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_spawning()
    }

    fn emits<R: Send + 'static, Task: ApiAsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_emitting()
    }
}

impl<T: Clone + Send + 'static, I: TaskId> SpawningTask<T, I> for AsyncTask<T, I> {
    type TaskResult = Result<T, AsyncTaskError>;
    type OutputFuture = Pin<Box<dyn Future<Output = Self::TaskResult> + Send>>;
    type JoinChildrenFuture = Pin<Box<dyn Future<Output = Self::JoinChildrenResult> + Send + 'static>>;
    type JoinChildrenResult = Result<Vec<I>, AsyncTaskError>;
    type AsyncWork = Box<dyn Fn() -> T + Send + 'static>;
    
    fn task_id(&self) -> I {
        self.id
    }

    fn run(self, _work: Self::AsyncWork) -> Self {
        // Implementation would execute the work
        // This is a simplification - to be properly implemented in future PRs
        self
    }

    fn run_child<R>(&self, _task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Send + 'static,
        Self: SpawningTask<R, I>
    {
        // Implementation would create and execute a child task
        // This is a placeholder implementation
        Box::pin(async { Err(AsyncTaskError::Failure("Not implemented yet".to_string())) })
    }

    fn join_children(&self) -> Self::JoinChildrenFuture {
        // Implementation would wait for all child tasks to complete
        // This is a placeholder implementation
        Box::pin(async { Err(AsyncTaskError::Failure("Not implemented yet".to_string())) })
    }

    fn value(&self) -> Option<&T> {
        // Implementation would return the current value if available
        None
    }

    fn chain<U, F>(self, _f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: sweet_async_api::task::builder::AsyncWork<U> + Send + 'static,
        U: Send + 'static,
        Self: SpawningTask<U, I>
    {
        // Implementation would chain the next operation
        // This is a placeholder implementation
        Box::pin(async { Err(AsyncTaskError::Failure("Not implemented yet".to_string())) })
    }
}
