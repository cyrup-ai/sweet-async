//! Task spawn module for creating future-based tasks
//!
//! This module provides implementations for spawning asynchronous tasks
//! that return results directly.

pub mod builder;
pub mod result;
pub mod spawning_task;

pub use builder::TokioSpawningTaskBuilder;
pub use result::{TokioAsyncResult, TokioTaskResult};
pub use spawning_task::TokioSpawningTask;
