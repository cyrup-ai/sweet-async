use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use futures::Stream;
use std::pin::Pin;

use crate::api::task::{
    AsyncResult, AsyncTaskError, CancellableTask, PrioritizedTask, 
    RecoverableTask, TaskId, TaskResult, 
    TaskStatus, TracingTask
};
use crate::api::task::sse::ReceiverEvent;

/// Core trait for all asynchronous tasks
///
/// This trait provides the foundation for all specialized tasks,
/// ensuring they maintain identity, priority, and consistent execution.
pub trait AsyncTask<Id: Debug + Send + Sync + 'static, T: Send + 'static>: 
    PrioritizedTask<Id, T> + 
    CancellableTask<Id, T> +
    TracingTask<Id, T> + 
    RecoverableTask<Id, T>
{
    /// Execute the task and return a Future that resolves to the result
    fn execute(self) -> impl Future<Output = T> + Send + 'static;
    
    /// Get the number of retry attempts configured for this task
    fn retry_attempts(&self) -> u8;
    
    /// Check if tracing is enabled for this task
    fn is_tracing_enabled(&self) -> bool;
    
    /// Get the current status of the task
    fn status(&self) -> TaskStatus;

    /// Await a result from the given task function
    fn await_result<F, R>(&self, task: F) -> AsyncResult<Id, T>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Into<TaskResult<Id, T>> + Send + 'static;
}

