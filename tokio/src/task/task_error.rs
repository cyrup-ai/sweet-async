//! Task error types for the Tokio implementation
//!
//! This module provides comprehensive error handling for all task operations,
//! ensuring that failures are properly categorized and can be handled appropriately.

use std::fmt;
use thiserror::Error;

/// Comprehensive error type for all task operations
///
/// This enum covers all possible failure modes in the task execution system,
/// providing detailed context for error handling and recovery strategies.
#[derive(Error, Debug, Clone)]
pub enum AsyncTaskError {
    /// Task execution failed with a custom error message
    #[error("Task execution failed: {0}")]
    Failure(String),

    /// Task was cancelled before completion
    #[error("Task was cancelled")]
    Cancelled,

    /// Task exceeded its timeout duration
    #[error("Task timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Task panicked during execution
    #[error("Task panicked: {message}")]
    Panic { message: String },

    /// Resource allocation failed
    #[error("Resource allocation failed: {resource}")]
    ResourceExhaustion { resource: String },

    /// Task dependency failed
    #[error("Task dependency failed: {dependency_id}")]
    DependencyFailure { dependency_id: String },

    /// Network or I/O error occurred
    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// Serialization/deserialization error
    #[error("Serialization error: {source}")]
    SerializationError {
        #[from]
        source: serde_json::Error,
    },

    /// Task pool is shut down
    #[error("Task pool is shut down")]
    PoolShutdown,

    /// Task configuration is invalid
    #[error("Invalid task configuration: {details}")]
    ConfigurationError { details: String },

    /// Channel communication error
    #[error("Channel error: {message}")]
    ChannelError { message: String },

    /// Task join error
    #[error("Task join failed: {source}")]
    JoinError {
        #[from]
        source: tokio::task::JoinError,
    },
}

impl AsyncTaskError {
    /// Create a new failure error with a message
    pub fn failure(message: impl Into<String>) -> Self {
        Self::Failure(message.into())
    }

    /// Create a new timeout error
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Create a new panic error
    pub fn panic(message: impl Into<String>) -> Self {
        Self::Panic {
            message: message.into(),
        }
    }

    /// Create a new resource exhaustion error
    pub fn resource_exhaustion(resource: impl Into<String>) -> Self {
        Self::ResourceExhaustion {
            resource: resource.into(),
        }
    }

    /// Create a new dependency failure error
    pub fn dependency_failure(dependency_id: impl Into<String>) -> Self {
        Self::DependencyFailure {
            dependency_id: dependency_id.into(),
        }
    }

    /// Create a new configuration error
    pub fn configuration_error(details: impl Into<String>) -> Self {
        Self::ConfigurationError {
            details: details.into(),
        }
    }

    /// Create a new channel error
    pub fn channel_error(message: impl Into<String>) -> Self {
        Self::ChannelError {
            message: message.into(),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AsyncTaskError::Timeout { .. }
                | AsyncTaskError::ResourceExhaustion { .. }
                | AsyncTaskError::IoError { .. }
                | AsyncTaskError::ChannelError { .. }
        )
    }

    /// Check if this error indicates cancellation
    pub fn is_cancellation(&self) -> bool {
        matches!(self, AsyncTaskError::Cancelled)
    }

    /// Check if this error indicates a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, AsyncTaskError::Timeout { .. })
    }

    /// Check if this error indicates a panic
    pub fn is_panic(&self) -> bool {
        matches!(self, AsyncTaskError::Panic { .. })
    }

    /// Get the error category for metrics and logging
    pub fn category(&self) -> &'static str {
        match self {
            AsyncTaskError::Failure(_) => "failure",
            AsyncTaskError::Cancelled => "cancelled",
            AsyncTaskError::Timeout { .. } => "timeout",
            AsyncTaskError::Panic { .. } => "panic",
            AsyncTaskError::ResourceExhaustion { .. } => "resource_exhaustion",
            AsyncTaskError::DependencyFailure { .. } => "dependency_failure",
            AsyncTaskError::IoError { .. } => "io_error",
            AsyncTaskError::SerializationError { .. } => "serialization_error",
            AsyncTaskError::PoolShutdown => "pool_shutdown",
            AsyncTaskError::ConfigurationError { .. } => "configuration_error",
            AsyncTaskError::ChannelError { .. } => "channel_error",
            AsyncTaskError::JoinError { .. } => "join_error",
        }
    }
}

/// Result type for task operations
pub type TaskResult<T> = Result<T, AsyncTaskError>;

/// Error recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry,
    /// Fall back to an alternative approach
    Fallback,
    /// Fail fast without recovery
    FailFast,
    /// Ignore the error and continue
    Ignore,
}

impl RecoveryStrategy {
    /// Get the appropriate recovery strategy for an error
    pub fn for_error(error: &AsyncTaskError) -> Self {
        match error {
            AsyncTaskError::Timeout { .. } => RecoveryStrategy::Retry,
            AsyncTaskError::ResourceExhaustion { .. } => RecoveryStrategy::Retry,
            AsyncTaskError::IoError { .. } => RecoveryStrategy::Retry,
            AsyncTaskError::ChannelError { .. } => RecoveryStrategy::Retry,
            AsyncTaskError::DependencyFailure { .. } => RecoveryStrategy::Fallback,
            AsyncTaskError::ConfigurationError { .. } => RecoveryStrategy::FailFast,
            AsyncTaskError::Panic { .. } => RecoveryStrategy::FailFast,
            AsyncTaskError::Cancelled => RecoveryStrategy::FailFast,
            AsyncTaskError::PoolShutdown => RecoveryStrategy::FailFast,
            AsyncTaskError::SerializationError { .. } => RecoveryStrategy::FailFast,
            AsyncTaskError::JoinError { .. } => RecoveryStrategy::FailFast,
            AsyncTaskError::Failure(_) => RecoveryStrategy::Fallback,
        }
    }
}