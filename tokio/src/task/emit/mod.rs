//! Emit module for Tokio implementation of Sweet Async
//!
//! This module contains components for building stream-based tasks.

// These modules are now available for use
pub mod async_work_wrapper;
pub mod channel_builder;
pub mod collector;
pub mod event;
pub mod task;

// Use the task implementations
pub use task::{TokioEmittingTask, TokioSenderTask, TokioReceiverTask};

// Use the channel-based builders
pub use channel_builder::{
    TokioEmittingTaskBuilder, 
    ChannelReceiverBuilder as TokioReceiverBuilder, 
    ChannelSenderBuilder as TokioSenderBuilder,
};

// Use the collector implementation
pub use collector::{TokioCollector, DataSourceConfig};

// Use event types
pub use event::{TokioEvent, TokioEventReceiver, TokioEventSender};

// CSV processing types and extension traits
pub use super::{ChunkSize, Delimiter, DurationExt, RowsExt, CsvRecord, FromCsvLine, CsvParseError};

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
    /// 
    /// # Panics
    /// Panics if the collected_items map contains no successful results, as a FinalEvent
    /// must represent at least some successful collection activity.
    pub fn new(
        summary_data: TSummary,
        collected_items: HashMap<Uuid, Result<CCollected, EItem>>, // Accepts map of Results
        task_id: I,
    ) -> Self {
        let collected_items_count = collected_items.len();
        Self::try_new(summary_data, collected_items, task_id)
            .unwrap_or_else(|error_msg| {
                // Provide comprehensive error context for debugging
                tracing::error!(
                    error = error_msg,
                    task_id = ?task_id,
                    collected_items_count = collected_items_count,
                    "Failed to create TokioFinalEvent - no successful items found"
                );
                
                // Since this method is documented to panic, we provide a clear panic message
                // with additional context for debugging
                panic!(
                    "Cannot create TokioFinalEvent with no successful collected items: {}. \
                     Task ID: {:?}, Total items: {}. \
                     This indicates that all {} collected items resulted in errors.",
                    error_msg,
                    task_id,
                    collected_items_count,
                    collected_items_count
                );
            })
    }

    /// Creates a new TokioFinalEvent, returning an error if no successful items exist.
    /// 
    /// This is the non-panicking version of `new()` that returns a Result.
    pub fn try_new(
        summary_data: TSummary,
        collected_items: HashMap<Uuid, Result<CCollected, EItem>>,
        task_id: I,
    ) -> Result<Self, &'static str> {
        // Extract successful items for caching
        let successful_items: HashMap<Uuid, CCollected> = collected_items
            .iter()
            .filter_map(|(k, v)| v.as_ref().ok().map(|c| (*k, c.clone())))
            .collect();
        
        // Validate that we have at least one successful item
        if successful_items.is_empty() {
            return Err("FinalEvent must have at least one successfully collected item");
        }
            
        let task_uuid = Uuid::new_v4(); // Generate UUID for the task
        Ok(Self {
            summary_data: summary_data.clone(),
            collected_items,
            successful_items,
            event_id: Uuid::new_v4(),
            task_id,
            task_uuid,
            event_type: StreamingEventType::Final(summary_data),
        })
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

// Implementation of FinalEvent<T, C, C> for TokioFinalEvent<T, C, EItem, I>
// This allows the type to work with EmittingTask<T, C, EItem, I>
impl<T, C, EItem, I: TaskId>
    FinalEvent<T, C, C>
    for TokioFinalEvent<T, C, EItem, I>
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


// Generic implementation of ReceiverEvent<T, C> for TokioFinalEvent<T, C, EItem, I>
// This allows the type to work with any T, not just ()
impl<T, C, EItem, I> ReceiverEvent<T, C> for TokioFinalEvent<T, C, EItem, I>
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
        &self.summary_data
    }

    fn event_type(&self) -> &StreamingEventType<T> {
        &self.event_type
    }

    fn is_final(&self) -> bool {
        true
    }

    fn collector(&self) -> &C {
        // Return a reference to the first successful item as the collector
        self.successful_items
            .values()
            .next()
            .unwrap_or_else(|| {
                tracing::error!(
                    event_id = ?self.event_id,
                    task_id = ?self.task_id,
                    successful_items_len = self.successful_items.len(),
                    "TokioFinalEvent collector() called with empty successful_items"
                );
                panic!("TokioFinalEvent::collector() architectural invariant violated")
            })
    }
}

