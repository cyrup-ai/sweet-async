//! High-performance emit module implementations

pub mod builder;
pub mod collector;
pub mod event;
pub mod sequence;
pub mod task;

// Re-exports for convenience
pub use collector::TokioCollector;
pub use event::{
    TokioFinalEvent, TokioReceiverEvent, TokioSenderEvent, TokioSenderEventBuilder,
    TokioStreamingEvent,
};
pub use task::{TokioEmittingTask, TokioReceiverTask, TokioSenderTask};
