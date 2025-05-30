use std::path::PathBuf;

use crate::orchestra::runtime::Runtime;
use crate::task::TaskRelationships;

/// Task that has execution context information
///
/// The ContextualizedTask trait provides information about the task's
/// execution context, including communication channels with related tasks
/// and runtime environment. This enables decoupled task communication
/// and coordinated execution through message passing.
pub trait ContextualizedTask<T: Clone + Send + 'static, I: crate::task::TaskId> {
    type RuntimeType: Runtime<T, I>;
    type RelationshipsType: TaskRelationships<T, I>;
    
    /// Get communication handles for this task's relationships
    ///
    /// Returns handles for communicating with parent and child tasks
    /// through message passing channels.
    fn relationships(&self) -> &Self::RelationshipsType;

    /// Get mutable communication handles for this task's relationships
    ///
    /// Returns mutable handles for managing parent and child task communications.
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType;

    /// Get the runtime this task is running in
    ///
    /// Returns a reference to the runtime that is executing this task.
    fn runtime(&self) -> &Self::RuntimeType;

    /// Get the current working directory for task execution
    ///
    /// Returns the path that should be used as the working directory
    /// for any filesystem operations performed by this task.
    fn cwd(&self) -> PathBuf;
}
