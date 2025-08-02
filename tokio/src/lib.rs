pub mod orchestra;
pub mod task;
pub mod task_id_uuid;

#[cfg(test)]
mod validation_test;


// Re-export core components
pub use orchestra::TokioOrchestratorBuilder;
pub use orchestra::runtime::TokioRuntime;
pub use task::{TokioAsyncTask, TokioEmittingTask, TokioSenderTask, TokioReceiverTask, TokioFinalEvent};
pub use task::builder::TokioAsyncTaskBuilder;
pub use task_id_uuid::*;

// Re-export extension traits from our task module for user convenience
pub use task::{DurationExt, RowsExt, ChunkSize, Delimiter};

// Type alias to enable README.md syntax: AsyncTask::to::<T>() and AsyncTask::emits::<T>()
pub type AsyncTask<T, I = UuidTaskId> = TokioAsyncTask<T, I>;
