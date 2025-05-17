mod runtime;
mod orchestrator;
mod task;
mod utils;

// Re-export core components
pub use runtime::TokioRuntime;
pub use orchestrator::TokioOrchestrator;
pub use task::*;
pub use utils::*;

/// Create a new Tokio runtime using the current handle
pub fn new_runtime() -> TokioRuntime {
    TokioRuntime::new()
}

/// Create a new Tokio runtime with custom configuration
pub fn new_runtime_with_config(workers: usize) -> TokioRuntime {
    TokioRuntime::with_config(workers)
}

/// Create a new Tokio orchestrator using the given runtime
pub fn new_orchestrator(runtime: TokioRuntime) -> TokioOrchestrator<(), impl sweet_async_api::task::TaskId> {
    TokioOrchestrator::new(runtime)
}
