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

/// Placeholder for the final event implementation
/// Will be fully implemented in a future PR
pub struct TokioFinalEvent<T, C>
where
    T: Send + 'static,
    C: Send + 'static,
{
    data: T,
    results: Vec<C>,
    event_id: Uuid,
    task_id: Uuid,
}

impl<T, C> TokioFinalEvent<T, C>
where
    T: Send + 'static,
    C: Send + 'static,
{
    pub fn new(data: T, results: Vec<C>) -> Self {
        Self {
            data,
            results,
            event_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
        }
    }
}

// This trait implementation will be completed in a future PR
impl<T, C> FinalEvent<T, C, C> for TokioFinalEvent<T, C>
where
    T: Send + 'static,
    C: Clone + Send + 'static,
{
    fn collected(&self) -> &HashMap<Uuid, C> {
        todo!("Not fully implemented yet")
    }

    fn yield_results(&self) -> Vec<C> {
        self.results.clone()
    }
}

// This trait implementation will be completed in a future PR  
impl<T, C> ReceiverEvent<T, C> for TokioFinalEvent<T, C>
where
    T: Send + 'static,
    C: Clone + Send + 'static,
{
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }

    fn task_id(&self) -> &Uuid {
        &self.task_id
    }

    fn data(&self) -> &T {
        &self.data
    }

    fn event_type(&self) -> &StreamingEventType<T> {
        todo!("Not fully implemented yet")
    }

    fn is_final(&self) -> bool {
        true
    }

    fn collector(&self) -> &C {
        todo!("Not fully implemented yet")
    }
}