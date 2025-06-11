//! Implementation of the IntoAsyncResult trait for Tokio
//!
//! This module provides Tokio-specific implementations of the IntoAsyncResult trait.
//! It's used to convert various result types to a standardized format.

use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;

// Re-export the API trait for convenience
pub use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;