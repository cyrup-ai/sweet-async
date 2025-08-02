use std::time::Duration;
use thiserror::Error;

/// Standard error type for task operations
#[derive(Debug, Clone, Error)]
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
    
    #[error("Invalid data format")]
    InvalidData,

    #[error("Key version too old, minimum required: {0}")]
    KeyVersionTooOld(u8),
    
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
    
    #[error("IO error: {0}")]
    Io(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for AsyncTaskError {
    fn from(err: std::io::Error) -> Self {
        AsyncTaskError::Io(err.to_string())
    }
}
