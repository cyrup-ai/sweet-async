use std::future::Future;
use thiserror::Error;

use crate::task::{AsyncTask, AsyncTaskError, TaskId, TaskStatus};

/// Error types for task orchestration operations
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Task {0} not found")]
    TaskNotFound(String),

    #[error("Group {0} not found")]
    GroupNotFound(String),

    #[error("Task {0} already exists")]
    TaskAlreadyExists(String),

    #[error("Group {0} already exists")]
    GroupAlreadyExists(String),

    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Task {0} is not in a state where this operation can be performed")]
    InvalidTaskState(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Task orchestrator for managing multiple related tasks
///
/// The TaskOrchestrator trait provides high-level coordination between multiple
/// AsyncTask instances, supporting dependencies, batching, and lifecycle management.
pub trait TaskOrchestrator<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> {
    type RegisterTaskReturn;
    type StartTaskFuture: Future<Output = Result<T, AsyncTaskError>> + Send;
    type StartAllFuture: Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send;
    type JoinAllFuture: Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send;
    type StartGroupFuture: Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send;

    /// Register a new task with the orchestrator
    ///
    /// Adds a task to the orchestrator's management scope without starting execution.
    ///
    fn register_task(&self, task: Task) -> Self::RegisterTaskReturn;

    /// Add a dependency relationship between tasks
    ///
    /// Specifies that one task depends on another, ensuring the dependent task
    /// won't start until the dependency completes successfully.
    ///
    /// # Errors
    ///
    /// Returns `OrchestratorError::TaskNotFound` if either task doesn't exist.
    /// Returns `OrchestratorError::DependencyCycle` if this would create a dependency cycle.
    /// Returns `OrchestratorError::InvalidTaskState` if a task is in a state where
    /// dependencies can't be added (e.g., already running or completed).
    ///
    fn add_dependency(&self, dependent_id: &I, dependency_id: &I) -> Result<(), OrchestratorError>;

    /// Start execution of a specific task
    ///
    /// Begins execution of the task with the given ID, respecting any dependencies.
    /// Returns a future that completes when the task is finished.
    ///
    fn start_task(&self, task_id: &I) -> Self::StartTaskFuture;

    /// Start execution of all registered tasks
    ///
    /// Begins execution of all registered tasks, respecting dependencies.
    /// Tasks will be scheduled according to their priorities and dependencies.
    ///
    fn start_all(&self) -> Self::StartAllFuture;

    /// Cancel a specific task
    ///
    /// Attempts to cancel the task with the given ID. Also cancels any tasks
    /// that depend on this task.
    ///
    /// Attempts to cancel the task with the given ID. Returns Ok(()) on success, or Err(OrchestratorError) if cancellation failed.
    fn cancel_task(&self, task_id: &I) -> Result<(), OrchestratorError>;

    /// Get the current status of a task
    ///
    /// Returns the current execution status of the task with the given ID.
    ///
    fn task_status(&self, task_id: &I) -> Option<TaskStatus>;

    /// Get all tasks with their current status
    ///
    /// Returns a mapping of task IDs to their current execution status.
    ///
    fn all_task_statuses(&self) -> Vec<(I, TaskStatus)>;

    /// Wait for all tasks to complete
    ///
    /// Returns a future that completes when all registered tasks have finished
    /// execution, either successfully or with an error.
    ///
    fn join_all(&self) -> Self::JoinAllFuture;

    /// Create a task group for batch operations
    ///
    /// Creates a named group that can be used to organize and manage related tasks.
    /// Groups allow operations to be performed on multiple tasks at once.
    ///
    /// Creates a named group for organizing related tasks. Returns Ok(()) if the group was created, or Err(OrchestratorError) if creation failed.
    fn create_group(&self, group_name: &str) -> Result<(), OrchestratorError>;

    /// Add a task to a group
    ///
    /// Associates a task with a named group for batch operations.
    ///
    /// Associates a task with a named group. Returns Ok(()) if successful, or Err(OrchestratorError) if the operation failed.
    fn add_task_to_group(&self, task_id: &I, group_name: &str) -> Result<(), OrchestratorError>;

    /// Start all tasks in a group
    ///
    /// Begins execution of all tasks in the specified group, respecting dependencies.
    ///
    fn start_group(&self, group_name: &str) -> Self::StartGroupFuture;

    /// Cancel all tasks in a group
    ///
    /// Attempts to cancel all tasks in the specified group.
    ///
    fn cancel_group(&self, group_name: &str) -> usize;
}
