use crate::task::builder::ReceiverStrategy;
use crate::task::emit::FinalEvent;

use crate::orchestra::OrchestratorError;
use crate::task::AsyncTask;
/// A task that emits events with a configurable processing strategy
use crate::task::spawn::into_async_result::IntoAsyncResult;
use crate::task::task_id::TaskId;

#[allow(dead_code)]
pub trait SenderTask<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    Send + 'static
{
    type EmittingTaskType: EmittingTask<T, C, E, I>;
    fn receiver<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(/* ... */) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static;
}

#[allow(dead_code)]
pub trait ReceiverTask<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    Send + 'static
{
    type EmittingTaskType: EmittingTask<T, C, E, I>;
    fn emit_events<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(/* ... */) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static;
}

pub trait EmittingTask<T: Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    AsyncTask<T, I> + Send + 'static
{
    type Final: FinalEvent<T, C, C>;

    #[allow(dead_code)]
    fn is_complete(&self) -> bool;
    #[allow(dead_code)]
    fn cancel(&self) -> Result<(), OrchestratorError>;
}
