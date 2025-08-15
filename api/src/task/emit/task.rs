use crate::task::builder::ReceiverStrategy;
use crate::task::emit::FinalEvent;
use crate::task::emit::event::Collector;

use crate::orchestra::OrchestratorError;
use crate::task::AsyncTask;
/// A task that emits events with a configurable processing strategy
use crate::task::spawn::into_async_result::IntoAsyncResult;
use crate::task::task_id::TaskId;

#[allow(dead_code)]
pub trait SenderTask<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    Send + 'static
{
    type EmittingTaskType: EmittingTask<T, C, E, I>;
    fn receiver<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(&T, &mut dyn Collector<T, C>) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static;
}

#[allow(dead_code)]
pub trait ReceiverTask<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    Send + 'static
{
    type EmittingTaskType: EmittingTask<T, C, E, I>;
    fn emit_events<F, R>(&self, receiver: F, strategy: ReceiverStrategy) -> Self::EmittingTaskType
    where
        F: Fn(&T, &mut dyn Collector<T, C>) -> R + Send + 'static,
        R: IntoAsyncResult<C, E> + Send + 'static;
}

pub trait EmittingTask<T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId>:
    AsyncTask<T, I> + Send + 'static
{
    type Final: FinalEvent<T, C, C>;

    #[allow(dead_code)]
    fn is_complete(&self) -> bool;
    #[allow(dead_code)]
    fn cancel(&self) -> Result<(), OrchestratorError>;

    /// Await the final event and apply handler function
    fn await_final_event<Handler, R>(self, handler: Handler) -> R
    where
        Handler: Fn(Self::Final, &dyn Collector<T, C>) -> R + Send + 'static,
        R: Send + 'static;
}
