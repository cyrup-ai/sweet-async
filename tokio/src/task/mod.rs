//! Task module for Tokio implementation of Sweet Async
//!
//! This module contains the Tokio-specific implementations of the task-related
//! components defined in the sweet_async_api crate.

// Core modules matching API structure
pub mod adaptive;
pub mod async_task;
pub mod builder;
pub mod error_fallback;
pub mod cancellable_task;
pub mod default_context;
pub mod emit;
pub mod message_builder;
pub mod recoverable_task;
pub mod spawn;
pub mod task_communication;
pub mod task_context;
pub mod task_error;
pub mod task_id;
pub mod task_metrics;
pub mod task_priority;
pub mod task_relationships;
pub mod task_status;
pub mod timed_task;
pub mod tracing_task;

// Usage-specific traits
pub mod cpu_usage;
pub mod io_usage;
pub mod memory_usage;
pub mod named_task;

// Main task implementation
pub mod tokio_task;

// Re-exports to match API structure
pub use async_task::TokioAsyncTask;
pub use builder::*;
pub use cancellable_task::*;
pub use cpu_usage::*;
pub use emit::task::{TokioEmittingTask, TokioSenderTask, TokioReceiverTask};
pub use emit::TokioFinalEvent;
pub use io_usage::*;
pub use memory_usage::*;
pub use message_builder::*;
pub use named_task::*;
pub use recoverable_task::*;
pub use spawn::*;
pub use task_communication::*;
pub use task_context::*;
pub use task_error::*;
pub use task_id::*;
pub use task_metrics::*;
pub use task_priority::*;
pub use task_relationships::*;
pub use task_status::*;
pub use timed_task::*;
pub use tracing_task::*;
pub use tokio_task::*;
