use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Stream;
use std::future::Future;
use std::sync::mpsc::{Sender, Receiver, channel};

use crate::api::task::{AsyncTask, AsyncTaskError, TaskResult};
use crate::api::task::builder::{ReceiverStrategy, AsyncWork};
use crate::api::task::emit::{ReceiverEvent, SenderEvent, EventType, FinalEvent};

/// A task that emits events with a configurable processing strategy
pub trait SenderTask<Id, T>: AsyncTask<Id, T> + Send + 'static
where
    Id: Debug + Send + Sync + 'static,
    T: Send + 'static
{
    /// Configure the task with a channel for sending events
    fn with_channel<C>(
        &self, 
        sender: Sender<Box<dyn SenderEvent<T, Id>>>, 
        strategy: ReceiverStrategy
    ) -> impl SenderTask<Id, T> + Send + 'static;
    
    /// Configure the task with a receiver for processing events
    ///
    /// The receiver is responsible for handling events as they're produced.
    /// The strategy controls how events are consumed (serial, parallel, adaptive).
    fn with_receiver<C, R>(
        &self, 
        receiver: C, 
        strategy: ReceiverStrategy
    ) -> impl EmittingTask<Id, T, R> + Send + 'static
    where
        C: AsyncWork<R> + Send + 'static,
        R: Send + 'static;
}

/// A task that receives and processes events
pub trait ReceiverTask<Id, T>: AsyncTask<Id, T> + Send + 'static
where
    Id: Debug + Send + Sync + 'static,
    T: Send + 'static
{
    /// Start the event emission and processing
    fn emit_events<C, R>(
        &self, 
        receiver: C, 
        strategy: ReceiverStrategy
    ) -> impl EmittingTask<Id, T, R> + Send + 'static
    where
        C: AsyncWork<R> + Send + 'static,
        R: Send + 'static;
}

/// A task that emits events and provides access to collected results
/// 
/// This trait combines AsyncTask capabilities with event streaming and collection.
/// It allows awaiting the final event along with collected results,
/// checking completion status, and cancellation.
pub trait EmittingTask<Id: Debug + Send + Sync + 'static, T: Send + 'static, C>: 
    AsyncTask<Id, T> + Send + 'static {

    fn send_and_receive(&self, sender: Sender<ReceiverEvent<T>>, receiver: Receiver<ReceiverEvent<T>>) -> impl Future<Output = (C, FinalEmission<T>)> + Send + 'static;

    /// Wait for the final event and return it along with collected results
    fn await_result(&self) -> Result<C, FinalEvent<T>> + Send + 'static;
    
    /// Check if processing is complete
    fn is_complete(&self) -> bool;
    
    
    /// Cancel processing
    fn cancel(&self) -> bool;
}
