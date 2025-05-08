use std::future::Future;
use crate::AsyncTask;
use crate::api::task::spawn::TaskResult;
use crate::api::task::spawn::AsyncResult;

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
pub trait SpawningTask<T: Send + 'static, I: crate::api::task::TaskId>:
    AsyncTask<T, I> + Send + 'static
{
    type OutputFuture: Future<Output = Self::TaskResult> + Send + 'static;
    type TaskResult: TaskResult<T>;
    type JoinChildrenFuture: Future<Output = Self::JoinChildrenResult> + Send + 'static;
    type JoinChildrenResult: AsyncResult<Vec<I>>;

    /// Spawn a new task from this one
    fn spawn(self, work: Box<dyn FnOnce() -> T + Send + 'static>) -> Self;
    
    /// Spawn a child task 
    fn spawn_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Send + 'static, Self: SpawningTask<R, I>;

    /// Wait for all child tasks to complete
    fn join_children(&self) -> Self::JoinChildrenFuture;
    
    /// Get the task's unique identifier
    fn task_id(&self) -> I;
    
    /// Access the underlying value being created by this task
    fn value(&self) -> Option<&T>;
    
    /// Create a task that can be chained with others
    fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: FnOnce(T) -> U + Send + 'static,
        U: Send + 'static, Self: SpawningTask<U, I>;
}
