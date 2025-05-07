use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use crate::api::task::{
    AsyncTaskError, TaskId, TaskPriority, TaskStatus, AsyncTask
};

/// Task orchestrator for managing multiple related tasks
///
/// The TaskOrchestrator trait provides high-level coordination between multiple
/// AsyncTask instances, supporting dependencies, batching, and lifecycle management.
pub trait TaskOrchestrator<Id: Debug + Send + Sync + 'static, T: Send + 'static> {
    /// Register a new task with the orchestrator
    ///
    /// Adds a task to the orchestrator's management scope without starting execution.
    /// 
    fn register_task(&self, task: impl AsyncTask<Id, T> + 'static) -> Id;
    
    /// Add a dependency relationship between tasks
    ///
    /// Specifies that one task depends on another, ensuring the dependent task
    /// won't start until the dependency completes successfully.
    ///
    fn add_dependency(&self, dependent_id: &Id, dependency_id: &Id) -> bool;
    
    /// Start execution of a specific task
    ///
    /// Begins execution of the task with the given ID, respecting any dependencies.
    /// Returns a future that completes when the task is finished.
    ///
    fn start_task(&self, task_id: &Id) -> impl Future<Output = Result<T, AsyncTaskError>> + Send;
    
    /// Start execution of all registered tasks
    ///
    /// Begins execution of all registered tasks, respecting dependencies.
    /// Tasks will be scheduled according to their priorities and dependencies.
    ///
    fn start_all(&self) -> impl Future<Output = Vec<(Id, Result<T, AsyncTaskError>)>> + Send;
    
    /// Cancel a specific task
    ///
    /// Attempts to cancel the task with the given ID. Also cancels any tasks
    /// that depend on this task.
    ///
    fn cancel_task(&self, task_id: &Id) -> bool;
    
    /// Get the current status of a task
    ///
    /// Returns the current execution status of the task with the given ID.
    ///
    fn task_status(&self, task_id: &Id) -> Option<TaskStatus>;
    
    /// Get all tasks with their current status
    ///
    /// Returns a mapping of task IDs to their current execution status.
    ///
    fn all_task_statuses(&self) -> Vec<(Id, TaskStatus)>;
    
    /// Wait for all tasks to complete
    ///
    /// Returns a future that completes when all registered tasks have finished
    /// execution, either successfully or with an error.
    ///
    fn join_all(&self) -> impl Future<Output = Vec<(Id, Result<T, AsyncTaskError>)>> + Send;
    
    /// Create a task group for batch operations
    ///
    /// Creates a named group that can be used to organize and manage related tasks.
    /// Groups allow operations to be performed on multiple tasks at once.
    ///
    fn create_group(&self, group_name: &str) -> bool;
    
    /// Add a task to a group
    ///
    /// Associates a task with a named group for batch operations.
    ///
    fn add_task_to_group(&self, task_id: &Id, group_name: &str) -> bool;
    
    /// Start all tasks in a group
    ///
    /// Begins execution of all tasks in the specified group, respecting dependencies.
    ///
    fn start_group(&self, group_name: &str) -> impl Future<Output = Vec<(Id, Result<T, AsyncTaskError>)>> + Send;
    
    /// Cancel all tasks in a group
    ///
    /// Attempts to cancel all tasks in the specified group.
    ///
    fn cancel_group(&self, group_name: &str) -> usize;
}
