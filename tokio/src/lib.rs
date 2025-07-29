pub mod orchestra;
pub mod runtime;
pub mod task;
pub mod task_id_uuid;


// Re-export core components
pub use orchestra::{TokioOrchestratorBuilder, TokioOrchestrator};
pub use runtime::TokioRuntime;
pub use runtime::safe_blocking;
pub use task::{TokioAsyncTask, TokioEmittingTask, TokioSenderTask, TokioReceiverTask, TokioFinalEvent};
pub use task::builder::TokioAsyncTaskBuilder;
pub use task_id_uuid::*;

/// Create a new Tokio runtime using the current handle
pub fn new_runtime() -> TokioRuntime {
    TokioRuntime::new()
}

/// Create a new Tokio runtime with custom configuration
pub fn new_runtime_with_config(workers: usize) -> TokioRuntime {
    TokioRuntime::with_config(workers)
}
