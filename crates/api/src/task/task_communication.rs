use std::fmt::Debug;
use std::sync::Arc;
use futures::channel::mpsc;
use crate::task::{TaskId, TaskStatus, AsyncTaskError};

use crate::task::encryption::EncryptionProvider;

/// Envelope containing a message with full context
#[derive(Clone, Debug)]
pub struct TaskEnvelope<T: Clone + Send + 'static, I: TaskId> {
    /// The sender's task ID
    pub sender_id: I,
    /// The sender's hostname
    pub sender_hostname: String,
    /// Timestamp when message was sent (nanoseconds since epoch)
    pub timestamp: u64,
    /// The actual message
    pub message: TaskMessage<T>,
    /// Whether this message is encrypted
    pub is_encrypted: bool,
    /// Optional correlation ID for tracing
    pub correlation_id: Option<String>,
}

/// Messages that can be sent between tasks
#[derive(Clone, Debug)]
pub enum TaskMessage<T: Clone + Send + 'static> {
    /// Status update from a task
    StatusUpdate(TaskStatus),
    
    /// Data payload between tasks
    Data(T),
    
    /// Encrypted data payload (for when T needs to be encrypted)
    EncryptedData(Vec<u8>),
    
    /// Request to cancel
    CancelRequest,
    
    /// Acknowledgment of cancellation
    CancelAck,
    
    /// Error occurred in task
    Error(AsyncTaskError),
    
    /// Task completed successfully
    Completed,
    
    /// Heartbeat/keepalive signal
    Heartbeat,
    
    /// Custom message type for extensibility
    Custom(String, Vec<u8>),
}

/// Channel configuration for task communication
#[derive(Clone)]
pub struct ChannelConfig {
    /// Buffer size for the channel
    pub buffer_size: usize,
    /// Whether to use unbounded channels
    pub unbounded: bool,
    /// Encryption provider (if encryption is enabled)
    pub encryption_provider: Option<Arc<dyn EncryptionProvider>>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            unbounded: false,
            encryption_provider: None,
        }
    }
}

/// Bidirectional communication channel for tasks
pub struct TaskChannel<T: Clone + Send + 'static, I: TaskId> {
    /// Channel to receive enveloped messages
    pub inbox: mpsc::Receiver<TaskEnvelope<T, I>>,
    /// Channel to send enveloped messages
    pub outbox: mpsc::Sender<TaskEnvelope<T, I>>,
    /// This task's ID
    pub task_id: I,
}

/// Sender for communicating with a task
#[derive(Clone)]
pub struct TaskSender<T: Clone + Send + 'static, I: TaskId> {
    /// Channel to send messages to the task
    pub sender: mpsc::Sender<TaskEnvelope<T, I>>,
    /// The sender's task ID (who is sending)
    pub sender_id: I,
    /// The target task's debug name
    pub target_name: Option<String>,
    /// Encryption provider for secure communication
    pub encryption_provider: Option<Arc<dyn EncryptionProvider>>,
}


/// Registry for task relationships using channels
pub struct TaskRelationships<T: Clone + Send + 'static, I: TaskId> {
    /// Sender to communicate with parent task
    pub parent: Option<TaskSender<T, I>>,
    /// Senders to communicate with child tasks
    pub children: Vec<TaskSender<T, I>>,
}

impl<T: Clone + Send + 'static, I: TaskId> Default for TaskRelationships<T, I> {
    fn default() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
        }
    }
}


