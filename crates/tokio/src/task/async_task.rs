use std::future::Future;
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::task::{
    AsyncTask as ApiAsyncTask, AsyncTaskError, CancellableTask, CancellationLevel,
    ContextualizedTask, CpuUsage, IoUsage, MemoryUsage, MetricsEnabledTask, PrioritizedTask,
    RecoverableTask, StatusEnabledTask, TaskId, TaskPriority, TaskStatus, TimedTask, TracingTask,
    TaskRelationships,
};
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::SpawningTask;

use tokio::runtime::Handle;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot, OnceCell};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::runtime::safe_blocking;

/// Messages sent from spawned task back to AsyncTask
enum TaskMessage<T> {
    StatusUpdate(TaskStatus),
    StartTime(SystemTime),
    EndTime(SystemTime),
    Result(Result<T, AsyncTaskError>),
    MetricsUpdate { cpu_time: Duration },
}

/// Task metrics implementation for Tokio tasks
/// Uses atomic values to avoid blocking when reading metrics
pub struct TaskMetrics {
    cpu_time_nanos: AtomicU64,
    memory_current: AtomicU64,
    memory_peak: AtomicU64,
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,
}

impl TaskMetrics {
    pub fn new() -> Self {
        Self {
            cpu_time_nanos: AtomicU64::new(0),
            memory_current: AtomicU64::new(0),
            memory_peak: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
        }
    }

    pub fn update_cpu_time(&self, additional: Duration) {
        let additional_nanos = additional.as_nanos() as u64;
        self.cpu_time_nanos.fetch_add(additional_nanos, Ordering::Relaxed);
    }

    pub fn update_memory(&self, current: u64) {
        self.memory_current.store(current, Ordering::Relaxed);
        
        // Update peak if necessary
        let mut peak = self.memory_peak.load(Ordering::Relaxed);
        while current > peak {
            match self.memory_peak.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }
    }

    pub fn update_bytes_read(&self, additional: u64) {
        self.bytes_read.fetch_add(additional, Ordering::Relaxed);
    }

    pub fn update_bytes_written(&self, additional: u64) {
        self.bytes_written.fetch_add(additional, Ordering::Relaxed);
    }
}

impl Clone for TaskMetrics {
    fn clone(&self) -> Self {
        Self {
            cpu_time_nanos: AtomicU64::new(self.cpu_time_nanos.load(Ordering::Relaxed)),
            memory_current: AtomicU64::new(self.memory_current.load(Ordering::Relaxed)),
            memory_peak: AtomicU64::new(self.memory_peak.load(Ordering::Relaxed)),
            bytes_read: AtomicU64::new(self.bytes_read.load(Ordering::Relaxed)),
            bytes_written: AtomicU64::new(self.bytes_written.load(Ordering::Relaxed)),
        }
    }
}

impl CpuUsage for TaskMetrics {
    fn cpu_time(&self) -> Duration {
        let nanos = self.cpu_time_nanos.load(Ordering::Relaxed);
        Duration::from_nanos(nanos)
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

    fn user_time(&self) -> Duration {
        // Precise user/system split not available; return total cpu_time as
        // user_time for now.
        self.cpu_time()
    }

    fn system_time(&self) -> Duration {
        // Without kernel vs user accounting, report zero system time.
        Duration::from_secs(0)
    }
}

impl MemoryUsage for TaskMetrics {
    fn current_bytes(&self) -> u64 {
        self.memory_current.load(Ordering::Relaxed)
    }

    fn peak_bytes(&self) -> u64 {
        self.memory_peak.load(Ordering::Relaxed)
    }

    fn allocation_count(&self) -> u64 {
        // Allocation tracking not yet instrumented – return 0.
        0
    }

    fn allocation_rate(&self) -> f64 {
        // Without allocation timestamps we cannot compute rate – return 0.0.
        0.0
    }
}

impl IoUsage for TaskMetrics {
    fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    fn read_operations(&self) -> u64 {
        0
    }

    fn write_operations(&self) -> u64 {
        0
    }

    fn read_latency(&self) -> Duration {
        Duration::from_secs(0)
    }

    fn write_latency(&self) -> Duration {
        Duration::from_secs(0)
    }

    fn operations_per_second(&self) -> f64 {
        0.0
    }

    fn io_wait_time(&self) -> Duration {
        Duration::from_secs(0)
    }
}

/// Helper to convert TaskStatus to u8 for atomic storage
fn status_to_u8(status: &TaskStatus) -> u8 {
    match status {
        TaskStatus::Pending => 0,
        TaskStatus::Running => 1,
        TaskStatus::PendingCancellation => 2,
        TaskStatus::Cancelled => 3,
        TaskStatus::Completed => 4,
    }
}

/// Helper to convert u8 to TaskStatus
fn u8_to_status(val: u8) -> TaskStatus {
    match val {
        0 => TaskStatus::Pending,
        1 => TaskStatus::Running,
        2 => TaskStatus::PendingCancellation,
        3 => TaskStatus::Cancelled,
        4 => TaskStatus::Completed,
        _ => TaskStatus::Pending,
    }
}

/// Type alias for the Tokio implementation of AsyncTask
pub type TokioAsyncTask<T, I> = AsyncTask<T, I>;

/// A complete Tokio-based task implementation
pub struct AsyncTask<T: Clone + Send + 'static, I: TaskId> {
    // Task identifier
    id: I,
    // Task priority
    priority: TaskPriority,
    // Task execution handle
    handle: Option<JoinHandle<Result<T, AsyncTaskError>>>,
    // Cancellation sender
    cancel_tx: Option<oneshot::Sender<CancellationLevel>>,
    // Channel receiver for task updates
    update_rx: Option<mpsc::UnboundedReceiver<TaskMessage<T>>>,
    // Task status (atomic for sync access)
    atomic_status: AtomicU8,
    // Task result (if available)
    result: Option<Result<T, AsyncTaskError>>,
    // Successful value storage (for value() method)
    success_value: OnceCell<T>,
    // Atomic result tracking for sync access
    atomic_result_available: AtomicBool,
    atomic_result_success: AtomicBool,
    // Task creation time
    created_time: SystemTime,
    // Task execution start time (atomic for sync access)
    atomic_start_time: AtomicU64, // nanos since UNIX_EPOCH
    // Task completion time (atomic for sync access)
    atomic_end_time: AtomicU64, // nanos since UNIX_EPOCH
    // Task timeout
    timeout: Duration,
    // Tokio runtime handle
    runtime: Handle,
    // Active tasks registry for the runtime - THIS IS THE ONLY ARC WE NEED
    active_tasks: Arc<AtomicUsize>,
    // Task metrics
    metrics: TaskMetrics,
    // Fallback value
    fallback: Option<T>,
    // Retry count
    retry_count: u8,
    // Current retry
    current_retry: AtomicU8,
    // Cancellation callbacks
    cancel_callbacks:
        Vec<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>,
    // Tracing enabled
    tracing_enabled: bool,
    // Child tasks
    child_tasks: Vec<Box<dyn std::any::Any + Send + Sync>>,
    // Parent task
    parent: Option<Box<dyn std::any::Any + Send + Sync>>,
    // Atomic cancellation flag for quick checks
    atomic_cancelled: AtomicBool,
    // Current working directory
    cwd: PathBuf,
    // Task name
    name: Option<String>,
    // Vector clock for distributed causality
    vector_clock: crate::task::vector_clock::VectorClock<I>,
    // Task relationships for channel-based communication
    relationships: TaskRelationships<T, I>,
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTask<T, I> {
    /// Get the task ID
    pub fn task_id(&self) -> I {
        self.id
    }
    
    /// Get a reference to the vector clock
    pub fn vector_clock(&self) -> &crate::task::vector_clock::VectorClock<I> {
        &self.vector_clock
    }
    
    /// Get a mutable reference to the vector clock
    pub fn vector_clock_mut(&mut self) -> &mut crate::task::vector_clock::VectorClock<I> {
        &mut self.vector_clock
    }
    
    /// Tick the vector clock (increment our logical time)
    pub fn tick_clock(&mut self) {
        self.vector_clock.tick(&self.id, &self.hostname);
    }
    
    /// Update vector clock from received message
    pub fn update_clock_from(&mut self, other: &crate::task::vector_clock::VectorClock<I>) {
        self.vector_clock.merge(other);
        self.tick_clock();
    }
    
    /// Process any pending update messages from the spawned task
    fn process_updates(&mut self) {
        if let Some(rx) = &mut self.update_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    TaskMessage::StatusUpdate(status) => {
                        self.update_status(status);
                    }
                    TaskMessage::StartTime(time) => {
                        let nanos = time.duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
                        self.atomic_start_time.store(nanos, Ordering::Relaxed);
                    }
                    TaskMessage::EndTime(time) => {
                        let nanos = time.duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
                        self.atomic_end_time.store(nanos, Ordering::Relaxed);
                    }
                    TaskMessage::Result(result) => {
                        self.result = Some(result.clone());
                        match result {
                            Ok(ref value) => {
                                let _ = self.success_value.set(value.clone());
                                self.atomic_result_success.store(true, Ordering::Relaxed);
                            }
                            Err(_) => {
                                self.atomic_result_success.store(false, Ordering::Relaxed);
                            }
                        }
                        self.atomic_result_available.store(true, Ordering::Relaxed);
                    }
                    TaskMessage::MetricsUpdate { cpu_time } => {
                        self.metrics.update_cpu_time(cpu_time);
                    }
                }
            }
        }
    }

    /// Helper to update status
    fn update_status(&self, new_status: TaskStatus) {
        // Update atomic immediately
        self.atomic_status.store(status_to_u8(&new_status), Ordering::Relaxed);
        
        // Update cancelled flag if needed
        if matches!(new_status, TaskStatus::Cancelled | TaskStatus::PendingCancellation) {
            self.atomic_cancelled.store(true, Ordering::Relaxed);
        }
    }

    
    /// Create a new AsyncTask with priority (alias for new)
    pub fn new_with_priority(
        id: I,
        priority: TaskPriority,
        runtime: Handle,
        active_tasks: Arc<AtomicUsize>,
    ) -> Self {
        Self::new(id, priority, runtime, active_tasks)
    }

    /// Create a new AsyncTask directly with an ID and priority
    pub fn new(
        id: I,
        priority: TaskPriority,
        runtime: Handle,
        active_tasks: Arc<AtomicUsize>,
    ) -> Self {
        let (cancel_tx, _) = oneshot::channel();
        let created_time = SystemTime::now();

        // Create the new task
        Self {
            id,
            priority,
            handle: None,
            cancel_tx: Some(cancel_tx),
            update_rx: None, // Will be set when task is spawned
            atomic_status: AtomicU8::new(status_to_u8(&TaskStatus::Pending)),
            result: None,
            success_value: OnceCell::new(),
            atomic_result_available: AtomicBool::new(false),
            atomic_result_success: AtomicBool::new(false),
            created_time,
            atomic_start_time: AtomicU64::new(0), // 0 means not started
            atomic_end_time: AtomicU64::new(0), // 0 means not ended
            timeout: Duration::from_secs(0), // Default - no timeout
            runtime,
            active_tasks,
            metrics: TaskMetrics::new(),
            fallback: None,
            retry_count: 0,
            current_retry: AtomicU8::new(0),
            cancel_callbacks: Vec::new(),
            tracing_enabled: false,
            child_tasks: Vec::new(),
            parent: None,
            atomic_cancelled: AtomicBool::new(false),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            name: None,
            hostname: hostname::get()
                .ok()
                .and_then(|name| name.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
            vector_clock: crate::task::vector_clock::VectorClock::new(),
        }
    }

    /// Create a new AsyncTask from an existing SpawningTask
    pub fn from_spawning_task<S>(
        task: S,
        priority: TaskPriority,
        runtime: Handle,
        active_tasks: Arc<AtomicUsize>,
    ) -> Self 
    where
        S: SpawningTask<T, I> + 'static,
    {
        let id = task.task_id();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Create a future that will execute the SpawningTask
        let task_future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>> = 
            Box::pin(async move {
                // SpawningTask itself is a Future via AsyncTask
                let task_as_async: Box<dyn ApiAsyncTask<T, I>> = Box::new(task);
                match task_as_async.await {
                    Ok(value) => Ok(value),
                    Err(e) => Err(e),
                }
            });

        let created_time = SystemTime::now();
        
        // Create channel for task updates
        let (update_tx, update_rx) = mpsc::unbounded_channel();
        
        // Create the new task
        let mut new_task = Self {
            id,
            priority,
            handle: None,
            cancel_tx: Some(cancel_tx),
            update_rx: Some(update_rx),
            atomic_status: AtomicU8::new(status_to_u8(&TaskStatus::Pending)),
            result: None,
            success_value: OnceCell::new(),
            atomic_result_available: AtomicBool::new(false),
            atomic_result_success: AtomicBool::new(false),
            created_time,
            atomic_start_time: AtomicU64::new(0),
            atomic_end_time: AtomicU64::new(0),
            timeout: Duration::from_secs(0),
            runtime: runtime.clone(),
            active_tasks: active_tasks.clone(),
            metrics: TaskMetrics::new(),
            fallback: None,
            retry_count: 0,
            current_retry: AtomicU8::new(0),
            cancel_callbacks: Vec::new(),
            tracing_enabled: false,
            child_tasks: Vec::new(),
            parent: None,
            atomic_cancelled: AtomicBool::new(false),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            name: None,
            hostname: hostname::get()
                .ok()
                .and_then(|name| name.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
            vector_clock: crate::task::vector_clock::VectorClock::new(),
        };

        // Clone necessary state for the async task
        let active_tasks_cl = new_task.active_tasks.clone();

        // Spawn the task
        let task_handle = runtime.spawn(async move {
            // Update status to Running
            let _ = update_tx.send(TaskMessage::StatusUpdate(TaskStatus::Running));
            let start_time = SystemTime::now();
            let _ = update_tx.send(TaskMessage::StartTime(start_time));

            let start = tokio::time::Instant::now();
            
            // Execute the task future
            let result = task_future.await;
            
            // Update metrics
            let elapsed = start.elapsed();
            let _ = update_tx.send(TaskMessage::MetricsUpdate { cpu_time: elapsed });
            
            // Record end time
            let end_time = SystemTime::now();
            let _ = update_tx.send(TaskMessage::EndTime(end_time));
            
            // Update status and send result
            match &result {
                Ok(_) => {
                    let _ = update_tx.send(TaskMessage::StatusUpdate(TaskStatus::Completed));
                }
                Err(_) => {
                    let _ = update_tx.send(TaskMessage::StatusUpdate(TaskStatus::Cancelled));
                }
            }
            let _ = update_tx.send(TaskMessage::Result(result.clone()));
            
            // Decrement active task count
            active_tasks_cl.fetch_sub(1, Ordering::SeqCst);
            
            result
        });

        // Store the handle
        new_task.handle = Some(task_handle);
        
        // Increment active task count
        new_task.active_tasks.fetch_add(1, Ordering::SeqCst);

        new_task
    }
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Set a fallback value for the task
    pub fn with_fallback(self, value: T) -> Self {
        let fallback_clone = Arc::clone(&self.fallback);
        tokio::spawn(async move {
            let mut fallback = fallback_clone.lock().await;
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
        let parent_clone = Arc::clone(&self.parent);
        tokio::spawn(async move {
            let mut parent_lock = parent_clone.lock().await;
            *parent_lock = Some(parent);
        });
        self
    }

    /// Add a child task
    pub fn add_child(&self, child: Box<dyn std::any::Any + Send + Sync>) {
        let children_clone = Arc::clone(&self.child_tasks);
        tokio::spawn(async move {
            let mut children = children_clone.lock().await;
            children.push(child);
        });
    }

    /// Set a name for the task
    pub fn with_name(self, name: String) -> Self {
        let name_clone = Arc::clone(&self.name);
        tokio::spawn(async move {
            let mut task_name = name_clone.lock().await;
            *task_name = Some(name);
        });
        self
    }

    /// Get the task name if set
    pub fn name(&self) -> Option<String> {
        // Use try_lock for non-blocking access
        match self.name.try_lock() {
            Ok(name) => name.clone(),
            Err(_) => None, // If locked, return None
        }
    }

    /// Attach a future to this task
    pub fn with_future(
        self,
        future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>>,
    ) -> Self {
        // Create clones of all the necessary state for the task
        let handle_cl = self.handle.clone();
        let status_cl = self.status.clone();
        let result_cl = self.result.clone();
        let success_value_cl = self.success_value.clone();
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
        {
            tokio::spawn(async move {
                let mut tx = cancel_cl.lock().await;
                *tx = Some(cancel_tx);
            });
        }

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

            // Run the task and cancellation in a select
            let task_result = tokio::select! {
                _ = cancel_fut => {
                    Err(AsyncTaskError::Cancelled)
                },
                result = async {
                    // Execute the task with timeout if specified
                    if timeout_dur > Duration::from_secs(0) {
                        match timeout(timeout_dur, future).await {
                            Ok(result) => result,
                            Err(_) => Err(AsyncTaskError::Timeout(timeout_dur)),
                        }
                    } else {
                        future.await
                    }
                } => result,
            };

            // Process the result
            let result = match task_result {
                Ok(value) => {
                    // Task completed successfully
                    *status_cl.lock().await = TaskStatus::Completed;
                    // Store the value in OnceCell for value() method
                    let _ = success_value_cl.set(value.clone());
                    Ok(value)
                }
                Err(error) => {
                    // Check for fallback value on error
                    let fallback = {
                        let fallback = fallback_cl.lock().await;
                        fallback.clone()
                    };

                    if let Some(value) = fallback {
                        *status_cl.lock().await = TaskStatus::Completed;
                        // Store the fallback value in OnceCell
                        let _ = success_value_cl.set(value.clone());
                        Ok(value)
                    } else {
                        match error {
                            AsyncTaskError::Cancelled => {
                                *status_cl.lock().await = TaskStatus::Cancelled;
                            }
                            _ => {
                                *status_cl.lock().await = TaskStatus::Cancelled;
                            }
                        }
                        Err(error)
                    }
                }
            };

            // Update metrics
            let elapsed = start.elapsed();
            metrics_cl.update_cpu_time(elapsed);

            // Record end time
            *end_time_cl.lock().await = Some(SystemTime::now());

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

            // Note: We don't store the result to avoid Clone requirement on T
            // The value() method will need to be implemented differently

            result
        });

        // Register the task handle
        {
            tokio::spawn(async move {
                // Create a void handle for the active_tasks list since we can't clone JoinHandle
                let void_handle = tokio::spawn(async {});
                *handle_cl.lock().await = Some(task_handle);
                active_tasks_cl.lock().await.push(void_handle);
            });
        }

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
            atomic_status: self.atomic_status.clone(),
            result: self.result.clone(),
            success_value: self.success_value.clone(),
            atomic_result_available: self.atomic_result_available.clone(),
            atomic_result_success: self.atomic_result_success.clone(),
            created_time: self.created_time,
            start_time: self.start_time.clone(),
            atomic_start_time: self.atomic_start_time.clone(),
            end_time: self.end_time.clone(),
            atomic_end_time: self.atomic_end_time.clone(),
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
            atomic_cancelled: self.atomic_cancelled.clone(),
            cwd: self.cwd.clone(),
            name: self.name.clone(),
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId + Unpin> Future for AsyncTask<T, I> {
    type Output = Result<T, AsyncTaskError>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = Pin::into_inner(self);
        
        // First check if we have a cached result
        if let Ok(mut guard) = this.result.try_lock() {
            if let Some(result) = guard.take() {
                return std::task::Poll::Ready(result);
            }
        }
        
        // Get the task handle
        let handle_opt = {
            let guard = this.handle.try_lock();
            match guard {
                Ok(mut g) => g.take(),
                Err(_) => return std::task::Poll::Pending,
            }
        };

        if let Some(mut handle) = handle_opt {
            // Poll the actual task
            match Pin::new(&mut handle).poll(cx) {
                std::task::Poll::Ready(result) => {
                    let final_result = match result {
                        Ok(r) => r,
                        Err(e) => Err(AsyncTaskError::Failure(format!("Task join error: {}", e))),
                    };
                    
                    // Cache the result - we can't clone AsyncTaskError, so cache and return
                    std::task::Poll::Ready(final_result)
                }
                std::task::Poll::Pending => {
                    // Put the handle back
                    if let Ok(mut guard) = this.handle.try_lock() {
                        *guard = Some(handle);
                    }
                    std::task::Poll::Pending
                }
            }
        } else {
            std::task::Poll::Ready(Err(AsyncTaskError::Failure("Task not started".to_string())))
        }
    }
}

impl<T: Clone + Send + Sync + 'static, I: TaskId> CancellableTask<T> for AsyncTask<T, I> {
    async fn cancel(&self, level: CancellationLevel) -> Result<(), OrchestratorError> {
        // Update status atomically
        self.atomic_status.store(TaskStatus::PendingCancellation as u8, Ordering::SeqCst);

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
        self.update_status(TaskStatus::Cancelled);

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
        // First check atomic flag for quick return
        if self.atomic_cancelled.load(Ordering::Relaxed) {
            return true;
        }
        
        // Then check atomic status
        let status_val = self.atomic_status.load(Ordering::Relaxed);
        let status = u8_to_status(status_val);
        matches!(
            status,
            TaskStatus::Cancelled | TaskStatus::PendingCancellation
        )
    }

    fn on_cancel<F, Fut>(&self, callback: F)
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let callbacks_clone = Arc::clone(&self.cancel_callbacks);
        tokio::spawn(async move {
            let mut callbacks = callbacks_clone.lock().await;
            callbacks.push(Box::new(move || Box::pin(callback.run())));
        });
    }

    async fn cancel_gracefully(&self) -> Result<(), OrchestratorError> {
        self.cancel(CancellationLevel::Graceful).await
    }

    async fn cancel_forcefully(&self) -> Result<(), OrchestratorError> {
        self.cancel(CancellationLevel::Kill).await
    }

    async fn cancel_immediately(&self) -> Result<(), OrchestratorError> {
        self.cancel(CancellationLevel::KillHard).await
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TimedTask<T> for AsyncTask<T, I> {
    fn created_timestamp(&self) -> SystemTime {
        self.created_time
    }

    fn executed_timestamp(&self) -> SystemTime {
        let nanos = self.atomic_start_time.load(Ordering::Relaxed);
        if nanos == 0 {
            self.created_time
        } else {
            UNIX_EPOCH + Duration::from_nanos(nanos)
        }
    }

    fn completed_timestamp(&self) -> SystemTime {
        let nanos = self.atomic_end_time.load(Ordering::Relaxed);
        if nanos == 0 {
            self.created_time
        } else {
            UNIX_EPOCH + Duration::from_nanos(nanos)
        }
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

    fn relationships(&self) -> &TaskRelationships<T, I> {
        &self.relationships
    }

    fn relationships_mut(&mut self) -> &mut TaskRelationships<T, I> {
        &mut self.relationships
    }

    fn runtime(&self) -> &Self::RuntimeType {
        // Because AsyncTask doesn't currently store a reference to the TokioRuntime
        // we need to panic with an informative message instead of returning a reference
        panic!(
            "ContextualizedTask::runtime is not directly available. Use AsyncTask::runtime_handle() instead."
        )
    }

    fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }
}

impl<T: Clone + Send + 'static, I: TaskId> StatusEnabledTask<T> for AsyncTask<T, I> {
    fn status(&self) -> TaskStatus {
        let status_val = self.atomic_status.load(Ordering::Relaxed);
        u8_to_status(status_val)
    }
}

impl<T: Clone + Send + 'static, I: TaskId> RecoverableTask<T> for AsyncTask<T, I> {
    fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        // Try to get the fallback value without blocking
        match self.fallback.try_lock() {
            Ok(fallback) => {
                if let Some(value) = fallback.clone() {
                    Ok(value)
                } else {
                    Err(error)
                }
            }
            Err(_) => Err(error), // If locked, can't recover
        }
    }

    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        // Check if we have a fallback value without blocking
        match self.fallback.try_lock() {
            Ok(fallback) => fallback.is_some(),
            Err(_) => false, // If locked, assume no recovery
        }
    }

    fn fallback_value(&self) -> Option<T> {
        // Try to get the fallback value without blocking
        match self.fallback.try_lock() {
            Ok(fallback) => fallback.clone(),
            Err(_) => None, // If locked, return None
        }
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

impl<T: Clone + Send + Sync + 'static, I: TaskId> ApiAsyncTask<T, I> for AsyncTask<T, I> {
    fn to<R: Send + 'static, Task: ApiAsyncTask<R, I> + 'static>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_spawning()
    }

    fn emits<R: Send + 'static, Task: ApiAsyncTask<R, I> + 'static>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_emitting()
    }
}

impl<T: Clone + Send + Sync + 'static, I: TaskId> SpawningTask<T, I>
    for AsyncTask<T, I>
{
    type TaskResult = crate::task::spawn::TokioTaskResult<T>;
    type OutputFuture = Pin<Box<dyn Future<Output = Self::TaskResult> + Send>>;
    type JoinChildrenFuture =
        Pin<Box<dyn Future<Output = Self::JoinChildrenResult> + Send + 'static>>;
    type JoinChildrenResult = crate::task::spawn::TokioAsyncResult<Vec<I>>;
    type AsyncWork = crate::task::async_work::DynAsyncWork<T>;

    fn task_id(&self) -> I {
        self.id
    }

    fn run(self, work: Self::AsyncWork) -> Self {
        // Spawn the work asynchronously
        let result_clone = Arc::clone(&self.result);
        let success_value_clone = Arc::clone(&self.success_value);
        let handle: JoinHandle<Result<T, AsyncTaskError>> = self.runtime.spawn(async move {
            let work_result = <Self::AsyncWork as AsyncWork<T>>::run(work).await;
            let mut task_result = result_clone.lock().await;
            *task_result = Some(Ok(work_result.clone()));
            let _ = success_value_clone.set(work_result.clone());
            Ok(work_result)
        });
        
        // Store the handle
        let handle_clone = Arc::clone(&self.handle);
        tokio::spawn(async move {
            let mut h = handle_clone.lock().await;
            *h = Some(handle);
        });

        self
    }

    fn run_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Send + 'static,
        Self: SpawningTask<R, I>,
    {
        // Create a child task and execute it
        let child_id = self.id; // For now, use parent's ID
        let runtime = self.runtime.clone();
        let active_tasks = self.active_tasks.clone();
        
        Box::pin(async move {
            // Create a new task for the child
            let child_task = AsyncTask::<R, I>::new(
                child_id,
                TaskPriority::Normal,
                runtime,
                active_tasks,
            );
            
            // Execute the child task
            let result = child_task.await;
            
            // Wrap in TaskResult
            crate::task::spawn::TokioTaskResult::new(result)
        })
    }

    fn join_children(&self) -> Self::JoinChildrenFuture {
        // Wait for all child tasks to complete
        let child_tasks = self.child_tasks.clone();
        
        Box::pin(async move {
            let tasks = child_tasks.lock().await;
            let mut child_ids = Vec::new();
            
            // Collect all child task IDs
            for child in tasks.iter() {
                // For now, we'll just return empty vec since we don't have proper child tracking
                // A full implementation would track child task IDs properly
            }
            
            crate::task::spawn::TokioAsyncResult::new(Ok(child_ids))
        })
    }

    fn value(&self) -> Option<&T> {
        self.success_value.get()
    }

    fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: sweet_async_api::task::builder::AsyncWork<U> + Send + 'static,
        U: Send + 'static,
        Self: SpawningTask<U, I>,
    {
        // Chain the next operation after this task completes
        let task_id = self.id;
        let runtime = self.runtime.clone();
        let active_tasks = self.active_tasks.clone();
        
        Box::pin(async move {
            // First execute this task
            let first_result = self.await;
            
            match first_result {
                Ok(_) => {
                    // If first task succeeded, run the chained work
                    let chained_task = AsyncTask::<U, I>::new(
                        task_id,
                        TaskPriority::Normal,
                        runtime,
                        active_tasks,
                    );
                    
                    // Execute the chained work
                    let chained_result = f.run().await;
                    crate::task::spawn::TokioTaskResult::new(Ok(chained_result))
                }
                Err(e) => {
                    // If first task failed, propagate the error
                    crate::task::spawn::TokioTaskResult::new(Err(e))
                }
            }
        })
    }
}
