//! High-performance event implementation with zero allocation

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use sweet_async_api::task::emit::event::{
    FinalEvent, ReceiverEvent, SenderEvent, SenderEventBuilder, StreamingEvent, StreamingEventType,
};
use uuid::Uuid;

/// Zero-allocation streaming event implementation
#[derive(Debug, Clone)]
pub struct TokioStreamingEvent<T> {
    event_id: Uuid,
    task_id: Uuid,
    data: T,
    event_type: StreamingEventType<T>,
    created_timestamp: DateTime<Utc>,
    started_timestamp: DateTime<Utc>,
    completed_timestamp: DateTime<Utc>,
    is_final: bool,
}

impl<T> TokioStreamingEvent<T>
where
    T: Clone + Send + 'static,
{
    #[inline]
    pub fn new(task_id: Uuid, event_id: Uuid, data: T) -> Self {
        let now = Utc::now();
        Self {
            event_id,
            task_id,
            data,
            event_type: StreamingEventType::Continue,
            created_timestamp: now,
            started_timestamp: now,
            completed_timestamp: now,
            is_final: false,
        }
    }

    #[inline]
    pub fn with_type(mut self, event_type: StreamingEventType<T>) -> Self {
        self.event_type = event_type;
        self
    }

    #[inline]
    pub fn mark_final(mut self) -> Self {
        self.is_final = true;
        self
    }

    #[inline]
    pub fn mark_started(&mut self) {
        self.started_timestamp = Utc::now();
    }

    #[inline]
    pub fn mark_completed(&mut self) {
        self.completed_timestamp = Utc::now();
    }
}

impl<T> StreamingEvent<T> for TokioStreamingEvent<T>
where
    T: Clone + Send + 'static,
{
    #[inline]
    fn created_timestamp(&self) -> &DateTime<Utc> {
        &self.created_timestamp
    }

    #[inline]
    fn started_timestamp(&self) -> &DateTime<Utc> {
        &self.started_timestamp
    }

    #[inline]
    fn completed_timestamp(&self) -> &DateTime<Utc> {
        &self.completed_timestamp
    }

    #[inline]
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    #[inline]
    fn task_id(&self) -> &Uuid {
        &self.task_id
    }

    #[inline]
    fn data(&self) -> &T {
        &self.data
    }

    #[inline]
    fn event_type(&self) -> &StreamingEventType<T> {
        &self.event_type
    }

    #[inline]
    fn is_final(&self) -> bool {
        self.is_final
    }
}

/// Zero-allocation sender event builder
#[derive(Debug, Clone)]
pub struct TokioSenderEventBuilder<T> {
    event: TokioStreamingEvent<T>,
}

impl<T> TokioSenderEventBuilder<T>
where
    T: Clone + Send + 'static,
{
    #[inline]
    pub fn build(self) -> TokioSenderEvent<T> {
        TokioSenderEvent { event: self.event }
    }
}

impl<T> SenderEventBuilder<T> for TokioSenderEventBuilder<T>
where
    T: Clone + Send + 'static,
{
    #[inline]
    fn new(task_id: Uuid, event_id: Uuid, data: T) -> Self {
        Self {
            event: TokioStreamingEvent::new(task_id, event_id, data),
        }
    }

    #[inline]
    fn event_type(mut self, event_type: StreamingEventType<T>) -> Self {
        self.event = self.event.with_type(event_type);
        self
    }

    #[inline]
    fn data(mut self, data: T) -> Self {
        self.event.data = data;
        self
    }

    #[inline]
    fn is_final(mut self) -> Self {
        self.event = self.event.mark_final();
        self
    }
}

impl<T> StreamingEvent<T> for TokioSenderEventBuilder<T>
where
    T: Clone + Send + 'static,
{
    #[inline]
    fn created_timestamp(&self) -> &DateTime<Utc> {
        self.event.created_timestamp()
    }

    #[inline]
    fn started_timestamp(&self) -> &DateTime<Utc> {
        self.event.started_timestamp()
    }

    #[inline]
    fn completed_timestamp(&self) -> &DateTime<Utc> {
        self.event.completed_timestamp()
    }

    #[inline]
    fn event_id(&self) -> &Uuid {
        self.event.event_id()
    }

    #[inline]
    fn task_id(&self) -> &Uuid {
        self.event.task_id()
    }

    #[inline]
    fn data(&self) -> &T {
        self.event.data()
    }

    #[inline]
    fn event_type(&self) -> &StreamingEventType<T> {
        self.event.event_type()
    }

    #[inline]
    fn is_final(&self) -> bool {
        self.event.is_final()
    }
}

impl<T> SenderEvent<T> for TokioSenderEventBuilder<T>
where
    T: Clone + Send + 'static,
{
    type Builder = TokioSenderEventBuilder<T>;

    #[inline]
    fn builder() -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), unsafe {
            std::mem::zeroed()
        })
    }

    #[inline]
    fn event_type(event_type: StreamingEventType<T>) -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), unsafe {
            std::mem::zeroed()
        })
        .event_type(event_type)
    }

    #[inline]
    fn data(data: T) -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), data)
    }

    #[inline]
    fn is_final() -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), unsafe {
            std::mem::zeroed()
        })
        .is_final()
    }
}

/// Zero-allocation sender event implementation
#[derive(Debug, Clone)]
pub struct TokioSenderEvent<T> {
    event: TokioStreamingEvent<T>,
}

impl<T> StreamingEvent<T> for TokioSenderEvent<T>
where
    T: Clone + Send + 'static,
{
    #[inline]
    fn created_timestamp(&self) -> &DateTime<Utc> {
        self.event.created_timestamp()
    }

    #[inline]
    fn started_timestamp(&self) -> &DateTime<Utc> {
        self.event.started_timestamp()
    }

    #[inline]
    fn completed_timestamp(&self) -> &DateTime<Utc> {
        self.event.completed_timestamp()
    }

    #[inline]
    fn event_id(&self) -> &Uuid {
        self.event.event_id()
    }

    #[inline]
    fn task_id(&self) -> &Uuid {
        self.event.task_id()
    }

    #[inline]
    fn data(&self) -> &T {
        self.event.data()
    }

    #[inline]
    fn event_type(&self) -> &StreamingEventType<T> {
        self.event.event_type()
    }

    #[inline]
    fn is_final(&self) -> bool {
        self.event.is_final()
    }
}

impl<T> SenderEvent<T> for TokioSenderEvent<T>
where
    T: Clone + Send + 'static,
{
    type Builder = TokioSenderEventBuilder<T>;

    #[inline]
    fn builder() -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), unsafe {
            std::mem::zeroed()
        })
    }

    #[inline]
    fn event_type(event_type: StreamingEventType<T>) -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), unsafe {
            std::mem::zeroed()
        })
        .event_type(event_type)
    }

    #[inline]
    fn data(data: T) -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), data)
    }

    #[inline]
    fn is_final() -> Self::Builder {
        TokioSenderEventBuilder::new(Uuid::new_v4(), Uuid::new_v4(), unsafe {
            std::mem::zeroed()
        })
        .is_final()
    }
}

/// Zero-allocation receiver event implementation
#[derive(Debug)]
pub struct TokioReceiverEvent<T, C> {
    event_id: Uuid,
    task_id: Uuid,
    data: T,
    event_type: StreamingEventType<T>,
    collector: C,
    is_final: bool,
}

impl<T, C> TokioReceiverEvent<T, C>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
{
    #[inline]
    pub fn new(event_id: Uuid, task_id: Uuid, data: T, collector: C) -> Self {
        Self {
            event_id,
            task_id,
            data,
            event_type: StreamingEventType::Continue,
            collector,
            is_final: false,
        }
    }

    #[inline]
    pub fn with_type(mut self, event_type: StreamingEventType<T>) -> Self {
        self.event_type = event_type;
        self
    }

    #[inline]
    pub fn mark_final(mut self) -> Self {
        self.is_final = true;
        self
    }
}

impl<T, C> ReceiverEvent<T, C> for TokioReceiverEvent<T, C>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
{
    #[inline]
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    #[inline]
    fn task_id(&self) -> &Uuid {
        &self.task_id
    }

    #[inline]
    fn data(&self) -> &T {
        &self.data
    }

    #[inline]
    fn event_type(&self) -> &StreamingEventType<T> {
        &self.event_type
    }

    #[inline]
    fn is_final(&self) -> bool {
        self.is_final
    }

    #[inline]
    fn collector(&self) -> &C {
        &self.collector
    }
}

/// Zero-allocation final event implementation
#[derive(Debug)]
pub struct TokioFinalEvent<T, C, Item> {
    receiver_event: TokioReceiverEvent<T, C>,
    collected_items: HashMap<Uuid, Item>,
}

impl<T, C, Item> TokioFinalEvent<T, C, Item>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
    Item: Clone + Send + 'static,
{
    #[inline]
    pub fn new(data: T, collector: C, collected_items: HashMap<Uuid, Item>) -> Self {
        Self {
            receiver_event: TokioReceiverEvent::new(
                Uuid::new_v4(),
                Uuid::new_v4(),
                data,
                collector,
            )
            .mark_final(),
            collected_items,
        }
    }
}

impl<T, C, Item> ReceiverEvent<T, C> for TokioFinalEvent<T, C, Item>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
    Item: Clone + Send + 'static,
{
    #[inline]
    fn event_id(&self) -> &Uuid {
        self.receiver_event.event_id()
    }

    #[inline]
    fn task_id(&self) -> &Uuid {
        self.receiver_event.task_id()
    }

    #[inline]
    fn data(&self) -> &T {
        self.receiver_event.data()
    }

    #[inline]
    fn event_type(&self) -> &StreamingEventType<T> {
        self.receiver_event.event_type()
    }

    #[inline]
    fn is_final(&self) -> bool {
        true
    }

    #[inline]
    fn collector(&self) -> &C {
        self.receiver_event.collector()
    }
}

impl<T, C, Item> FinalEvent<T, C, Item> for TokioFinalEvent<T, C, Item>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
    Item: Clone + Send + 'static,
{
    #[inline]
    fn collected(&self) -> &HashMap<Uuid, Item> {
        &self.collected_items
    }

    #[inline]
    fn yield_results(&self) -> Vec<Item> {
        self.collected_items.values().cloned().collect()
    }
}
