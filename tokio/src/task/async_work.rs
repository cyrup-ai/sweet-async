//! Async work execution for Tokio runtime
//!
//! This module provides the AsyncWork trait implementation and utilities
//! for executing async work within the Tokio runtime context.

use std::future::Future;

// Re-export AsyncWork from the API crate
pub use sweet_async_api::task::builder::AsyncWork;

/// Tokio-specific async work wrapper
pub struct TokioAsyncWork<F, R> {
    work: F,
    _phantom: std::marker::PhantomData<R>,
}

impl<F, R> TokioAsyncWork<F, R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    /// Create a new async work wrapper
    pub fn new(work: F) -> Self {
        Self {
            work,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<F, R> AsyncWork<R> for TokioAsyncWork<F, R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    async fn run(self) -> R {
        (self.work)()
    }
}

/// Async work wrapper for async closures
pub struct TokioAsyncFutureWork<F, Fut, R> {
    work: F,
    _phantom: std::marker::PhantomData<(Fut, R)>,
}

impl<F, Fut, R> TokioAsyncFutureWork<F, Fut, R>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    /// Create a new async future work wrapper
    pub fn new(work: F) -> Self {
        Self {
            work,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<F, Fut, R> AsyncWork<R> for TokioAsyncFutureWork<F, Fut, R>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    async fn run(self) -> R {
        (self.work)().await
    }
}

/// Utility function to create async work from a sync closure
pub fn sync_work<F, R>(work: F) -> TokioAsyncWork<F, R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    TokioAsyncWork::new(work)
}

/// Utility function to create async work from an async closure
pub fn async_work<F, Fut, R>(work: F) -> TokioAsyncFutureWork<F, Fut, R>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    TokioAsyncFutureWork::new(work)
}