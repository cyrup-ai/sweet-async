//! Tokio implementation of TaskId trait
//!
//! This module provides the TokioTaskId implementation that exactly
//! matches the TaskId trait defined in the API.

use sweet_async_api::task::TaskId;
use uuid::Uuid;

/// Tokio implementation of TaskId trait
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokioTaskId {
    id: u128, // Use u128 instead of Uuid for Ord trait
}

impl TokioTaskId {
    /// Create a new TokioTaskId
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().as_u128(),
        }
    }
}

impl Default for TokioTaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for TokioTaskId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TokioTaskId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl TaskId for TokioTaskId {
    fn to_string(&self) -> String {
        self.id.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        s.parse::<u128>().ok().map(|id| Self { id })
    }
}
