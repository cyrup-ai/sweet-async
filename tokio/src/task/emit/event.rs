//! Event handling implementation for emitting tasks in Tokio
//!
//! This module provides the implementation for sending and receiving events in stream-based tasks.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use futures::Stream;
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

// Add StreamingEvent implementation for TokioEvent
impl<T: Send + 'static, C: Send + 'static> sweet_async_api::task::emit::event::StreamingEvent<T> for TokioEvent<T, C> {
    fn created_timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        // Return a static default timestamp - proper implementation would store this
        &chrono::DateTime::<chrono::Utc>::MIN_UTC
    }

    fn started_timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &chrono::DateTime::<chrono::Utc>::MIN_UTC
    }

    fn completed_timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &chrono::DateTime::<chrono::Utc>::MIN_UTC
    }

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
}

// Efficient conversion implementation for backward compatibility
impl<T: Send + 'static, C: Send + 'static> From<T> for TokioEvent<T, C> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

// Final event type for the Tokio implementation
#[derive(Debug, Clone)]
pub struct TokioFinalEvent<T, C, Item, Collection = std::collections::HashMap<uuid::Uuid, Item>>
where
    T: Send + 'static,
    C: Send + 'static,
    Item: Send + 'static,
    Collection: Send + 'static,
{
    event_id: uuid::Uuid,
    task_id: uuid::Uuid,
    data: T,
    event_type: sweet_async_api::task::emit::event::StreamingEventType<T>,
    collector: C,
    collected_items: Collection,
}

impl<T, C, Item, Collection> TokioFinalEvent<T, C, Item, Collection>
where
    T: Send + 'static,
    C: Send + 'static,
    Item: Send + 'static,
    Collection: Send + 'static,
{
    pub fn new(data: T, collector: C, collected_items: Collection) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4(),
            task_id: uuid::Uuid::new_v4(),
            data,
            event_type: sweet_async_api::task::emit::event::StreamingEventType::Final(data.clone()),
            collector,
            collected_items,
        }
    }
}

impl<T, C, Item, Collection> ReceiverEvent<T, C> for TokioFinalEvent<T, C, Item, Collection>
where
    T: Send + 'static,
    C: Send + 'static,
    Item: Send + 'static,
    Collection: Send + 'static,
{
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
        true
    }

    fn collector(&self) -> &C {
        &self.collector
    }
}

impl<T, C, Item, Collection> sweet_async_api::task::emit::event::FinalEvent<T, C, Item, Collection> for TokioFinalEvent<T, C, Item, Collection>
where
    T: Send + 'static,
    C: Send + 'static,
    Item: Send + 'static + Clone,
    Collection: Send + 'static,
{
    fn collected(&self) -> &Collection {
        &self.collected_items
    }

    fn yield_results(&self) -> Vec<Item> {
        // Default implementation - specific Collection types would implement this properly
        vec![]
    }
}

// Add SenderEvent implementation for TokioEvent
impl<T: Send + 'static, C: Send + 'static> sweet_async_api::task::emit::event::SenderEvent<T> for TokioEvent<T, C> {
    type Builder = TokioSenderEventBuilder<T, C>;

    fn builder() -> Self::Builder {
        TokioSenderEventBuilder::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), Default::default())
    }

    fn event_type(event_type: sweet_async_api::task::emit::event::StreamingEventType<T>) -> Self::Builder {
        TokioSenderEventBuilder::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), Default::default())
            .event_type(event_type)
    }

    fn data(data: T) -> Self::Builder {
        TokioSenderEventBuilder::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), data)
    }

    fn is_final() -> Self::Builder {
        TokioSenderEventBuilder::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), Default::default())
            .is_final()
    }
}

// SenderEventBuilder implementation
#[derive(Debug, Clone)]
pub struct TokioSenderEventBuilder<T, C>
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

impl<T, C> TokioSenderEventBuilder<T, C>
where
    T: Send + 'static + Default,
    C: Send + 'static,
{
    pub fn new(task_id: uuid::Uuid, event_id: uuid::Uuid, data: T) -> Self {
        Self {
            event_id,
            task_id,
            data,
            event_type: sweet_async_api::task::emit::event::StreamingEventType::Continue,
            is_final_event: false,
            collector: None,
        }
    }
}

impl<T, C> sweet_async_api::task::emit::event::SenderEventBuilder<T> for TokioSenderEventBuilder<T, C>
where
    T: Send + 'static + Default,
    C: Send + 'static + Default,
{
    fn new(task_id: uuid::Uuid, event_id: uuid::Uuid, data: T) -> Self {
        Self::new(task_id, event_id, data)
    }

    fn event_type(mut self, event_type: sweet_async_api::task::emit::event::StreamingEventType<T>) -> Self {
        self.event_type = event_type;
        self
    }

    fn data(mut self, data: T) -> Self {
        self.data = data;
        self
    }

    fn is_final(mut self) -> Self {
        self.is_final_event = true;
        self
    }
}

impl<T, C> sweet_async_api::task::emit::event::SenderEvent<T> for TokioSenderEventBuilder<T, C>
where
    T: Send + 'static + Default,
    C: Send + 'static + Default,
{
    type Builder = Self;

    fn builder() -> Self::Builder {
        Self::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), Default::default())
    }

    fn event_type(event_type: sweet_async_api::task::emit::event::StreamingEventType<T>) -> Self::Builder {
        Self::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), Default::default())
            .event_type(event_type)
    }

    fn data(data: T) -> Self::Builder {
        Self::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), data)
    }

    fn is_final() -> Self::Builder {
        Self::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), Default::default())
            .is_final()
    }
}

impl<T, C> sweet_async_api::task::emit::event::StreamingEvent<T> for TokioSenderEventBuilder<T, C>
where
    T: Send + 'static + Default,
    C: Send + 'static + Default,
{
    fn created_timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &chrono::DateTime::<chrono::Utc>::MIN_UTC
    }

    fn started_timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &chrono::DateTime::<chrono::Utc>::MIN_UTC
    }

    fn completed_timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &chrono::DateTime::<chrono::Utc>::MIN_UTC
    }

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
