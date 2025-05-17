use std::time::Duration;
use thiserror::Error;

/// Standard error type for task operations
#[derive(Error, Debug)]
pub enum AsyncTaskError {
    #[error("Task timed out after {0:?}")]
    Timeout(Duration),

    #[error("Task was cancelled")]
    Cancelled,

    #[error("Task failed: {0}")]
    Failure(String),

    #[error("Task panicked: {0}")]
    Panic(String),

    #[error("Task rejected: {0}")]
    Rejected(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Invalid task state: {0}")]
    InvalidState(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
