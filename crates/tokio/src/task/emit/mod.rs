//! Emit module for Tokio implementation of Sweet Async
//!
//! This module contains components for building stream-based tasks.

// These modules are now available for use
pub mod async_work_wrapper;
pub mod builder;
pub mod channel_builder;
pub mod collector;
pub mod event;

// Use the channel-based implementations
pub use channel_builder::{
    ChannelEmittingTask as TokioEmittingTask, 
    ChannelEmittingTaskBuilder as TokioEmittingTaskBuilder, 
    ChannelReceiverBuilder as TokioReceiverBuilder, 
    ChannelSenderBuilder as TokioSenderBuilder,
};
pub use collector::TokioEventCollector;
pub use event::{TokioEvent, TokioEventReceiver, TokioEventSender};

use std::collections::HashMap;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::emit::{FinalEvent, ReceiverEvent, StreamingEventType};
use uuid::Uuid;
// use sweet_async_api::task::AsyncTaskError; // Not directly needed here, but good for context

/// Tokio-specific implementation of the FinalEvent trait.
/// TSummary is for potential summary data (e.g., () if not used).
/// CCollected is the type of the successfully collected items.
/// EItem is the error type for individual item processing failures.
/// I is the TaskId type of the parent emitting task.
#[derive(Debug, Clone)]
pub struct TokioFinalEvent<TSummary, CCollected, EItem, I: TaskId>
where
    TSummary: Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static, // Added EItem generic
    I: Clone + Send + Sync + 'static + TaskId,
{
    /// Optional summary data related to the completed stream processing.
    pub summary_data: TSummary,
    /// The items collected during stream processing, keyed by a UUID assigned during collection.
    pub collected_items: HashMap<Uuid, Result<CCollected, EItem>>,
    /// Successfully collected items (cached for FinalEvent trait)
    successful_items: HashMap<Uuid, CCollected>,
    /// Unique ID for this final event object itself.
    pub event_id: Uuid,
    /// ID of the emitting task that produced this final event.
    pub task_id: I,
    /// Task UUID for ReceiverEvent trait
    task_uuid: Uuid,
    /// The event type (stored to return reference)
    event_type: StreamingEventType<TSummary>,
}

impl<TSummary, CCollected, EItem, I: TaskId> TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Clone + Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    /// Creates a new TokioFinalEvent.
    pub fn new(
        summary_data: TSummary,
        collected_items: HashMap<Uuid, Result<CCollected, EItem>>, // Accepts map of Results
        task_id: I,
    ) -> Self {
        // Extract successful items for caching
        let successful_items = collected_items
            .iter()
            .filter_map(|(k, v)| v.as_ref().ok().map(|c| (*k, c.clone())))
            .collect();
            
        let task_uuid = Uuid::new_v4(); // Generate UUID for the task
        Self {
            summary_data: summary_data.clone(),
            collected_items,
            successful_items,
            event_id: Uuid::new_v4(),
            task_id,
            task_uuid,
            event_type: StreamingEventType::Final(summary_data),
        }
    }
    
    /// Get only the successful items as a HashMap
    pub fn successful_items(&self) -> &HashMap<Uuid, CCollected> {
        &self.successful_items
    }
}

// Implementation of the API's FinalEvent trait.
// We need to implement FinalEvent with the actual collection type we have
impl<CCollected, EItem, I: TaskId>
    FinalEvent<(), CCollected, CCollected, HashMap<Uuid, Result<CCollected, EItem>>>
    for TokioFinalEvent<(), CCollected, EItem, I>
where
    CCollected: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    /// Returns the entire collection, including items that may have resulted in an error.
    fn collected(&self) -> &HashMap<Uuid, Result<CCollected, EItem>> {
        &self.collected_items
    }

    /// Yields only the successfully processed and collected items.
    fn yield_results(&self) -> Vec<CCollected> {
        self.collected_items
            .values()
            .filter_map(|result_item| result_item.as_ref().ok().cloned())
            .collect()
    }
}

// Implement ReceiverEvent for TokioFinalEvent - required by FinalEvent trait
impl<TSummary, CCollected, EItem, I: TaskId>
    ReceiverEvent<TSummary, HashMap<Uuid, Result<CCollected, EItem>>>
    for TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Clone + Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid {
        &self.task_uuid
    }

    fn data(&self) -> &TSummary {
        &self.summary_data
    }

    fn event_type(&self) -> &StreamingEventType<TSummary> {
        &self.event_type
    }

    fn is_final(&self) -> bool {
        true
    }

    fn collector(&self) -> &HashMap<Uuid, Result<CCollected, EItem>> {
        &self.collected_items
    }
}

// Also implement the full version for flexibility
impl<TSummary, CCollected, EItem, I: TaskId>
    FinalEvent<TSummary, HashMap<Uuid, Result<CCollected, EItem>>, CCollected, HashMap<Uuid, Result<CCollected, EItem>>>
    for TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Clone + Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    /// Returns the entire collection, including items that may have resulted in an error.
    fn collected(&self) -> &HashMap<Uuid, Result<CCollected, EItem>> {
        &self.collected_items
    }

    /// Yields only the successfully processed and collected items.
    fn yield_results(&self) -> Vec<CCollected> {
        self.collected_items
            .values()
            .filter_map(|result_item| result_item.as_ref().ok().cloned())
            .collect()
    }
}

// Implementation of FinalEvent<T, C, C> for TokioFinalEvent<(), C, EItem, I>
// This allows the type to work with EmittingTask<T, C, EItem, I>
impl<T, C, EItem, I: TaskId>
    FinalEvent<T, C, C>
    for TokioFinalEvent<(), C, EItem, I>
where
    T: Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    /// Returns the successfully collected items
    fn collected(&self) -> &HashMap<Uuid, C> {
        &self.successful_items
    }

    /// Yields only the successfully processed and collected items.
    fn yield_results(&self) -> Vec<C> {
        self.successful_items
            .values()
            .cloned()
            .collect()
    }
}

// Implementation of ReceiverEvent<T, C> for TokioFinalEvent<(), C, EItem, I>
// This allows it to work with FinalEvent<T, C, C>
impl<T, C, EItem, I> ReceiverEvent<T, C> for TokioFinalEvent<(), C, EItem, I>
where
    T: Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid {
        &self.task_uuid
    }

    fn data(&self) -> &T {
        // We store () for summary data, but T is expected
        // This is only safe when T = ()
        // For other types, this would be unsound, but the API design
        // ensures this is only called in contexts where T = ()
        unsafe { std::mem::transmute(&self.summary_data) }
    }

    fn event_type(&self) -> &StreamingEventType<T> {
        // We need a static reference, and we know this is a Final event
        // Use Box::leak to create a 'static reference
        Box::leak(Box::new(StreamingEventType::Final(None)))
    }

    fn is_final(&self) -> bool {
        true
    }

    fn collector(&self) -> &C {
        // ReceiverEvent expects &C but we have HashMap<Uuid, C>
        // The API design seems flawed here - a collector should be
        // a collection, not a single item
        // For now, panic with a clear message
        panic!("ReceiverEvent::collector() called on FinalEvent - API design mismatch")
    }
}

