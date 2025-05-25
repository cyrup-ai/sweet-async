use std::time::{SystemTime, UNIX_EPOCH};
use sweet_async_api::task::{TaskId, TaskStatus, AsyncTaskError, TaskMessage, TaskEnvelope as ApiTaskEnvelope};
use crate::task::vector_clock::VectorClock;

/// Extended task envelope with vector clock support
#[derive(Clone, Debug)]
pub struct TaskEnvelopeExt<T: Clone + Send + 'static, I: TaskId> {
    /// The base envelope from the API
    pub base: ApiTaskEnvelope<T, I>,
    /// Vector clock for distributed causality
    pub vector_clock: VectorClock<I>,
}

impl<T: Clone + Send + 'static, I: TaskId> TaskEnvelopeExt<T, I> {
    /// Create a new envelope with vector clock
    pub fn new(
        sender_id: I,
        message: TaskMessage<T>,
        vector_clock: VectorClock<I>,
    ) -> Self {
        let hostname = hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
            
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
            
        let base = ApiTaskEnvelope {
            sender_id,
            sender_hostname: hostname,
            timestamp,
            message,
            is_encrypted: false,
            correlation_id: None,
        };
        
        Self {
            base,
            vector_clock,
        }
    }
    
    /// Check if this envelope happened before another
    pub fn happened_before(&self, other: &Self) -> bool {
        self.vector_clock.happened_before(&other.vector_clock)
    }
    
    /// Check if this envelope is concurrent with another
    pub fn is_concurrent(&self, other: &Self) -> bool {
        self.vector_clock.is_concurrent(&other.vector_clock)
    }
    
    /// Get the sender's logical time
    pub fn sender_time(&self) -> u64 {
        self.vector_clock.get_time(&self.base.sender_id, &self.base.sender_hostname)
    }
}

/// Builder for creating envelopes with vector clocks
pub struct EnvelopeBuilder<T: Clone + Send + 'static, I: TaskId> {
    sender_id: I,
    hostname: String,
    vector_clock: VectorClock<I>,
    correlation_id: Option<String>,
    is_encrypted: bool,
}

impl<T: Clone + Send + 'static, I: TaskId> EnvelopeBuilder<T, I> {
    /// Create a new envelope builder
    pub fn new(sender_id: I, vector_clock: VectorClock<I>) -> Self {
        let hostname = hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
            
        Self {
            sender_id,
            hostname,
            vector_clock,
            correlation_id: None,
            is_encrypted: false,
        }
    }
    
    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: String) -> Self {
        self.correlation_id = Some(id);
        self
    }
    
    /// Mark as encrypted
    pub fn encrypted(mut self) -> Self {
        self.is_encrypted = true;
        self
    }
    
    /// Build the envelope with a message
    pub fn build(mut self, message: TaskMessage<T>) -> TaskEnvelopeExt<T, I> {
        // Tick our vector clock before sending
        self.vector_clock.tick(&self.sender_id, &self.hostname);
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
            
        let base = ApiTaskEnvelope {
            sender_id: self.sender_id,
            sender_hostname: self.hostname,
            timestamp,
            message,
            is_encrypted: self.is_encrypted,
            correlation_id: self.correlation_id,
        };
        
        TaskEnvelopeExt {
            base,
            vector_clock: self.vector_clock,
        }
    }
}