use crate::api::task::builder::{AsyncWork, ReceiverStrategy};
use crate::api::task::emit::FinalEvent;

/// A task that emits events with a configurable processing strategy
pub trait SenderTask<T>: Send + 'static {
    type EmittingTask<U>: EmittingTask<T, U>;
    fn with_receiver<C, U>(&self, receiver: C, strategy: ReceiverStrategy) -> Self::EmittingTask<U>
    where
        C: AsyncWork<U> + Send + 'static,
        U: Send + 'static;
}

/// A task that receives and processes events
pub trait ReceiverTask<T>: Send + 'static {
    type EmittingTask<U>: EmittingTask<T, U>;
    fn emit_events<C, U>(&self, receiver: C, strategy: ReceiverStrategy) -> Self::EmittingTask<U>
    where
        C: AsyncWork<U> + Send + 'static,
        U: Send + 'static;
}

/// A task that emits events and provides access to collected results
/// 
/// This trait combines AsyncTask capabilities with event streaming and collection.
/// It allows awaiting the final event along with collected results,
/// checking completion status, and cancellation.
pub trait EmittingTask<T, C>: Send + 'static {
    type Final: FinalEvent<T, C, C>;
    fn await_result(&self) -> Result<C, Self::Final>;
    fn is_complete(&self) -> bool;
    fn cancel(&self) -> bool;
}
