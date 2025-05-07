use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::api::task::{AsyncTask, TaskId, AsyncResult, AsyncTaskError, TaskResult};

/// A specialized AsyncTask that can be awaited
///
/// SpawningTask is a specialized AsyncTask that implements Future,
/// allowing it to be directly awaited. When awaited, it yields 
/// a TaskResult containing both the result value and maintaining
/// task identity.
///
/// This enables structured concurrency with parent-child relationships,
/// chained task execution, cancellation propagation, and resource 
/// lifecycle management.
pub trait SpawningTask<Id: Debug + Send + Sync + 'static, T: Send + 'static>: 
    AsyncTask<Id, T> + Future<Output = impl TaskResult<Id, T>> + Send + 'static 
{
    /// Spawn a new task from this one
    ///
    /// Returns a new specialized AsyncTask that can be awaited.
    fn spawn<W>(&self, work: W) -> impl SpawningTask<Id, T> + Future<Output = impl TaskResult<Id, T>> + Send + 'static
    where
        W: crate::api::task::builder::AsyncWork<T> + Send + 'static;
        
    /// Spawn a child task 
    ///
    /// Returns a child task that inherits properties from the parent
    /// and maintains a parent-child relationship.
    fn spawn_child<R>(&self, task: R) -> impl SpawningTask<Id, R> + Future<Output = impl TaskResult<Id, R>> + Send + 'static
    where
        R: Send + 'static;

    /// Wait for all child tasks to complete
    ///
    /// Returns a future that resolves when all child tasks of this task have completed.
    fn join_children(&self) -> impl Future<Output = impl AsyncResult<Vec<Id>, AsyncTaskError>> + Send + 'static;
    
    /// Get the task's unique identifier
    fn task_id(&self) -> Id;
    
    /// Access the underlying value being created by this task
    fn value(&self) -> Option<&T>;
    
    /// Create a task that can be chained with others
    fn chain<U, F>(self, f: F) -> impl SpawningTask<Id, U> + Future<Output = impl TaskResult<Id, U>> + Send + 'static
    where
        F: FnOnce(T) -> U + Send + 'static,
        U: Send + 'static;
}
