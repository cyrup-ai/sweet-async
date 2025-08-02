use crate::task::task_communication::TaskEnvelope;
use crate::task::{AsyncTaskError, TaskId, TaskStatus};

#[cfg(feature = "cryypt")]
use cryypt_cipher::Cipher;

/// Trait for building task messages with a fluent API
pub trait TaskMessageBuilder<T: Clone + Send + 'static, I: TaskId>: Sized {
    /// Set a custom hostname (defaults to system hostname)
    fn hostname(self, hostname: impl Into<String>) -> Self;

    /// Set a correlation ID for distributed tracing
    fn correlation_id(self, id: impl Into<String>) -> Self;

    /// Set the cipher for encryption
    #[cfg(feature = "cryypt")]
    fn cipher(self, cipher: std::sync::Arc<Cipher>) -> Self;

    /// Enable encryption for this message
    fn encrypt(self) -> Self;

    /// Build a data message
    fn data(self, data: T) -> TaskEnvelope<T, I>;

    /// Build a status update message
    fn status(self, status: TaskStatus) -> TaskEnvelope<T, I>;

    /// Build an error message
    fn error(self, error: AsyncTaskError) -> TaskEnvelope<T, I>;

    /// Build a cancellation request
    fn cancel_request(self) -> TaskEnvelope<T, I>;

    /// Build a cancellation acknowledgment
    fn cancel_ack(self) -> TaskEnvelope<T, I>;

    /// Build a completion message
    fn completed(self) -> TaskEnvelope<T, I>;

    /// Build a heartbeat message
    fn heartbeat(self) -> TaskEnvelope<T, I>;

    /// Build a custom message
    fn custom(self, tag: impl Into<String>, data: Vec<u8>) -> TaskEnvelope<T, I>;

    fn encrypted_data(self, data: Vec<u8>) -> TaskEnvelope<T, I>;
}

/// Extension trait for convenient message building
pub trait MessageBuilderExt<T: Clone + Send + 'static>: TaskId + Clone {
    /// The type of message builder to create
    type Builder: TaskMessageBuilder<T, Self>;

    /// Create a message builder for this task ID
    fn message(&self) -> Self::Builder;
}
