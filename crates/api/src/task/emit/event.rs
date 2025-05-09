use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// The type of event in the streaming event system
#[derive(Debug, Clone)]
pub enum StreamingEventType<T> {
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
#[allow(dead_code)]
pub trait StreamingEvent<T>: Send + 'static {
    /// When the event was created
    fn created_timestamp(&self) -> &DateTime<Utc>;
    
    /// When the event processing started
    fn started_timestamp(&self) -> &DateTime<Utc>;
    
    /// When the event processing completed
    fn completed_timestamp(&self) -> &DateTime<Utc>;
    
    /// Unique identifier for this event
    fn event_id(&self) -> &Uuid;
    
    /// Identifier of the task this event belongs to
    fn task_id(&self) -> &Uuid;
    
    /// Access the data payload of the event
    fn data(&self) -> &T;
    
    /// The type of this event
    fn event_type(&self) -> &StreamingEventType<T>;
    
    /// Whether this is the final event in a sequence
    fn is_final(&self) -> bool;
}

/// Collects and manages event results
#[allow(dead_code)]
pub trait Collector<T, C>: Send + 'static {
    /// Collect a single item
    fn collect_item(&mut self, item: C);
    
    /// Collect multiple items
    fn collect_items(&mut self, items: Vec<C>);
    
    /// Get all collected items by event ID
    fn collected(&self) -> HashMap<Uuid, C>;
}

/// Builder for sender events - uses a fluent API without explicit build steps
#[allow(dead_code)]
pub trait SenderEventBuilder<T>: SenderEvent<T> + Send + 'static {
    /// Create a new sender event builder
    fn new(task_id: Uuid, event_id: Uuid, data: T) -> Self;
    
    /// Set the event type
    fn with_type(self, event_type: StreamingEventType<T>) -> Self;
    
    /// Update the event data
    fn with_data(self, data: T) -> Self;
    
    /// Mark this as the final event
    fn is_final(self) -> Self;
}

/// Event that can be sent through the system
#[allow(dead_code)]
pub trait SenderEvent<T>: StreamingEvent<T> + Send + 'static {
    type Builder: SenderEventBuilder<T>;
    /// Create a new builder for this event type
    fn builder() -> Self::Builder;
    
    /// Create a builder with specified event type
    fn with_type(event_type: StreamingEventType<T>) -> Self::Builder;
    
    /// Create a builder with specified data
    fn with_data(data: T) -> Self::Builder;
    
    /// Create a builder for a final event
    fn is_final() -> Self::Builder;
}

/// Event that can be received for processing
pub trait ReceiverEvent<T, C>: Send + 'static {
    /// Event identifier
    fn event_id(&self) -> &Uuid;
    
    /// Task identifier
    fn task_id(&self) -> &Uuid;
    
    /// Access the event data
    fn data(&self) -> &T;
    
    /// Get the event type
    fn event_type(&self) -> &StreamingEventType<T>;
    
    /// Whether this is the final event
    fn is_final(&self) -> bool;
    
    /// Access the collector for this event
    fn collector(&self) -> &C;
}

/// The final event in a sequence, containing all collected results
#[allow(dead_code)]
pub trait FinalEvent<T, C, Item>: ReceiverEvent<T, C> + Send + 'static {
    /// Access all collected events
    fn collected(&self) -> &HashMap<Uuid, Item>;
    
    /// Get all collected items as a vector
    fn yield_results(&self) -> Vec<Item>;
}

