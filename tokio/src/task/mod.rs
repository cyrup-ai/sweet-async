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
pub mod task_relationships_impl;
pub mod timed_task;
pub mod tracing_task;

// Usage-specific traits
pub mod cpu_usage;
pub mod io_usage;
pub mod memory_usage;
pub mod named_task;
pub mod prioritized_task;
pub mod status_enabled_task;

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
pub use emit::collector::{TokioCollector, DataSourceConfig};
pub use io_usage::*;
pub use memory_usage::*;
pub use message_builder::*;
pub use named_task::*;
pub use prioritized_task::*;
pub use status_enabled_task::*;
pub use recoverable_task::*;
pub use spawn::*;
pub use task_communication::*;
pub use task_context::*;
pub use task_id::*;
pub use task_metrics::*;
pub use relationships::TokioTaskRelationships;
pub use task_relationships::TaskRelationshipManager;
pub use task_relationships_impl::TokioTaskRelationships;

pub use timed_task::*;
pub use tracing_task::*;
pub use tokio_task::*;

// CSV processing types (exported via emit module)

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

/// Zero-allocation CSV record for efficient parsing and processing
/// 
/// Uses string slices for zero-copy parsing where the record references
/// data from the source buffer without allocation overhead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRecord<'a> {
    /// Unique identifier for this record (typically first field or line number)
    pub id: &'a str,
    /// Raw data fields as zero-copy string slices
    pub data: &'a [&'a str],
    /// Source line number for debugging and error reporting
    pub line_number: usize,
}

impl<'a> CsvRecord<'a> {
    /// Create a new CSV record with zero allocation
    /// 
    /// # Performance
    /// Uses string slices for zero-copy construction
    #[inline(always)]
    pub const fn new(id: &'a str, data: &'a [&'a str], line_number: usize) -> Self {
        Self { id, data, line_number }
    }
    
    /// Get the number of fields in this record
    #[inline(always)]
    pub const fn field_count(&self) -> usize {
        self.data.len()
    }
    
    /// Get a field by index with bounds checking
    #[inline(always)]
    pub fn get_field(&self, index: usize) -> Option<&'a str> {
        self.data.get(index).copied()
    }
    
    /// Check if this record is valid (has data)
    #[inline(always)]
    pub const fn is_valid(&self) -> bool {
        !self.data.is_empty() && !self.id.is_empty()
    }
}

/// Type-safe trait for parsing CSV lines into structured records
/// 
/// Enables zero-allocation parsing by working with string slices
/// and reused parsing buffers for maximum performance.
pub trait FromCsvLine<'a>: Sized {
    /// Error type for parsing failures
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Parse a CSV line into a typed record
    /// 
    /// # Arguments
    /// * `line` - The raw CSV line as a string slice
    /// * `line_number` - Line number for error reporting
    /// * `delimiter` - The delimiter configuration to use
    /// * `field_buffer` - Reusable buffer for field parsing (zero allocation)
    /// 
    /// # Performance
    /// Implementations should use the provided field_buffer to avoid
    /// allocations during parsing. The buffer can be reused across
    /// multiple line parsing operations.
    fn from_csv_line(
        line: &'a str, 
        line_number: usize, 
        delimiter: Delimiter,
        field_buffer: &'a mut Vec<&'a str>
    ) -> Result<Self, Self::Error>;
}

/// Standard implementation for CsvRecord parsing
impl<'a> FromCsvLine<'a> for CsvRecord<'a> {
    type Error = CsvParseError;
    
    fn from_csv_line(
        line: &'a str, 
        line_number: usize, 
        delimiter: Delimiter,
        field_buffer: &'a mut Vec<&'a str>
    ) -> Result<Self, Self::Error> {
        // Clear and reuse the buffer for zero allocation
        field_buffer.clear();
        
        // Parse fields using the delimiter
        let delim_char = delimiter.as_char();
        for field in line.split(delim_char) {
            field_buffer.push(field.trim());
        }
        
        // Validate we have at least one field
        if field_buffer.is_empty() {
            return Err(CsvParseError::EmptyLine { line_number });
        }
        
        // Use first field as ID, all fields as data
        let id = field_buffer[0];
        let data = field_buffer.as_slice();
        
        Ok(CsvRecord::new(id, data, line_number))
    }
}

/// Comprehensive error type for CSV parsing operations
/// 
/// Provides detailed error information for debugging and error handling
/// without allocation overhead in the error path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvParseError {
    /// Empty line encountered during parsing
    EmptyLine { line_number: usize },
    /// Invalid field count for expected schema
    InvalidFieldCount { line_number: usize, expected: usize, actual: usize },
    /// Field validation failed
    InvalidField { line_number: usize, field_index: usize, reason: &'static str },
    /// I/O error during file reading
    IoError { message: String },
    /// UTF-8 encoding error
    EncodingError { line_number: usize },
}

impl std::fmt::Display for CsvParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsvParseError::EmptyLine { line_number } => {
                write!(f, "Empty line at line {}", line_number)
            }
            CsvParseError::InvalidFieldCount { line_number, expected, actual } => {
                write!(f, "Invalid field count at line {}: expected {}, got {}", 
                       line_number, expected, actual)
            }
            CsvParseError::InvalidField { line_number, field_index, reason } => {
                write!(f, "Invalid field at line {}, field {}: {}", 
                       line_number, field_index, reason)
            }
            CsvParseError::IoError { message } => {
                write!(f, "I/O error: {}", message)
            }
            CsvParseError::EncodingError { line_number } => {
                write!(f, "UTF-8 encoding error at line {}", line_number)
            }
        }
    }
}

impl std::error::Error for CsvParseError {}

// Extension traits are already exported via glob imports above
