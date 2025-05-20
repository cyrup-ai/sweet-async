//! Emit module for Tokio implementation of Sweet Async
//!
//! This module contains components for building stream-based tasks.

// These modules are now available for use
pub mod builder;
pub mod event;
pub mod collector;

pub use builder::{TokioEmittingTaskBuilder, TokioSenderBuilder, TokioReceiverBuilder, TokioEmittingTask};
pub use event::{TokioEventSender, TokioEventReceiver, TokioEvent};
pub use collector::TokioEventCollector;

use std::collections::HashMap;
use uuid::Uuid;
use sweet_async_api::task::emit::{ReceiverEvent, FinalEvent, StreamingEventType};
use sweet_async_api::task::AsyncTaskError; // Added for Result in handler
use sweet_async_api::task::TaskId; // Added TaskId

/// Tokio-specific implementation of the FinalEvent trait.
/// TSummary is for potential summary data (e.g., () if not used).
/// CCollected is the type of the items collected.
/// I is the TaskId type of the parent emitting task.
#[derive(Debug, Clone)] // Added Debug, Clone
pub struct TokioFinalEvent<TSummary, CCollected, I: TaskId> 
where
    TSummary: Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId, // Ensure TaskId bound here too
{
    /// Optional summary data related to the completed stream processing.
    pub summary_data: TSummary, 
    /// The items collected during stream processing, keyed by a UUID assigned during collection.
    pub collected_items: HashMap<Uuid, CCollected>,
    /// Unique ID for this final event object itself.
    pub event_id: Uuid,
    /// ID of the emitting task that produced this final event.
    pub task_id: I,
}

impl<TSummary, CCollected, I: TaskId> TokioFinalEvent<TSummary, CCollected, I> 
where
    TSummary: Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    /// Creates a new TokioFinalEvent.
    pub fn new(
        summary_data: TSummary, 
        collected_items: HashMap<Uuid, CCollected>, 
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
impl<TSummary, CCollected, I: TaskId> 
    FinalEvent<TSummary, CCollected, HashMap<Uuid, CCollected>> 
    for TokioFinalEvent<TSummary, CCollected, I>
where
    TSummary: Send + Sync + 'static,
    CCollected: Clone + Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId,
{
    fn collected(&self) -> &HashMap<Uuid, CCollected> {
        &self.collected_items
    }

    fn yield_results(&self) -> Vec<CCollected> {
        self.collected_items.values().cloned().collect()
    }
}

// Define a static for StreamingEventType::Stop to be used by event_type()
// Assuming TSummary can be represented by () for Stop, or Stop doesn't use its generic for equality.
// The API trait is `event_type(&self) -> &StreamingEventType<TSummary>`. 
// If TSummary is not (), then StreamingEventType::Stop (which is StreamingEventType<()>) is not &StreamingEventType<TSummary>.
// This indicates a deeper issue with expecting a static reference for a generic type variant.
// Let's use a helper that would only work if TSummary is (). This will show the constraint.
const STOP_EVENT_TYPE: StreamingEventType<()> = StreamingEventType::Stop;

// Implementation of ReceiverEvent for TokioFinalEvent.
// This makes it somewhat act like an event itself, as implied by README's handler `|event, collector|`.
// TData for ReceiverEvent -> TSummary (data on the FinalEvent)
// CContext for ReceiverEvent -> HashMap<Uuid, CCollected> (the collection itself, accessed via `collected()`)
impl<TSummary, CCollected, I: TaskId> 
    ReceiverEvent<TSummary, HashMap<Uuid, CCollected>> 
    for TokioFinalEvent<TSummary, CCollected, I>
where
    TSummary: Send + Sync + 'static + Default, // Added Default for event_type placeholder
    CCollected: Clone + Send + Sync + 'static,
    I: Clone + Send + Sync + 'static + TaskId + AsRef<Uuid>, // Added AsRef<Uuid> for task_id()
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid { 
        self.task_id.as_ref()
    }

    fn data(&self) -> &TSummary {
        &self.summary_data
    }

    fn event_type(&self) -> &StreamingEventType<TSummary> { 
        // This is problematic if TSummary is not (). 
        // Returning a static reference to StreamingEventType::Final(Some(self.summary_data)) is not possible.
        // Returning a generic Stop marker if TSummary is not () requires a cast that is unsafe or a different API.
        // If TSummary is constrained to be (), then we can return a static Final(None).
        // Adding Default bound to TSummary to allow creation of StreamingEventType::Final(Some(Default::default()))
        // but we can't return a ref to that temporary. So this doesn't help for a ref.
        
        // Safest static thing to return that indicates termination and matches type *if TSummary = ()*:
        // If TSummary is indeed (), then Final(None) is the most appropriate.
        // Otherwise, this implementation is problematic. The previous `panic!` was more honest.
        // Let's keep the panic to strongly indicate the API/impl mismatch here for general TSummary.
        // A real fix would be API adjustment for `event_type` or `StreamingEventType::Final`.
        if std::any::TypeId::of::<TSummary>() == std::any::TypeId::of::<()>() {
            // This is a runtime check, not ideal. We need a static dispatch way.
            // For now, to make it conceptually work for TSummary = ():
            // We can't easily cast &STOP_EVENT_TYPE (which is &StreamingEventType<()>) to &StreamingEventType<TSummary>
            // unless TSummary IS (). 
            // This entire method implementation for a generic TSummary is flawed given current constraints.
            // The most correct thing given the constraints is to state it cannot be generally implemented.
            unimplemented!(
                "ReceiverEvent::event_type() for TokioFinalEvent cannot return a valid static &StreamingEventType<TSummary> \ 
                 when TSummary is generic and not known to be '()' for StreamingEventType::Final(None) or if a generic 'Stop' is not suitable. \ 
                 API may need `event_type()` to return owned or `StreamingEventType` needs a generic terminal variant."
            );
        } else {
            // If TSummary is not (), we absolutely cannot return a static ref to StreamingEventType::Final(Some(self.summary_data))
            // And casting STOP_EVENT_TYPE is wrong.
            unimplemented!(
                "ReceiverEvent::event_type() for TokioFinalEvent: TSummary is not '()'. See previous comment."
            );
        }
        // One possible path if the API guarantees TSummary is Default and Final can take Option<Default::default()> for static.
        // Or if StreamingEventType::Stop was considered &StreamingEventType<AnythingThatIsTerminal> which is not how generics work.
    }

    fn is_final(&self) -> bool {
        true
    }

    fn collector(&self) -> &HashMap<Uuid, CCollected> {
        self.collected() 
    }
}