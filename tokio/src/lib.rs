//! # Sweet Async - Tokio Implementation
//!
//! A fluent, immutable builder for orchestrating asynchronous work with powerful abstractions
//! for complex async workflows. This crate provides the Tokio-based implementation of Sweet Async's
//! fluent API for task orchestration, event streaming, and structured concurrency.
//!
//! ## Core Features
//!
//! - **Fluent Task Building**: Immutable builder pattern for async task configuration
//! - **Event Streaming**: Powerful emit/collect patterns for processing large datasets
//! - **Adaptive Concurrency**: Intelligent workload-based concurrency management
//! - **Structured Concurrency**: Safe, composable async workflow orchestration
//! - **Zero-allocation Design**: Performance-optimized for high-throughput scenarios
//!
//! ## Quick Start Examples
//!
//! ### Basic Task Execution
//!
//! ```rust
//! use sweet_async_tokio::AsyncTask;
//! use sweet_async_tokio::task::DurationExt;
//!
//! # async fn example() -> Result<String, Box<dyn std::error::Error>> {
//! // Execute async work with timeout and error handling
//! let result = AsyncTask::to::<String>()
//!     .timeout(30.seconds())
//!     .run(|| async {
//!         // Your async work here
//!         tokio::time::sleep(std::time::Duration::from_millis(100)).await;
//!         Ok("Processing complete".to_string())
//!     })
//!     .await?;
//! # Ok(result)
//! # }
//! ```
//!
//! ### Block Reduction Syntax (Sync-looking, Async Execution)
//!
//! ```rust
//! use sweet_async_tokio::AsyncTask;
//! use sweet_async_tokio::task::DurationExt;
//!
//! # async fn example() -> Result<String, Box<dyn std::error::Error>> {
//! // Write sync-looking code that runs asynchronously
//! let result = AsyncTask::to::<String>({
//!     // This block runs asynchronously despite sync appearance
//!     "async work result".to_string()
//! }).await_result(|result| {
//!     match result {
//!         Ok(data) => Ok(data),
//!         Err(e) => Err(format!("Processing failed: {}", e).into())
//!     }
//! }).await?;
//! # Ok(result)
//! # }
//! ```
//!
//! ### Event Streaming for Data Processing
//!
//! ```rust
//! use sweet_async_tokio::{AsyncTask, task::{DurationExt, RowsExt, Delimiter}};
//!
//! # async fn example() -> Result<Vec<String>, Box<dyn std::error::Error>> {
//! // Process CSV data with streaming collectors
//! let csv_records = AsyncTask::emits::<String>()
//!     .with_timeout(60.seconds())
//!     .sender(|collector| {
//!         // Configure data source and chunking
//!         collector.of_file("data.csv")
//!             .with_delimiter(Delimiter::NewLine)
//!             .into_chunks(100.rows());
//!     })
//!     .receiver(|event, collector| {
//!         // Process each event and collect results
//!         let record = event.data().clone();
//!         if !record.is_empty() {
//!             collector.collect(event.id(), record);
//!         }
//!     })
//!     .await_final_event(|_event, collector| {
//!         // Return final collected results
//!         Ok(collector.collected())
//!     })
//!     .await?;
//! # Ok(csv_records)
//! # }
//! ```
//!
//! ### Large-Scale Data Processing with Smart Collectors
//!
//! ```rust
//! use sweet_async_tokio::{AsyncTask, task::{RowsExt, ChunkSize}};
//!
//! # async fn example() -> Result<Vec<String>, Box<dyn std::error::Error>> {
//! let processed_data = AsyncTask::emits::<String>()
//!     .sender(|collector| {
//!         // Auto-chunk large collections to prevent memory issues
//!         let large_dataset: Vec<String> = (0..1000000).map(|i| format!("item_{}", i)).collect();
//!         collector.of(large_dataset)
//!             .into_chunks(500.rows());
//!
//!         // Stream from multiple sources concurrently
//!         collector.from_database("connection", "SELECT * FROM large_table")
//!             .into_chunks(1000.rows());
//!
//!         // Process files with custom chunking
//!         collector.of_file("huge_file.jsonl")
//!             .with_delimiter(Delimiter::NewLine)
//!             .into_chunks(ChunkSize::Lines(250));
//!     })
//!     .receiver(|event, collector| {
//!         // Efficient per-item processing
//!         let item = event.data().clone();
//!         let processed = format!("processed_{}", item);
//!         collector.collect(event.id(), processed);
//!     })
//!     .await_final_event(|_event, collector| {
//!         Ok(collector.collected())
//!     })
//!     .await?;
//! # Ok(processed_data)
//! # }
//! ```
//!
//! ### Adaptive Concurrency Processing
//!
//! ```rust
//! use sweet_async_tokio::task::adaptive::{AdaptiveConfig, adaptive_stream};
//! use futures::StreamExt;
//!
//! # async fn example() -> Vec<usize> {
//! // Automatically adapt concurrency based on workload characteristics
//! let config = AdaptiveConfig::default();
//! let data = 0..10000;
//!
//! let results = adaptive_stream(data, |item| {
//!     // CPU-bound work automatically uses Rayon
//!     // IO-bound work automatically uses Tokio
//!     item * 2
//! }, config)
//! .collect::<Vec<_>>()
//! .await;
//! # results
//! # }
//! ```
//!
//! ## Architecture
//!
//! This implementation provides the Tokio runtime integration for Sweet Async's fluent API,
//! enabling high-performance asynchronous task orchestration with:
//!
//! - **Immutable Builders**: Thread-safe, composable task configuration
//! - **Event Pipeline**: Efficient streaming data processing with backpressure handling
//! - **Adaptive Engine**: Intelligent CPU vs IO-bound workload detection and optimization  
//! - **Resource Management**: Automatic cleanup and cancellation support
//! - **Type Safety**: Strong typing with generic task and error handling

pub mod orchestra;
pub mod task;
pub mod task_id_uuid;

#[cfg(test)]
mod validation_test;


// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CORE API EXPORTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// Orchestra Components - Task orchestration and runtime management
pub use orchestra::TokioOrchestratorBuilder;
pub use orchestra::runtime::TokioRuntime;

// Core Task Types - Main task implementations for fluent API
pub use task::{TokioAsyncTask, TokioEmittingTask, TokioSenderTask, TokioReceiverTask, TokioFinalEvent};
pub use task::builder::TokioAsyncTaskBuilder;

// Task ID System - UUID-based task identification
pub use task_id_uuid::*;

// Extension Traits - Ergonomic syntax sugar for fluent API
pub use task::{DurationExt, RowsExt, ChunkSize, Delimiter};

// Adaptive Concurrency - High-performance workload-adaptive processing  
pub use task::adaptive::{AdaptiveConfig, adaptive_stream, process_adaptive};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FLUENT API ALIAS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Type alias to enable README.md fluent syntax: `AsyncTask::to::<T>()` and `AsyncTask::emits::<T>()`
///
/// This alias provides the primary entry point for Sweet Async's fluent API, allowing users
/// to write intuitive async task orchestration code as shown in the documentation examples.
pub type AsyncTask<T, I = UuidTaskId> = TokioAsyncTask<T, I>;
