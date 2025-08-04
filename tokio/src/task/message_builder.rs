//! Message builder implementation for inter-task communication

use std::collections::HashMap;

use serde_json::Value;
use sweet_async_api::task::{TaskMessageBuilder, TaskId, TaskStatus, AsyncTaskError};
use sweet_async_api::task::task_communication::{TaskEnvelope, TaskMessage};

#[cfg(feature = "cryypt_cipher")]
use cryypt_cipher::Cipher;

/// Tokio-specific message builder that implements TaskMessageBuilder
#[derive(Debug, Clone)]
pub struct TokioMessageBuilder<T: Clone + Send + 'static, I: TaskId> {
    /// Custom hostname for the sender (defaults to system hostname)
    hostname: Option<String>,
    /// Correlation ID for distributed tracing
    correlation_id: Option<String>,
    /// Cipher for encryption
    #[cfg(feature = "cryypt_cipher")]
    cipher: Option<Arc<Cipher>>,
    /// Whether to encrypt the message
    encrypt: bool,
    /// Sender task ID
    sender_id: I,
    /// Legacy content storage (kept for compatibility)
    content: HashMap<String, Value>,
    /// Legacy metadata storage (kept for compatibility)
    metadata: HashMap<String, String>,
    /// Legacy priority field (kept for compatibility)
    priority: i32,
    /// Legacy TTL field (kept for compatibility)
    ttl_ms: Option<u64>,
    /// Type marker
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + 'static, I: TaskId> TokioMessageBuilder<T, I> {
    /// Create a new message builder with sender task ID
    pub fn new(sender_id: I) -> Self {
        Self {
            hostname: None,
            correlation_id: None,
            #[cfg(feature = "cryypt_cipher")]
            cipher: None,
            encrypt: false,
            sender_id,
            content: HashMap::new(),
            metadata: HashMap::new(),
            priority: 0,
            ttl_ms: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new message builder with default system hostname
    pub fn new_with_hostname(sender_id: I) -> Self {
        let hostname = Self::get_system_hostname();
        
        Self {
            hostname: Some(hostname),
            correlation_id: None,
            #[cfg(feature = "cryypt_cipher")]
            cipher: None,
            encrypt: false,
            sender_id,
            content: HashMap::new(),
            metadata: HashMap::new(),
            priority: 0,
            ttl_ms: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get system hostname using standard library methods
    fn get_system_hostname() -> String {
        // Try to get hostname from environment first
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            if !hostname.is_empty() {
                return hostname;
            }
        }
        
        // Fallback to a default hostname
        "tokio-task".to_string()
    }

    /// Helper function to get current system time in nanoseconds since epoch
    fn current_timestamp_nanos() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    /// Helper function to get hostname
    fn get_hostname(&self) -> String {
        self.hostname.clone().unwrap_or_else(Self::get_system_hostname)
    }

    /// Add content to the message
    pub fn with_content(mut self, key: impl Into<String>, value: Value) -> Self {
        self.content.insert(key.into(), value);
        self
    }

    /// Add metadata to the message
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set message priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set time-to-live in milliseconds
    pub fn with_ttl(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = Some(ttl_ms);
        self
    }

    /// Build the final message
    pub fn build(self) -> TokioMessage {
        TokioMessage {
            content: self.content,
            metadata: self.metadata,
            priority: self.priority,
            ttl_ms: self.ttl_ms,
            created_at: std::time::SystemTime::now(),
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> Default for TokioMessageBuilder<T, I>
where
    I: Default,
{
    fn default() -> Self {
        Self::new(I::default())
    }
}

/// Implementation of TaskMessageBuilder trait for TokioMessageBuilder
impl<T: Clone + Send + 'static, I: TaskId> TaskMessageBuilder<T, I> for TokioMessageBuilder<T, I> {
    /// Set a custom hostname (defaults to system hostname)
    fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Set a correlation ID for distributed tracing
    fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set the cipher for encryption
    #[cfg(feature = "cryypt_cipher")]
    fn cipher(mut self, cipher: Arc<Cipher>) -> Self {
        self.cipher = Some(cipher);
        self
    }

    /// Enable encryption for this message
    fn encrypt(mut self) -> Self {
        self.encrypt = true;
        self
    }

    /// Build a data message
    fn data(self, data: T) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::Data(data),
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build a status update message
    fn status(self, status: TaskStatus) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::StatusUpdate(status),
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build an error message
    fn error(self, error: AsyncTaskError) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::Error(error),
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build a cancellation request
    fn cancel_request(self) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::CancelRequest,
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build a cancellation acknowledgment
    fn cancel_ack(self) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::CancelAck,
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build a completion message
    fn completed(self) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::Completed,
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build a heartbeat message
    fn heartbeat(self) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::Heartbeat,
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    /// Build a custom message
    fn custom(self, tag: impl Into<String>, data: Vec<u8>) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::Custom(tag.into(), data),
            is_encrypted: self.encrypt,
            correlation_id: self.correlation_id,
        }
    }

    fn encrypted_data(self, data: Vec<u8>) -> TaskEnvelope<T, I> {
        TaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.get_hostname(),
            timestamp: Self::current_timestamp_nanos(),
            message: TaskMessage::EncryptedData(data),
            is_encrypted: true,
            correlation_id: self.correlation_id,
        }
    }
}

/// Tokio-specific message
#[derive(Debug, Clone)]
pub struct TokioMessage {
    content: HashMap<String, Value>,
    metadata: HashMap<String, String>,
    priority: i32,
    ttl_ms: Option<u64>,
    created_at: std::time::SystemTime,
}

impl TokioMessage {
    /// Get message content
    pub fn content(&self) -> &HashMap<String, Value> {
        &self.content
    }

    /// Get message metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Get message priority
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Get time-to-live
    pub fn ttl_ms(&self) -> Option<u64> {
        self.ttl_ms
    }

    /// Check if message has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_ms {
            if let Ok(elapsed) = self.created_at.elapsed() {
                return elapsed.as_millis() as u64 > ttl;
            }
        }
        false
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.content)
    }
}