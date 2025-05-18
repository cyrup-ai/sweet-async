pub mod runtime;
pub mod orchestrator;
pub mod task;
pub mod builder;

// Re-export core components
pub use runtime::TokioRuntime;
pub use orchestrator::TokioOrchestrator;
pub use runtime::safe_blocking;

/// Create a new Tokio runtime using the current handle
pub fn new_runtime() -> TokioRuntime {
    TokioRuntime::new()
}

/// Create a new Tokio runtime with custom configuration
pub fn new_runtime_with_config(workers: usize) -> TokioRuntime {
    TokioRuntime::with_config(workers)
}

/// Create a new Tokio orchestrator using the given runtime
pub fn new_orchestrator(runtime: TokioRuntime) -> TokioOrchestrator<(), uuid::Uuid> {
    TokioOrchestrator::new(runtime)
}