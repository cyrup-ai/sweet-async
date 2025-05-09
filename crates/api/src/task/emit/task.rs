use crate::task::builder::{AsyncWork, ReceiverStrategy};
use crate::task::emit::FinalEvent;

/// A task that emits events with a configurable processing strategy
#[allow(dead_code)]
pub trait SenderTask<T>: Send + 'static {
    type EmittingTask<U>: EmittingTask<T, U>;
    fn with_receiver<C, U>(&self, receiver: C, strategy: ReceiverStrategy) -> Self::EmittingTask<U>
    where
        C: AsyncWork<U> + Send + 'static,
        U: Send + 'static;
}

/// A task that receives and processes events
#[allow(dead_code)]
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
    #[allow(dead_code)]
    fn await_result(&self) -> Result<C, Self::Final>;
    #[allow(dead_code)]
    fn is_complete(&self) -> bool;
    #[allow(dead_code)]
    fn cancel(&self) -> bool;
}
