//! Task module for Tokio implementation of Sweet Async
//!
//! This module contains the implementation of the task-related components
//! for the Tokio runtime, including async tasks, builders, and utility types.

// Core task implementation
pub mod tokio_task;

// Builder pattern implementation
pub mod builder;

// Task execution strategies
pub mod emit;
pub mod spawn;

// Utility modules
pub mod adaptive;
pub mod adaptive_channel;
pub mod async_work;

// Implementation modules
pub mod default_context;
pub mod relationships;

// Re-export the main TokioTask type for convenient access
pub use tokio_task::{TokioTask, TokioAsyncTask};
pub use builder::TokioAsyncTaskBuilder;
