//! Task spawn module for creating future-based tasks
//!
//! This module provides implementations for spawning asynchronous tasks
//! that return results directly.

pub mod builder;
pub mod result;
pub mod spawning_task;
pub mod into_async_result;

pub use builder::TokioSpawningTaskBuilder;
pub use result::{TokioAsyncResult, TokioTaskResult};
pub use spawning_task::TokioSpawningTask;
pub use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
