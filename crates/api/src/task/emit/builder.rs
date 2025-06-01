use crate::task::builder::{AsyncTaskBuilder, ReceiverStrategy, SenderStrategy};
use crate::task::emit::EmittingTask;
use crate::task::task_id::TaskId;

#[allow(dead_code)]
pub trait EmittingTaskBuilder<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    AsyncTaskBuilder
{
    type SenderBuilder: SenderBuilder<T, C, E, I>;
    /// Sets the sender for the emitting task.
    fn sender<F>(self, sender: F, strategy: SenderStrategy) -> Self::SenderBuilder
    where
        F: crate::task::builder::AsyncWork<T> + Send + 'static;
        
    /// Sets the sender for sequential execution (order-dependent workflows)
    fn sequence<F>(self, sender: F) -> Self::SenderBuilder
    where
        F: crate::task::builder::AsyncWork<T> + Send + 'static;
}

#[allow(dead_code)]
pub trait SenderBuilder<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    type ReceiverBuilder: ReceiverBuilder<T, C, E, I>;
    
    /// Add dependency via closure for zero-allocation access in sender scope
    fn with_dependency<D, F>(self, dependency_fn: F) -> Self
    where
        D: Send + 'static,
        F: FnOnce() -> D + Send + 'static;
    
    /// Add dependency for zero-allocation access in sender scope
    fn with<D>(self, dependency: D) -> Self
    where
        D: Clone + Send + 'static;
    
    /// Configure batch size for automatic chunking
    fn with_batch_size(self, batch_size: usize) -> Self;
        
    /// Sets the receiver for the sender builder.
    fn receiver<F>(self, receiver: F, strategy: ReceiverStrategy) -> Self::ReceiverBuilder
    where
        F: crate::task::builder::AsyncWork<C> + Send + 'static;
}

#[allow(dead_code)]
pub trait ReceiverBuilder<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId> {
    type Task: EmittingTask<T, C, E, I>;
    fn run(self) -> Self::Task;
    fn await_result(
        self,
    ) -> impl Future<Output = (C, <Self::Task as EmittingTask<T, C, E, I>>::Final)> + Send;
}
