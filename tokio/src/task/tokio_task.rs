//! Tokio-specific task implementation
//!
//! This module provides the main TokioTask type that serves as the primary
//! task implementation for the Tokio runtime.

pub use crate::task::async_task::AsyncTask as TokioTask;
pub use crate::task::task_metrics::TaskMetrics;
pub use crate::task::task_id::TokioTaskId;
pub use crate::task::task_error::AsyncTaskError;
pub use crate::task::task_status::TokioTaskStatus;
pub use crate::task::task_priority::TokioTaskPriority;

// Re-export common types for convenience
pub type DefaultTokioTask<T, I> = TokioTask<T, I, crate::task::async_work::AsyncWork<T>, crate::task::error_fallback::ErrorFallback<T>>;