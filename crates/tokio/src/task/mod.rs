//! Task module for Tokio implementation of Sweet Async
//!
//! This module contains the implementation of the task-related components
//! for the Tokio runtime, including async tasks, builders, and utility types.

// Core task implementation
pub mod async_task;

// Task trait implementations - removed to avoid conflicts with async_task implementations
// pub mod cancellable_task;
// pub mod recoverable_task;
// pub mod timed_task;
// pub mod tracing_task;
// pub mod task_context;

// Builder pattern implementation
pub mod builder;

// Task execution strategies
pub mod emit;
pub mod spawn;

// Utility modules
pub mod adaptive;
pub mod async_util;
pub mod async_work;

// No re-exports - use the modules directly via their clean interfaces

// Re-export the main AsyncTask type for convenient access
pub use async_task::AsyncTask;

// Re-export extension traits for additional utilities - removed to avoid conflicts
// pub use cancellable_task::*;
// pub use recoverable_task::*;
// pub use timed_task::*;
// pub use tracing_task::*;
// pub use task_context::*;
