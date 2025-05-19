//! Task spawn module for creating future-based tasks
//! 
//! This module provides implementations for spawning asynchronous tasks
//! that return results directly.

pub mod builder;
pub mod result;

pub use builder::TokioSpawningTaskBuilder;
pub use result::{TokioTaskResult, TokioAsyncResult};