use crate::task::builder::{AsyncTaskBuilder, ReceiverStrategy, SenderStrategy};
use crate::task::emit::EmittingTask;
use crate::task::task_id::TaskId;

#[allow(dead_code)]
pub trait EmittingTaskBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    AsyncTaskBuilder
{
    type SenderBuilder: SenderBuilder<T, C, E, I>;
    /// Sets the sender for the emitting task.
    fn sender<F>(self, sender: F, strategy: SenderStrategy) -> Self::SenderBuilder
    where
        F: crate::task::builder::AsyncWork<T> + Send + 'static;
}

#[allow(dead_code)]
pub trait SenderBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    type ReceiverBuilder: ReceiverBuilder<T, C, E, I>;
    /// Sets the receiver for the sender builder.
    fn receiver<F>(self, receiver: F, strategy: ReceiverStrategy) -> Self::ReceiverBuilder
    where
        F: crate::task::builder::AsyncWork<C> + Send + 'static;
}

#[allow(dead_code)]
pub trait ReceiverBuilder<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    type Task: EmittingTask<T, C, E, I>;
    fn run(self) -> Self::Task;
    fn await_result(
        self,
    ) -> impl Future<Output = (C, <Self::Task as EmittingTask<T, C, E, I>>::Final)> + Send;
}
