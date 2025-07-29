//! Message builder implementation for inter-task communication

use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use sweet_async_api::task::{TaskMessageBuilder, TaskId};

/// Tokio-specific message builder
#[derive(Debug, Clone)]
pub struct TokioMessageBuilder {
    content: HashMap<String, Value>,
    metadata: HashMap<String, String>,
    priority: i32,
    ttl_ms: Option<u64>,
}

impl TokioMessageBuilder {
    /// Create a new message builder
    pub fn new() -> Self {
        Self {
            content: HashMap::new(),
            metadata: HashMap::new(),
            priority: 0,
            ttl_ms: None,
        }
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

impl Default for TokioMessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Implementation of TaskMessageBuilder would go here when we have the proper generic parameters
// For now, we'll keep the builder as a simple utility struct

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