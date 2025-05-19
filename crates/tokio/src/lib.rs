pub mod runtime;
pub mod orchestrator;
pub mod task;
pub mod builder;

// Re-export core components
pub use runtime::TokioRuntime;
pub use orchestrator::TokioOrchestrator;
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

/// Create a new Tokio runtime using the current handle
pub fn new_runtime() -> TokioRuntime {
    TokioRuntime::new()
}

/// Create a new Tokio runtime with custom configuration
pub fn new_runtime_with_config(workers: usize) -> TokioRuntime {
    TokioRuntime::with_config(workers)
}

/// Create a new Tokio orchestrator using the given runtime
pub fn new_orchestrator(runtime: TokioRuntime) -> TokioOrchestrator<(), UuidTaskId> {
    TokioOrchestrator::new(runtime)
}