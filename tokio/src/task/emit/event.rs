//! Event handling implementation for emitting tasks in Tokio
//!
//! This module provides the implementation for sending and receiving events in stream-based tasks.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use futures::{Future, Sink, Stream};
use sweet_async_api::task::emit::ReceiverEvent;
use tokio::sync::mpsc::{Receiver, Sender};

/// Tokio implementation of event sender
pub struct TokioEventSender<T: Send + 'static> {
    /// Channel sender for events
    sender: Sender<T>,
    /// Flag indicating if the sender is closed
    closed: AtomicBool,
}

impl<T: Send + 'static> TokioEventSender<T> {
    /// Create a new TokioEventSender
    pub fn new(sender: Sender<T>) -> Self {
        Self {
            sender,
            closed: AtomicBool::new(false),
        }
    }

    /// Send an event to the channel
    pub async fn send(&self, item: T) -> Result<(), tokio::sync::mpsc::error::SendError<T>> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(tokio::sync::mpsc::error::SendError(item));
        }
        self.sender.send(item).await
    }

    /// Close the sender
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}

impl<T: Send + 'static> Clone for TokioEventSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            closed: AtomicBool::new(self.closed.load(Ordering::Relaxed)),
        }
    }
}

/// Tokio implementation of event receiver
pub struct TokioEventReceiver<T: Send + 'static> {
    /// Channel receiver for events
    receiver: Receiver<T>,
}

impl<T: Send + 'static> TokioEventReceiver<T> {
    /// Create a new TokioEventReceiver
    pub fn new(receiver: Receiver<T>) -> Self {
        Self { receiver }
    }

    /// Receive the next event from the channel
    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

impl<T: Send + 'static> Stream for TokioEventReceiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

// Sophisticated event type for the Tokio implementation
#[derive(Debug, Clone)]
pub struct TokioEvent<T, C>
where
    T: Send + 'static,
    C: Send + 'static,
{
    event_id: uuid::Uuid,
    task_id: uuid::Uuid,
    data: T,
    event_type: sweet_async_api::task::emit::event::StreamingEventType<T>,
    is_final_event: bool,
    collector: Option<C>,
}

impl<T, C> TokioEvent<T, C>
where
    T: Send + 'static,
    C: Send + 'static,
{
    pub fn new(data: T) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4(),
            task_id: uuid::Uuid::new_v4(),
            data,
            event_type: sweet_async_api::task::emit::event::StreamingEventType::Continue,
            is_final_event: false,
            collector: None,
        }
    }

    pub fn with_event_id(mut self, event_id: uuid::Uuid) -> Self {
        self.event_id = event_id;
        self
    }

    pub fn with_task_id(mut self, task_id: uuid::Uuid) -> Self {
        self.task_id = task_id;
        self
    }

    pub fn with_event_type(
        mut self,
        event_type: sweet_async_api::task::emit::event::StreamingEventType<T>,
    ) -> Self {
        self.event_type = event_type;
        self
    }

    pub fn with_is_final(mut self, is_final: bool) -> Self {
        self.is_final_event = is_final;
        self
    }

    pub fn with_collector(mut self, collector: C) -> Self {
        self.collector = Some(collector);
        self
    }
}

impl<T: Send + 'static, C: Send + 'static> ReceiverEvent<T, C> for TokioEvent<T, C> {
    fn event_id(&self) -> &uuid::Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &uuid::Uuid {
        &self.task_id
    }

    fn data(&self) -> &T {
        &self.data
    }

    fn event_type(&self) -> &sweet_async_api::task::emit::event::StreamingEventType<T> {
        &self.event_type
    }

    fn is_final(&self) -> bool {
        self.is_final_event
    }

    fn collector(&self) -> &C {
        match self.collector.as_ref() {
            Some(collector) => collector,
            None => {
                tracing::error!(
                    event_id = %self.event_id,
                    task_id = %self.task_id,
                    "Collector not available for event - use with_collector() to set one"
                );
                // Since we can't create a C without knowing its type, and the trait contract
                // requires returning &C, we abort instead of panic to avoid unwinding issues
                std::process::abort();
            }
        }
    }
}

// Efficient conversion implementation for backward compatibility
impl<T: Send + 'static, C: Send + 'static> From<T> for TokioEvent<T, C> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

/// Create a channel for event communication
pub fn create_event_channel<T: Send + 'static>(
    buffer_size: usize,
) -> (TokioEventSender<T>, TokioEventReceiver<T>) {
    let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);
    (
        TokioEventSender::new(sender),
        TokioEventReceiver::new(receiver),
    )
}
