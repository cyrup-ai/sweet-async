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
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
/// 
/// let builder = builder::<String, MyTaskId>();
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
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
/// 
/// let builder = builder::spawning_builder::<String, &'static str, MyTaskId>();
/// let task = builder.run(|| async { "Hello, world!".to_string() });
/// ```
pub fn spawning_builder<T: Clone + Send + 'static, E: Send + 'static, I: TaskId>() -> TokioSpawningTaskBuilder<T, E, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(Mutex::new(Vec::new()));
    TokioSpawningTaskBuilder::<T, E, I>::new(runtime, active_tasks)
}
