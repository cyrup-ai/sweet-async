//! Task builder implementation for Tokio
//!
//! This module provides the implementation of the builder pattern for Tokio tasks,
//! following the immutable builder pattern where each method returns a new instance.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use sweet_async_api::task::TaskId;
pub use sweet_async_api::task::builder::{
    AsyncTaskBuilder as ApiAsyncTaskBuilder, AsyncWork, ErrorStrategy, MinMax, ReceiverStrategy,
    SenderStrategy,
};
use tokio::runtime::Handle;

// Export the DefaultOrchestratorBuilder from the nested module
pub mod builder;
pub use self::builder::DefaultOrchestratorBuilder;

/// Builder for creating and configuring Tokio tasks
///
/// This builder implements the core configuration options common to all task types,
/// following the immutable builder pattern where each method returns a new instance.
#[derive(Clone)]
pub struct TokioAsyncTaskBuilder<T, I>
where
    T: Send + 'static,
    I: TaskId,
{
    /// Task name for identification
    name: Option<String>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks counter (lock-free)
    active_tasks: Arc<AtomicUsize>,
    /// Timeout for task execution
    timeout: Duration,
    /// Number of retry attempts
    retry_attempts: u8,
    /// Whether tracing is enabled
    tracing_enabled: bool,
    /// Phantom data for type parameters
    _phantom: PhantomData<(T, I)>,
}

impl<T, I> TokioAsyncTaskBuilder<T, I>
where
    T: Send + 'static,
    I: TaskId,
{
    /// Create a new builder with the specified runtime and active tasks counter
    pub fn new_with_runtime(
        runtime: Handle,
        active_tasks: Arc<AtomicUsize>,
    ) -> Self {
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

    /// Get the active tasks counter
    pub fn active_tasks(&self) -> &Arc<AtomicUsize> {
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

}

impl<T, I> ApiAsyncTaskBuilder for TokioAsyncTaskBuilder<T, I>
where
    T: Send + 'static,
    I: TaskId,
{
    /// Create a new builder instance with default settings
    fn new() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(AtomicUsize::new(0));
        Self::new_with_runtime(runtime, active_tasks)
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

impl<T: Send + 'static, I: TaskId> TokioAsyncTaskBuilder<T, I> {
    /// Set a descriptive name for the task (extension method)
    pub fn name(self, name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            ..self
        }
    }
}
