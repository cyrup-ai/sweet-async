pub mod builder;
pub mod runtime;
pub mod task;

// Re-export core components
pub use runtime::TokioRuntime;
pub use runtime::safe_blocking;

use sweet_async_api::task::TaskId;

// Implement TaskId for uuid::Uuid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UuidTaskId(pub uuid::Uuid);

impl TaskId for UuidTaskId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        uuid::Uuid::parse_str(s).ok().map(UuidTaskId)
    }
}

// Add AsRef<Uuid> implementation for UuidTaskId
impl AsRef<uuid::Uuid> for UuidTaskId {
    fn as_ref(&self) -> &uuid::Uuid {
        &self.0
    }
}

/// Create a new Tokio runtime using the current handle
pub fn new_runtime() -> TokioRuntime {
    TokioRuntime::new()
}

/// Create a new Tokio runtime with custom configuration
pub fn new_runtime_with_config(workers: usize) -> TokioRuntime {
    TokioRuntime::with_config(workers)
}
