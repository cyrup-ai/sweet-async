use crate::task::TaskId;
use crate::task::builder::AsyncTaskBuilder;
use crate::task::builder::AsyncWork;
use crate::task::spawn::SpawningTask;

use crate::task::spawn::into_async_result::IntoAsyncResult;

pub trait SpawningTaskBuilder<T: Clone + Send + 'static, E: Send + 'static, I: TaskId>:
    AsyncTaskBuilder
{
    type Task: SpawningTask<T, I>;
    type ParentType;

    /// Set the parent for this task
    ///
    /// This establishes a parent-child relationship where the Orchestra can be its own parent.
    /// Child tasks can communicate with their parent through the TaskRelationships interface.
    fn parent(self, parent: Self::ParentType) -> Self;

    /// Accepts a closure returning T, Result<T, E>, Future<Output = T>, or Future<Output = Result<T, E>>
    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static;

    fn await_result<F, R>(self, work: F) -> Result<T, E>
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static;

    fn await_result_with_handler<F, R, H, Out>(self, work: F, handler: H) -> Out
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
        H: AsyncWork<Out> + Send + 'static;
}
