//! Tokio backend implementation for Sweet Async API
//!
//! This crate provides Tokio-specific implementations of all traits
//! defined in the Sweet Async API, ensuring exact compliance with
//! the API's interface contracts.

// Core modules
pub mod orchestra;
pub mod task;

// Re-export key types with Tokio prefix
pub use task::task_id::TokioTaskId;

// Type aliases for convenience
pub type TaskId = TokioTaskId;
