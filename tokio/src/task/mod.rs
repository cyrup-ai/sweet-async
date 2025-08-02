//! Task module for Tokio implementation of Sweet Async
//!
//! This module contains the Tokio-specific implementations of the task-related
//! components defined in the sweet_async_api crate.

use std::time::Duration;

// Core modules matching API structure
pub mod adaptive;
pub mod async_task;
pub mod async_work;
pub mod builder;
pub mod error_fallback;
pub mod cancellable_task;
pub mod default_context;
pub mod emit;
pub mod message_builder;
pub mod recoverable_task;
pub mod spawn;
pub mod task_communication;
pub mod task_context;
pub mod task_id;
pub mod task_metrics;
pub mod relationships;
pub mod task_relationships;
pub mod timed_task;
pub mod tracing_task;

// Usage-specific traits
pub mod cpu_usage;
pub mod io_usage;
pub mod memory_usage;
pub mod named_task;

// Main task implementation
pub mod tokio_task;

// Re-exports to match API structure
pub use async_task::TokioAsyncTask;
pub use async_work::*;
pub use builder::*;
pub use cancellable_task::*;
pub use cpu_usage::*;
pub use emit::task::{TokioEmittingTask, TokioSenderTask, TokioReceiverTask};
pub use emit::TokioFinalEvent;
pub use io_usage::*;
pub use memory_usage::*;
pub use message_builder::*;
pub use named_task::*;
pub use recoverable_task::*;
pub use spawn::*;
pub use task_communication::*;
pub use task_context::*;
pub use task_id::*;
pub use task_metrics::*;
pub use relationships::*;
pub use task_relationships::*;
pub use timed_task::*;
pub use tracing_task::*;
pub use tokio_task::*;

// Extension traits for ergonomic API syntax

/// Extension trait for u64 to create Duration instances with fluent syntax
/// 
/// Enables syntax like `60.seconds()` for ergonomic duration creation.
/// Zero-allocation implementation using inline const expressions.
pub trait DurationExt {
    /// Convert the numeric value to a Duration in seconds
    /// 
    /// # Performance
    /// Inline function with zero allocation, optimized for constant folding
    fn seconds(self) -> Duration;
    
    /// Convert the numeric value to a Duration in milliseconds
    fn milliseconds(self) -> Duration;
    
    /// Convert the numeric value to a Duration in microseconds  
    fn microseconds(self) -> Duration;
}

impl DurationExt for u64 {
    #[inline(always)]
    fn seconds(self) -> Duration {
        Duration::from_secs(self)
    }
    
    #[inline(always)]
    fn milliseconds(self) -> Duration {
        Duration::from_millis(self)
    }
    
    #[inline(always)]
    fn microseconds(self) -> Duration {
        Duration::from_micros(self)
    }
}

impl DurationExt for u32 {
    #[inline(always)]
    fn seconds(self) -> Duration {
        Duration::from_secs(self as u64)
    }
    
    #[inline(always)]
    fn milliseconds(self) -> Duration {
        Duration::from_millis(self as u64)
    }
    
    #[inline(always)]
    fn microseconds(self) -> Duration {
        Duration::from_micros(self as u64)
    }
}

/// Semantic wrapper for chunk sizes in data processing
/// 
/// Provides type safety and clarity over raw numeric values.
/// Zero-allocation enum with optimized variant representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkSize {
    /// Chunk by number of rows/records
    Rows(usize),
    /// Chunk by byte size
    Bytes(usize),
    /// Chunk by time duration
    Duration(Duration),
}

impl ChunkSize {
    /// Get the numeric value for row-based chunking
    #[inline(always)]
    pub const fn rows(&self) -> Option<usize> {
        match self {
            ChunkSize::Rows(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Get the numeric value for byte-based chunking
    #[inline(always)]
    pub const fn bytes(&self) -> Option<usize> {
        match self {
            ChunkSize::Bytes(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Get the duration value for time-based chunking
    #[inline(always)]
    pub const fn duration(&self) -> Option<Duration> {
        match self {
            ChunkSize::Duration(d) => Some(*d),
            _ => None,
        }
    }
}

/// Extension trait for numeric types to create ChunkSize instances
/// 
/// Enables syntax like `100.rows()` for semantic chunking configuration.
/// Zero-allocation implementation with compile-time optimization.
pub trait RowsExt {
    /// Create a row-based chunk size
    fn rows(self) -> ChunkSize;
    
    /// Create a byte-based chunk size
    fn bytes(self) -> ChunkSize;
}

impl RowsExt for u64 {
    #[inline(always)]
    fn rows(self) -> ChunkSize {
        ChunkSize::Rows(self as usize)
    }
    
    #[inline(always)]
    fn bytes(self) -> ChunkSize {
        ChunkSize::Bytes(self as usize)
    }
}

impl RowsExt for u32 {
    #[inline(always)]
    fn rows(self) -> ChunkSize {
        ChunkSize::Rows(self as usize)
    }
    
    #[inline(always)]
    fn bytes(self) -> ChunkSize {
        ChunkSize::Bytes(self as usize)
    }
}

impl RowsExt for usize {
    #[inline(always)]
    fn rows(self) -> ChunkSize {
        ChunkSize::Rows(self)
    }
    
    #[inline(always)]
    fn bytes(self) -> ChunkSize {
        ChunkSize::Bytes(self)
    }
}

/// Delimiter configuration for CSV and text processing
/// 
/// Zero-allocation enum with single byte representation for maximum performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    /// Newline character delimiter (\n)
    NewLine,
    /// Comma delimiter (,)
    Comma,
    /// Tab delimiter (\t)
    Tab,
    /// Custom single character delimiter
    Custom(char),
}

impl Delimiter {
    /// Get the actual character representation of this delimiter
    #[inline(always)]
    pub const fn as_char(self) -> char {
        match self {
            Delimiter::NewLine => '\n',
            Delimiter::Comma => ',',
            Delimiter::Tab => '\t',
            Delimiter::Custom(c) => c,
        }
    }
    
    /// Get the byte representation for efficient parsing
    #[inline(always)]
    pub const fn as_byte(self) -> u8 {
        match self {
            Delimiter::NewLine => b'\n',
            Delimiter::Comma => b',',
            Delimiter::Tab => b'\t',
            Delimiter::Custom(c) => c as u8,
        }
    }
}

// Extension traits are already exported via glob imports above
