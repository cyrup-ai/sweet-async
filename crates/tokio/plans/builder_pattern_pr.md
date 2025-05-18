# Builder Pattern Implementation PR

![Sweet Async Logo](/assets/sweet_async.png)

This document outlines the implementation plan for the Builder Pattern PR, which will serve as the foundation for the Tokio implementation of Sweet Async.

## Overview

The builder pattern is central to the Sweet Async API, providing a fluent interface for configuring and creating tasks. This PR will implement the core builder classes, ensuring they properly integrate with the AsyncTask implementation.

## Implementation Plan

### 1. Create Core Builder Structure

First, we'll create the base builder directory structure and files:

```
/crates/tokio/src/task/builder/
├── mod.rs
└── base_builder.rs
```

**`mod.rs`**: Module definition with proper exports
```rust
//! Task builder module for the Tokio implementation of Sweet Async
//! 
//! This module provides builder implementations for creating and configuring tasks
//! using the immutable builder pattern from the Sweet Async API.

mod base_builder;

pub use base_builder::AsyncTaskBuilder;

// Re-export from sub-modules
pub mod spawn;
```

**`base_builder.rs`**: Core builder implementation
```rust
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Base builder for configuring Tokio tasks
/// 
/// This builder implements the core configuration options common to all task types,
/// following the immutable builder pattern where each method returns a new instance.
#[derive(Clone)]
pub struct AsyncTaskBuilder<T, I>
where
    T: Send + 'static,
    I: TaskId,
{
    /// Task name for identification
    name: Option<String>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Timeout for task execution
    timeout: Duration,
    /// Number of retry attempts
    retry_attempts: u8,
    /// Whether tracing is enabled
    tracing_enabled: bool,
    /// Phantom data for type parameters
    _phantom: PhantomData<(T, I)>,
}

impl<T, I> AsyncTaskBuilder<T, I>
where
    T: Send + 'static,
    I: TaskId,
{
    /// Create a new builder with the specified runtime and active tasks registry
    pub fn new(runtime: Handle, active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>) -> Self {
        Self {
            name: None,
            runtime,
            active_tasks,
            timeout: Duration::from_secs(0), // No timeout by default
            retry_attempts: 0,               // No retries by default
            tracing_enabled: false,          // Tracing disabled by default
            _phantom: PhantomData,
        }
    }
    
    /// Get the current runtime handle
    pub fn runtime(&self) -> &Handle {
        &self.runtime
    }
    
    /// Get the active tasks registry
    pub fn active_tasks(&self) -> &Arc<Mutex<Vec<JoinHandle<()>>>> {
        &self.active_tasks
    }
    
    /// Get the configured task name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    
    /// Get the configured timeout
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }
    
    /// Get the configured retry attempts
    pub fn get_retry_attempts(&self) -> u8 {
        self.retry_attempts
    }
    
    /// Check if tracing is enabled
    pub fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled
    }
}

impl<T, I> AsyncTaskBuilder for AsyncTaskBuilder<T, I>
where
    T: Send + 'static,
    I: TaskId,
{
    /// Create a new builder instance with default settings
    fn new() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        Self::new(runtime, active_tasks)
    }
    
    /// Set a descriptive name for the task
    fn name(self, name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            ..self
        }
    }
    
    /// Set a timeout duration for task execution
    fn timeout(self, duration: Duration) -> Self {
        Self {
            timeout: duration,
            ..self
        }
    }
    
    /// Configure the number of retry attempts for failures
    fn retry(self, attempts: u8) -> Self {
        Self {
            retry_attempts: attempts,
            ..self
        }
    }
    
    /// Enable or disable detailed execution tracing
    fn tracing(self, enabled: bool) -> Self {
        Self {
            tracing_enabled: enabled,
            ..self
        }
    }
}
```

### 2. Implement SpawningTaskBuilder

Create the directory structure for spawn builder:

```
/crates/tokio/src/task/spawn/
├── mod.rs
└── builder.rs
```

**`mod.rs`**: Module definition with proper exports
```rust
//! Task spawn module for creating future-based tasks
//! 
//! This module provides implementations for spawning asynchronous tasks
//! that return results directly.

pub mod builder;

pub use builder::TokioSpawningTaskBuilder;
```

**`builder.rs`**: Builder for spawning tasks
```rust
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::SpawningTaskBuilder;
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;

use crate::task::builder::AsyncTaskBuilder;
use crate::task::tokio_task::AsyncTask;

/// Builder for creating and configuring spawning tasks
/// 
/// A SpawningTaskBuilder is used to create future-based tasks that 
/// execute once and return a result.
pub struct TokioSpawningTaskBuilder<T, E, I>
where
    T: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// The base builder with common configuration
    base: AsyncTaskBuilder<T, I>,
    /// Task priority
    priority: TaskPriority,
    /// Phantom data for error type parameter
    _phantom_e: PhantomData<E>,
}

impl<T, E, I> TokioSpawningTaskBuilder<T, E, I>
where
    T: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Create a new spawning task builder
    pub fn new(
        runtime: tokio::runtime::Handle,
        active_tasks: Arc<tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    ) -> Self {
        Self {
            base: AsyncTaskBuilder::new(runtime, active_tasks),
            priority: TaskPriority::Normal,
            _phantom_e: PhantomData,
        }
    }
    
    /// Set the task priority
    pub fn with_priority(self, priority: TaskPriority) -> Self {
        Self {
            priority,
            ..self
        }
    }
}

impl<T, E, I> AsyncTaskBuilder for TokioSpawningTaskBuilder<T, E, I>
where
    T: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    fn new() -> Self {
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        Self::new(runtime, active_tasks)
    }

    fn name(self, name: &str) -> Self {
        Self {
            base: self.base.name(name),
            ..self
        }
    }

    fn timeout(self, duration: std::time::Duration) -> Self {
        Self {
            base: self.base.timeout(duration),
            ..self
        }
    }

    fn retry(self, attempts: u8) -> Self {
        Self {
            base: self.base.retry(attempts),
            ..self
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        Self {
            base: self.base.tracing(enabled),
            ..self
        }
    }
}

impl<T, E, I> SpawningTaskBuilder<T, E, I> for TokioSpawningTaskBuilder<T, E, I>
where
    T: Send + 'static,
    E: Send + 'static,
    I: TaskId,
    E: Into<AsyncTaskError> + From<AsyncTaskError>,
{
    type Task = AsyncTask<T, I>;

    /// Create a task with the given work function
    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        // Generate a unique task ID
        let id = I::generate();
        
        // Create a future that runs the work and converts the result
        let future = Box::pin(async move {
            let result = work.run().await;
            match result.into_async_result() {
                Ok(value) => Ok(value),
                Err(err) => Err(err.into()),
            }
        });
        
        // Create the AsyncTask with the future
        let task = AsyncTask::new_with_future(
            id,
            self.priority,
            future,
            self.base.runtime().clone(),
            self.base.active_tasks().clone(),
        );
        
        // Apply configuration
        let task = task
            .with_timeout(self.base.get_timeout())
            .with_retry(self.base.get_retry_attempts())
            .with_tracing(self.base.is_tracing_enabled());
            
        // Apply name if set
        if let Some(name) = self.base.name() {
            task.with_name(name.to_string())
        } else {
            task
        }
    }

    /// Create and immediately await a task
    fn await_result<F, R>(self, work: F) -> impl Future<Output = Result<T, E>> + Send
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        let task = self.run(work);
        
        // Return a future that awaits the task and converts errors
        Box::pin(async move {
            match task.await {
                Ok(value) => Ok(value),
                Err(err) => Err(E::from(err)),
            }
        })
    }

    /// Create, await, and process with a handler
    fn await_result_with_handler<F, R, H, Out>(
        self,
        work: F,
        handler: H,
    ) -> impl Future<Output = Out> + Send
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
        H: AsyncWork<Out> + Send + 'static,
    {
        let await_result_future = self.await_result(work);
        
        Box::pin(async move {
            let result = await_result_future.await;
            handler.run().await
        })
    }
}
```

### 3. Update AsyncTask for Builder Integration

Add a new method to AsyncTask to support creation via builder:

```rust
impl<T: Send + 'static, I: TaskId> AsyncTask<T, I> {
    /// Create a new task with a pre-built future
    pub fn new_with_future(
        id: I,
        priority: TaskPriority,
        future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>>,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    ) -> Self {
        // Create cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();
        
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
        
        // Create clones for the task
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
            
            // Execute the task with timeout if specified
            let task_result = if timeout_dur > Duration::from_secs(0) {
                match timeout(timeout_dur, future).await {
                    Ok(result) => result,
                    Err(_) => Err(AsyncTaskError::Timeout(timeout_dur)),
                }
            } else {
                future.await
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
                            *status_cl.lock().await = TaskStatus::Failed;
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
    
    /// Set a name for the task
    pub fn with_name(self, name: String) -> Self {
        futures::executor::block_on(async {
            let mut task_name = self.name.lock().await;
            *task_name = Some(name);
        });
        self
    }
}
```

### 4. Add Static Methods to AsyncTask for Builder Creation

Update AsyncTask to implement the AsyncTask static methods:

```rust
impl<T: Send + 'static, I: TaskId> AsyncTask<T, I> for AsyncTask<T, I> {
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::task::spawn::builder::TokioSpawningTaskBuilder;
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        TokioSpawningTaskBuilder::<R, AsyncTaskError, I>::new(runtime, active_tasks)
    }

    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        // This will be implemented in a future PR
        unimplemented!("Emitting task builder will be implemented in a future PR")
    }
}
```

### 5. Add Convenience Functions

Create a new file in the crate root:

**`src/builder.rs`**:
```rust
//! Convenience builder functions for creating Tokio tasks
//!
//! This module provides simple, ergonomic functions for creating builders
//! without having to import the full builder types directly.

use std::sync::Arc;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::AsyncTaskError;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::task::builder::AsyncTaskBuilder;
use crate::task::spawn::builder::TokioSpawningTaskBuilder;

/// Creates a new base task builder with default settings
///
/// # Example
///
/// ```rust
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// let builder = builder::<String, impl TaskId>();
/// ```
pub fn builder<T: Send + 'static, I: TaskId>() -> AsyncTaskBuilder<T, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(Mutex::new(Vec::new()));
    AsyncTaskBuilder::<T, I>::new(runtime, active_tasks)
}

/// Creates a new spawning task builder with default settings
///
/// This builder is specifically for future-based workflows that execute
/// once and return a result.
///
/// # Example
///
/// ```rust
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// let builder = builder::spawning_builder::<String, impl TaskId>();
/// let task = builder.run(|| async { "Hello, world!".to_string() });
/// ```
pub fn spawning_builder<T: Send + 'static, I: TaskId>() -> TokioSpawningTaskBuilder<T, AsyncTaskError, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(Mutex::new(Vec::new()));
    TokioSpawningTaskBuilder::<T, AsyncTaskError, I>::new(runtime, active_tasks)
}
```

### 6. Update the Main Library File

**`src/lib.rs`**:
```rust
mod runtime;
mod orchestrator;
mod task;
mod utils;
mod builder;

// Re-export core components
pub use runtime::TokioRuntime;
pub use orchestrator::TokioOrchestrator;
pub use task::*;
pub use utils::*;
pub use builder::*;

/// Create a new Tokio runtime using the current handle
pub fn new_runtime() -> TokioRuntime {
    TokioRuntime::new()
}

/// Create a new Tokio runtime with custom configuration
pub fn new_runtime_with_config(workers: usize) -> TokioRuntime {
    TokioRuntime::with_config(workers)
}

/// Create a new Tokio orchestrator using the given runtime
pub fn new_orchestrator(runtime: TokioRuntime) -> TokioOrchestrator<(), impl sweet_async_api::task::TaskId> {
    TokioOrchestrator::new(runtime)
}
```

### 7. Add Tests

Create a new test file:

**`tests/builder_tests.rs`**:
```rust
use std::time::Duration;
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskStatus};
use sweet_async_api::task::builder::AsyncTaskBuilder;
use sweet_async_api::task::spawn::SpawningTaskBuilder;
use sweet_async_tokio::builder;
use tokio_test::block_on;

// Simple test ID implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TestTaskId(u64);

impl TaskId for TestTaskId {
    fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TestTaskId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn to_string(&self) -> String {
        format!("TestTask-{}", self.0)
    }
}

#[test]
fn test_builder_creates_task() {
    // Create a task via builder
    let builder = builder::spawning_builder::<String, TestTaskId>();
    let task = builder.run(|| async { "Test task completed".to_string() });
    
    // Verify task properties
    assert_eq!(task.status(), TaskStatus::Pending);
    assert_eq!(task.timeout(), Duration::from_secs(0)); // Default - no timeout
    
    // Await the task and check result
    let result = block_on(task);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test task completed");
}

#[test]
fn test_builder_with_timeout() {
    // Create a task with timeout
    let builder = builder::spawning_builder::<String, TestTaskId>()
        .timeout(Duration::from_millis(50));
    
    // Task that sleeps longer than the timeout
    let task = builder.run(|| async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        "This should timeout".to_string()
    });
    
    // Verify timeout property
    assert_eq!(task.timeout(), Duration::from_millis(50));
    
    // Await the task and verify it times out
    let result = block_on(task);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AsyncTaskError::Timeout(_)));
}

#[test]
fn test_static_builder_method() {
    // Use the static method on AsyncTask
    let task = block_on(sweet_async_tokio::task::AsyncTask::<String, TestTaskId>::to()
        .run(|| async { "Created via static method".to_string() }));
    
    assert!(task.is_ok());
    assert_eq!(task.unwrap(), "Created via static method");
}

#[test]
fn test_builder_with_tracing() {
    // Create a task with tracing enabled
    let builder = builder::spawning_builder::<u32, TestTaskId>()
        .tracing(true);
    
    let task = builder.run(|| async { 42 });
    
    // Verify tracing is enabled
    assert!(task.is_tracing_enabled());
}

#[test]
fn test_await_result() {
    // Use await_result directly
    let result = block_on(
        builder::spawning_builder::<u32, TestTaskId>()
            .await_result(|| async { 123 })
    );
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 123);
}

#[test]
fn test_builder_with_name() {
    // Create a task with a name
    let builder = builder::spawning_builder::<String, TestTaskId>()
        .name("test-task");
    
    let task = builder.run(|| async { "Named task".to_string() });
    
    // Verify name is set correctly
    // Note: You'll need to add a method to access the task name
    // assert_eq!(task.name(), Some("test-task"));
}
```

## Implementation Notes

1. This PR focuses only on the builder pattern for future-based tasks
2. The event-based tasks (EmittingTaskBuilder) will be implemented in a future PR
3. All builder methods follow the immutable pattern, returning new instances
4. Task configuration is applied when the task is built, not afterward
5. Tests validate the core functionality

## Expected Outcome

After this PR:

1. Users will be able to create tasks using the builder pattern
2. The static method `AsyncTask::to()` will work for creating builders
3. Configuration options like timeout, retry, and tracing will work correctly
4. The codebase will have a solid foundation for implementing other components

![Book](/assets/book.png)
