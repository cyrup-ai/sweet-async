//! Task builder implementation for Tokio
//!
//! This module provides the implementation of the builder pattern for Tokio tasks,
//! following the immutable builder pattern where each method returns a new instance.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Builder for creating and configuring Tokio tasks
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
            timeout: Duration::from_secs(30), // Default timeout
            retry_attempts: 0,                // No retries by default
            tracing_enabled: false,           // Tracing disabled by default
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
    pub fn get_name(&self) -> Option<String> {
        self.name.clone()
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
    
    /// Set a descriptive name for the task
    pub fn name(self, name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            ..self
        }
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

/// Helper struct for min/max range specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MinMax<T>(pub T, pub T)
where
    T: PartialOrd + Copy + std::fmt::Debug;

impl<T> MinMax<T>
where
    T: PartialOrd + Copy + std::fmt::Debug,
{
    /// Create a new range with minimum and maximum values
    pub fn new(min: T, max: T) -> Self {
        debug_assert!(
            min <= max,
            "MinMax: min ({:?}) must be <= max ({:?})",
            min,
            max
        );
        Self(min, max)
    }

    /// Get the minimum value
    pub fn min(&self) -> T {
        self.0
    }

    /// Get the maximum value
    pub fn max(&self) -> T {
        self.1
    }
}

/// Strategy for processing events in a queue (receiver-side)
#[derive(Debug, Clone)]
pub enum ReceiverStrategy {
    /// Process sequentially
    Serial {
        timeout_seconds: u64, // default: 0 (no timeout)
    },
    /// Process in parallel
    Parallel {
        workers: usize,  // default: num_cpus::get()
        rate_limit: f64, // default: 0.0 (no rate limit)
    },
    /// Process in batches
    Batched {
        batch_size: usize,   // default: 64
        max_delay: Duration, // default: Duration::from_millis(100)
    },
    /// Automatically adjust based on workload
    Adaptive {
        initial_capacity: usize,     // default: 64
        max_concurrency: usize,      // default: num_cpus::get()
        adaptation_window: Duration, // default: Duration::from_secs(1)
        use_rayon_for_cpu: bool,     // default: false
    },
}

/// Strategy for sending events in a queue (sender-side)
#[derive(Debug, Clone)]
pub enum SenderStrategy {
    /// Send sequentially
    Serial {
        timeout_seconds: u64, // default: 0 (no timeout)
    },
    /// Send in parallel
    Parallel {
        workers: MinMax<usize>, // default: MinMax(1, num_cpus::get())
        rate_limit: f64,        // default: 0.0 (no rate limit)
    },
    /// Send in batches
    Batched {
        batch_size: usize,   // default: 64
        max_delay: Duration, // default: Duration::from_millis(100)
    },
    /// Automatically adjust based on workload
    Adaptive {
        initial_capacity: usize,     // default: 64
        max_concurrency: usize,      // default: num_cpus::get()
        adaptation_window: Duration, // default: Duration::from_secs(1)
        use_rayon_for_cpu: bool,     // default: false
    },
}

/// Strategies for handling errors in queue processing
pub enum ErrorStrategy {
    /// Stop processing on the first error
    StopOnFirst,
    /// Continue processing despite errors
    ContinueOnError,
    /// Retry failed operations with the specified attempts
    Retry(u8),
    /// Custom error handling logic
    Custom(fn(sweet_async_api::task::AsyncTaskError) -> bool),
}

/// Trait for work that can be executed asynchronously
pub trait AsyncWork<R> {
    /// Execute the work and return a Future that resolves to the result
    fn run(self) -> impl std::future::Future<Output = R> + Send + 'static;
}

// Implementation for async closures
impl<R, F, Fut> AsyncWork<R> for F
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = R> + Send + 'static,
{
    fn run(self) -> impl std::future::Future<Output = R> + Send + 'static {
        self()
    }
}
