use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::fmt::Debug;

/// The type of event in the streaming event system
#[derive(Debug, Clone)]
pub enum StreamingEventType<T, Y> {
    /// A final event, containing the final data
    Final(T),
    /// A continuation event, processing continues
    Continue,
    /// An error occurred during processing
    Error(String),
    /// The processing was cancelled
    Cancellation,
}

/// Base trait for all streaming events in the system
pub trait StreamingEvent<T, TaskId, EventId>: Send + 'static 
where 
    TaskId: Debug + Send + Sync + 'static,
    EventId: Debug + Send + Sync + 'static,
    T: Send + 'static,
{
    /// When the event was created
    fn created_timestamp(&self) -> &DateTime<Utc>;
    
    /// When the event processing started
    fn started_timestamp(&self) -> &DateTime<Utc>;
    
    /// When the event processing completed
    fn completed_timestamp(&self) -> &DateTime<Utc>;
    
    /// Unique identifier for this event
    fn event_id(&self) -> &EventId;
    
    /// Identifier of the task this event belongs to
    fn task_id(&self) -> &TaskId;
    
    /// Access the data payload of the event
    fn data(&self) -> &T;
    
    /// The type of this event
    fn event_type(&self) -> &StreamingEventType<T, Y>;
    
    /// Whether this is the final event in a sequence
    fn is_final(&self) -> bool;
}

/// Collects and manages event results
pub trait Collector<T, TaskId, EventId, Item>: Send + 'static 
where 
    TaskId: Debug + Send + Sync + 'static,
    EventId: Debug + Send + Sync + 'static,
    T: Send + 'static,
    Item: Send + 'static,
{
    /// Collect a single item
    fn collect_item(&mut self, item: Item);
    
    /// Collect multiple items
    fn collect_items(&mut self, items: Vec<Item>);
    
    /// Get all collected items by event ID
    fn collected(&self) -> HashMap<EventId, Item>;
}

/// Builder for sender events - uses a fluent API without explicit build steps
pub trait SenderEventBuilder<T, TaskId, EventId>: SenderEvent<T, TaskId, EventId> + Send + 'static 
where 
    TaskId: Debug + Send + Sync + 'static,
    EventId: Debug + Send + Sync + 'static,
    T: Send + 'static,
{
    /// Create a new sender event builder
    fn new(task_id: TaskId, event_id: EventId, data: T) -> Self;
    
    /// Set the event type
    fn with_type(self, event_type: StreamingEventType<T, Y>) -> Self;
    
    /// Update the event data
    fn with_data(self, data: T) -> Self;
    
    /// Mark this as the final event
    fn is_final(self) -> Self;
}

/// Event that can be sent through the system
pub trait SenderEvent<T, TaskId, EventId>: StreamingEvent<T, TaskId, EventId> + Send + 'static
where 
    TaskId: Debug + Send + Sync + 'static,
    EventId: Debug + Send + Sync + 'static,
    T: Send + 'static,
{
    /// Create a new builder for this event type
    fn builder() -> Box<dyn SenderEventBuilder<T, TaskId, EventId>>;
    
    /// Create a builder with specified event type
    fn with_type(event_type: StreamingEventType<T, Y>) -> Box<dyn SenderEventBuilder<T, TaskId, EventId>>;
    
    /// Create a builder with specified data
    fn with_data(data: T) -> Box<dyn SenderEventBuilder<T, TaskId, EventId>>;
    
    /// Create a builder for a final event
    fn is_final() -> Box<dyn SenderEventBuilder<T, TaskId, EventId>>;
}

/// Event that can be received for processing
pub trait ReceiverEvent<T, TaskId, EventId, C>: Send + 'static 
where 
    TaskId: Debug + Send + Sync + 'static,
    EventId: Debug + Send + Sync + 'static,
    T: Send + 'static,
    C: Send + 'static,
{
    /// Event identifier
    fn event_id(&self) -> &EventId;
    
    /// Task identifier
    fn task_id(&self) -> &TaskId;
    
    /// Access the event data
    fn data(&self) -> &T;
    
    /// Get the event type
    fn event_type(&self) -> &StreamingEventType<T, Y>;
    
    /// Whether this is the final event
    fn is_final(&self) -> bool;
    
    /// Access the collector for this event
    fn collector(&self) -> &C;
}

/// The final event in a sequence, containing all collected results
pub trait FinalEvent<T, TaskId, EventId, C, Item>: ReceiverEvent<T, TaskId, EventId, C> + Send + 'static 
where 
    TaskId: Debug + Send + Sync + 'static,
    EventId: Debug + Send + Sync + 'static,
    T: Send + 'static,
    C: Collector<T, TaskId, EventId, Item> + Send + 'static,
    Item: Send + 'static,
{
    /// Access all collected events
    fn collected(&self) -> &HashMap<EventId, Item>;
    
    /// Get all collected items as a vector
    fn yield_results(&self) -> Vec<Item>;
}

