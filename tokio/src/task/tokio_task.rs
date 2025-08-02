//! Tokio-specific task implementation
//!
//! This module provides the main TokioTask type that serves as the primary
//! task implementation for the Tokio runtime.

// Re-export the trait from the API
pub use sweet_async_api::task::AsyncTask;

// Re-export our concrete implementations
pub use crate::task::async_task::TokioAsyncTask as TokioTask;
pub use crate::task::task_metrics::TaskMetrics;
pub use crate::task::task_id::TokioTaskId;

// Re-export common types for convenience
use crate::task::async_task::TokioAsyncTask;
pub type DefaultTokioTask<T, I> = TokioAsyncTask<T, I>;