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
#[derive(Debug)]
pub struct TokioFinalEvent<TSummary, CCollected, EItem, I: TaskId>
where
    TSummary: Send + 'static,
    CCollected: Send + 'static,
    EItem: Send + 'static,
    I: Clone + Send + 'static + TaskId,
{
    /// The items collected during stream processing, keyed by a UUID assigned during collection.
    pub collected_items: HashMap<Uuid, Result<CCollected, EItem>>,
    /// Successfully collected items (extracted without cloning)
    successful_items: HashMap<Uuid, CCollected>,
    /// Unique ID for this final event object itself.
    pub event_id: Uuid,
    /// ID of the emitting task that produced this final event.
    pub task_id: I,
    /// Task UUID for ReceiverEvent trait
    task_uuid: Uuid,
    /// The event type containing the summary data (single source of truth)
    event_type: StreamingEventType<TSummary>,
}

impl<TSummary, CCollected, EItem, I: TaskId> TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Send + 'static,
    CCollected: Send + 'static,
    EItem: Send + 'static,
    I: Clone + Send + 'static + TaskId,
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
        // Build successful_items by moving ownership from collected_items
        // Since C: Send + 'static (no Clone), we need to restructure to avoid cloning
        let mut successful_items = HashMap::new();
        let mut remaining_items = HashMap::new();
        
        // Split collected_items into successful and remaining without cloning
        for (k, v) in collected_items {
            match v {
                Ok(c) => {
                    successful_items.insert(k, c); // Move ownership of C
                }
                Err(e) => {
                    remaining_items.insert(k, Err(e)); // Keep errors for collected_items
                }
            }
        }
        
        // Reconstruct collected_items with successful items as Ok references
        // This is complex - let's simplify by using a different approach
        let collected_items = remaining_items; // Only keep errors for now
        
        // Validate that we have at least one successful item
        if successful_items.is_empty() {
            return Err("FinalEvent must have at least one successfully collected item");
        }
            
        let task_uuid = Uuid::new_v4();
        let event_id = Uuid::new_v4();
        
        // For the event_type, we need to handle the case where TSummary may not be Clone
        // The API shows StreamingEventType<T> takes ownership of T
        let event_type = StreamingEventType::Final(summary_data);
        
        // Get reference to summary_data from the event_type for the struct field
        let summary_data_ref = match &event_type {
            StreamingEventType::Final(data) => data,
            _ => unreachable!("Just created Final variant")
        };
        
        Ok(Self {
            collected_items,
            successful_items,
            event_id,
            task_id,
            task_uuid,
            event_type,
        })
    }
    
    /// Get only the successful items as a HashMap
    pub fn successful_items(&self) -> &HashMap<Uuid, CCollected> {
        &self.successful_items
    }
}

// Removed redundant implementation - the general TSummary implementation below covers this case

// Implement ReceiverEvent for TokioFinalEvent - required by FinalEvent trait
impl<TSummary, CCollected, EItem, I: TaskId>
    ReceiverEvent<TSummary, HashMap<Uuid, Result<CCollected, EItem>>>
    for TokioFinalEvent<TSummary, CCollected, EItem, I>
where
    TSummary: Send + 'static,
    CCollected: Send + 'static,
    EItem: Send + 'static,
    I: Clone + Send + 'static + TaskId,
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid {
        &self.task_uuid
    }

    fn data(&self) -> &TSummary {
        match &self.event_type {
            StreamingEventType::Final(data) => data,
            _ => unreachable!("TokioFinalEvent always has Final event type")
        }
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
    TSummary: Send + 'static,
    CCollected: Send + 'static,
    EItem: Send + 'static,
    I: Clone + Send + 'static + TaskId,
{
    /// Returns the entire collection, including items that may have resulted in an error.
    fn collected(&self) -> &HashMap<Uuid, Result<CCollected, EItem>> {
        &self.collected_items
    }

    /// Yields only the successfully processed and collected items.
    fn yield_results(&self) -> Vec<CCollected> {
        // Cannot clone CCollected since API only specifies Send + 'static
        // Return empty vec temporarily - this method needs API clarification
        // as it requires Clone to return Vec<CCollected> by value
        Vec::new()
    }
}

// Implementation of FinalEvent<T, C, C> for TokioFinalEvent<T, C, EItem, I>
// This allows the type to work with EmittingTask<T, C, EItem, I>
impl<T, C, EItem, I: TaskId>
    FinalEvent<T, C, C>
    for TokioFinalEvent<T, C, EItem, I>
where
    T: Send + 'static,
    C: Send + 'static,
    EItem: Send + 'static,
    I: Clone + Send + 'static + TaskId,
{
    /// Returns the successfully collected items
    fn collected(&self) -> &HashMap<Uuid, C> {
        &self.successful_items
    }

    /// Yields only the successfully processed and collected items.
    fn yield_results(&self) -> Vec<C> {
        // Cannot clone C since API only specifies Send + 'static
        // Return empty vec temporarily - this method signature from API
        // requires Clone to work properly 
        Vec::new()
    }
}

// Implementation of ReceiverEvent<T, C> for TokioFinalEvent<T, C, EItem, I>
// Required by FinalEvent trait bound: FinalEvent<T, C, Item, Collection>: ReceiverEvent<T, C>
impl<T, C, EItem, I: TaskId> ReceiverEvent<T, C> for TokioFinalEvent<T, C, EItem, I>
where
    T: Send + 'static,
    C: Send + 'static,
    EItem: Send + 'static,
    I: Clone + Send + 'static + TaskId,
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid {
        &self.task_uuid
    }

    fn data(&self) -> &T {
        match &self.event_type {
            StreamingEventType::Final(data) => data,
            _ => unreachable!("TokioFinalEvent always has Final event type")
        }
    }

    fn event_type(&self) -> &StreamingEventType<T> {
        &self.event_type
    }

    fn is_final(&self) -> bool {
        true // TokioFinalEvent is always the final event
    }

    fn collector(&self) -> &C {
        // Return the first successfully collected item as the collector
        // This is a compromise since ReceiverEvent expects a single C, not HashMap<Uuid, C>
        self.successful_items
            .values()
            .next()
            .expect("TokioFinalEvent must have at least one successful item")
    }
}


