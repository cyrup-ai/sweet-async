//! Emit module for Tokio implementation of Sweet Async
//!
//! This module contains components for building stream-based tasks.

// These modules are now available for use
pub mod builder;
pub mod collector;
pub mod event;

pub use builder::{
    TokioEmittingTask, TokioEmittingTaskBuilder, TokioReceiverBuilder, TokioSenderBuilder,
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
    /// Unique ID for this final event object itself.
    pub event_id: Uuid,
    /// ID of the emitting task that produced this final event.
    pub task_id: I,
}

impl<TSummary, CCollected, EItem, I: TaskId> TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Send + Sync + 'static,
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
        Self {
            summary_data,
            collected_items,
            event_id: Uuid::new_v4(),
            task_id,
        }
    }
}

// Implementation of the API's FinalEvent trait.
// T_Intermediate from trait -> TSummary here
// C_CollectedItem from trait -> CCollected here (the success type)
// CollectionType from trait -> HashMap<Uuid, Result<CCollected, EItem>> here (the raw collection)
impl<TSummary, CCollected, EItem, I: TaskId>
    FinalEvent<TSummary, CCollected, HashMap<Uuid, Result<CCollected, EItem>>>
    for TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Send + Sync + 'static,
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

// Static definition for the event type when TSummary is ().
static FINAL_EVENT_TYPE_MARKER: StreamingEventType<()> = StreamingEventType::Final(None);

// ReceiverEvent implementation is now specifically for when TSummary is ().
// CContext is HashMap<Uuid, Result<CCollected, EItem>>.
impl<CCollected, EItem, I> ReceiverEvent<(), HashMap<Uuid, Result<CCollected, EItem>>>
    for TokioFinalEvent<(), CCollected, EItem, I>
where
    CCollected: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId + AsRef<Uuid>, // I must be AsRef<Uuid>
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid {
        // This relies on the UuidTaskId implementing AsRef<Uuid> if I is UuidTaskId,
        // or I being Uuid itself.
        self.task_id.as_ref()
    }

    fn data(&self) -> &() {
        // TSummary is ()
        &self.summary_data // which is &()
    }

    fn event_type(&self) -> &StreamingEventType<()> {
        // TSummary is ()
        // Since TSummary is (), we can now correctly return a static reference
        // to StreamingEventType::Final(None).
        &FINAL_EVENT_TYPE_MARKER
    }

    fn is_final(&self) -> bool {
        true
    }

    fn collector(&self) -> &HashMap<Uuid, Result<CCollected, EItem>> {
        self.collected()
    }
}
