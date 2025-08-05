//! Task ID implementation for the Tokio runtime
//!
//! This module provides a UUID-based task identifier implementation that is
//! optimized for performance and provides strong uniqueness guarantees.

use std::fmt;
use uuid::Uuid;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::message_builder::{MessageBuilderExt, TaskMessageBuilder};

/// UUID-based task identifier
///
/// This implementation uses UUID v4 for strong uniqueness guarantees
/// while maintaining efficient comparison and serialization.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokioTaskId(Uuid);

impl TokioTaskId {
    /// Create a new unique task ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a task ID from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn uuid(&self) -> Uuid {
        self.0
    }

    /// Create a task ID from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }
}

impl Default for TokioTaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskId for TokioTaskId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl fmt::Display for TokioTaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TokioTaskId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<TokioTaskId> for Uuid {
    fn from(task_id: TokioTaskId) -> Self {
        task_id.0
    }
}

/// Sequential task ID implementation for high-performance scenarios
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SequentialTaskId(u64);

impl SequentialTaskId {
    /// Create a new sequential task ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying ID
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl TaskId for SequentialTaskId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        s.parse::<u64>().ok().map(Self)
    }
}

impl fmt::Display for SequentialTaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for SequentialTaskId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<SequentialTaskId> for u64 {
    fn from(task_id: SequentialTaskId) -> Self {
        task_id.0
    }
}

impl<T: Clone + Send + 'static> MessageBuilderExt<T> for TokioTaskId {
    type Builder = crate::task::message_builder::TokioMessageBuilder<T, Self>;

    fn message(&self) -> Self::Builder {
        crate::task::message_builder::TokioMessageBuilder::new(*self)
    }
}

impl<T: Clone + Send + 'static> MessageBuilderExt<T> for SequentialTaskId {
    type Builder = crate::task::message_builder::TokioMessageBuilder<T, Self>;

    fn message(&self) -> Self::Builder {
        crate::task::message_builder::TokioMessageBuilder::new(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokio_task_id_creation() {
        let id1 = TokioTaskId::new();
        let id2 = TokioTaskId::new();
        
        // IDs should be unique
        assert_ne!(id1, id2);
        
        // Should be able to convert to/from string
        let id_str = id1.to_string();
        let parsed_id = TokioTaskId::from_string(&id_str).unwrap();
        assert_eq!(id1, parsed_id);
    }

    #[test]
    fn test_sequential_task_id() {
        let id1 = SequentialTaskId::new(42);
        let id2 = SequentialTaskId::new(43);
        
        assert_ne!(id1, id2);
        assert_eq!(id1.id(), 42);
        
        let id_str = id1.to_string();
        let parsed_id = SequentialTaskId::from_string(&id_str).unwrap();
        assert_eq!(id1, parsed_id);
    }
}