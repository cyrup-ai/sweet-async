use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use crate::api::task::{
    AsyncTask, AsyncTaskError, AsyncTaskHandle, TaskId, TaskPriority
};

/// Task runtime for managing async execution
///
/// The Runtime trait provides an abstraction over different async runtimes
/// (such as Tokio or the standard library) to enable consistent task management.
///
pub trait Runtime<Id: Debug + Send + Sync + 'static, T: Send + 'static> {
    /// Spawn a task with a specific priority
    ///
    /// Schedules a task for execution with an optional priority level. Returns a
    /// handle that can be used to await the result or cancel the task.
    ///
    fn spawn<F>(&self, task: impl AsyncTask<Id, T> + 'static, priority: TaskPriority) -> AsyncTaskHandle<Id, T>
    where
        F: Future<Output = Result<T, AsyncTaskError>> + Send + 'static,
        T: Send + 'static;

    /// Block and wait for a task to complete
    ///
    /// Executes a future to completion on the current thread. This method will
    /// block the current thread until the future completes.
    ///
    fn block_on<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send + 'static;

    /// Get the current number of active tasks
    ///
    /// Returns the number of tasks currently being managed by this runtime.
    ///
    fn active_task_count(&self) -> usize;

    /// Shutdown the runtime, waiting for all tasks to complete
    ///
    /// Attempts to gracefully shut down the runtime, waiting for active tasks
    /// to complete up to the specified timeout duration.
    ///
    /// # Returns
    /// - `true` if all tasks completed successfully within the timeout
    /// - `false` if the timeout was reached and some tasks were still running
    ///
    fn shutdown(&self, timeout: Duration) -> bool;

    /// Check if the runtime is still running
    ///
    /// Returns true if the runtime is active and can accept new tasks,
    /// false if it has been shut down or is in the process of shutting down.
    ///
    fn is_running(&self) -> bool;
    
    /// Get handles to all currently active tasks
    ///
    /// Returns a vector of task handles for all tasks currently being managed
    /// by this runtime.
    ///
    fn active_tasks(&self) -> Vec<AsyncTaskHandle<Id, T>>;
}
