use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Duration;

use crate::api::Runtime;
use crate::api::task::{
    CancellableTask, MetricsEnabledTask, PrioritizedTask, TaskId
};

/// Task that has execution context information
///
/// The ContextualizedTask trait provides information about the task's
/// execution context, including parent-child relationships and runtime environment.
/// This enables hierarchical task management and coordinated execution.
pub trait ContextualizedTask<Id: Debug + Send + Sync + 'static, T: Send + 'static>:
    PrioritizedTask<Id, T> + MetricsEnabledTask<Id, T> + CancellableTask<Id, T>
{
    /// Get a list of all child tasks spawned by this task
    ///
    /// Returns the IDs of all tasks that were spawned as children of this task.
    /// Child tasks are automatically canceled if the parent task is canceled.
    fn child_tasks(&self) -> Vec<Id>;

    /// Get this task's parent, if it has one
    ///
    /// Returns the ID of the parent task if this task was spawned as a child task.
    /// If this is a root task, returns None.
    fn parent(&self) -> Option<Id>;

    /// Get the runtime this task is running in
    ///
    /// Returns a reference to the runtime that is executing this task.
    fn runtime(&self) -> &dyn Runtime<Id, T>;

    /// Get the current working directory for task execution
    ///
    /// Returns the path that should be used as the working directory
    /// for any filesystem operations performed by this task.
    fn cwd(&self) -> PathBuf;
}
