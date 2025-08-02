//! TaskId implementation wrapper for `uuid::Uuid` so that Sweet Async can use UUIDs
//! as task identifiers out‐of‐the‐box.

use sweet_async_api::task::TaskId;

/// Wrapper around uuid::Uuid to implement TaskId trait (orphan rule compliance)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UuidTaskId(uuid::Uuid);

impl UuidTaskId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
    
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
    
    pub fn uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for UuidTaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<uuid::Uuid> for UuidTaskId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl TaskId for UuidTaskId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        uuid::Uuid::parse_str(s).ok().map(Self)
    }
}
