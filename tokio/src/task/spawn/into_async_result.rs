//! Implementation of the IntoAsyncResult trait for Tokio
//!
//! This module provides Tokio-specific implementations of the IntoAsyncResult trait.
//! It's used to convert various result types to a standardized format.

// Re-export the API trait
pub use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;