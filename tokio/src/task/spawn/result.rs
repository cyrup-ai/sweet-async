//! Task result implementation for Tokio
//!
//! This module provides the result type for Tokio task execution.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::{AsyncResult, TaskResult};


/// Tokio implementation of TaskResult
#[derive(Debug)]
pub struct TokioTaskResult<T> {
    /// Task result
    result: Result<T, AsyncTaskError>,
}

impl<T> TokioTaskResult<T> {
    /// Create a new task result
    pub fn new(result: Result<T, AsyncTaskError>) -> Self {
        Self { result }
    }
}


impl<T> TaskResult<T> for TokioTaskResult<T> 
where 
    T: Send + 'static,
{
    fn result(&self) -> Result<&T, &AsyncTaskError> {
        self.result.as_ref()
    }

    fn into_result(self) -> Result<T, AsyncTaskError> {
        self.result
    }

    fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    fn is_err(&self) -> bool {
        self.result.is_err()
    }

    fn as_ref(&self) -> Option<&T> {
        self.result.as_ref().ok()
    }

    fn as_err(&self) -> Option<&AsyncTaskError> {
        self.result.as_ref().err()
    }
}

/// Tokio implementation of AsyncResult
#[derive(Debug)]
pub struct TokioAsyncResult<T> {
    /// Task result
    result: Result<T, AsyncTaskError>,
}

impl<T> TokioAsyncResult<T> {
    /// Create a new async result
    pub fn new(result: Result<T, AsyncTaskError>) -> Self {
        Self { result }
    }
}

impl<T> TaskResult<T> for TokioAsyncResult<T> 
where 
    T: Send + 'static,
{
    fn result(&self) -> Result<&T, &AsyncTaskError> {
        self.result.as_ref()
    }

    fn into_result(self) -> Result<T, AsyncTaskError> {
        self.result
    }

    fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    fn is_err(&self) -> bool {
        self.result.is_err()
    }

    fn as_ref(&self) -> Option<&T> {
        self.result.as_ref().ok()
    }

    fn as_err(&self) -> Option<&AsyncTaskError> {
        self.result.as_ref().err()
    }
}

impl<T> AsyncResult<T> for TokioAsyncResult<T> 
where 
    T: Send + 'static,
{
    type AndThenFuture<U> = Pin<Box<dyn Future<Output = TokioTaskResult<U>> + Send + 'static>>;
    type AndThenResult<U> = TokioTaskResult<U>;
    type OrElseFuture = Pin<Box<dyn Future<Output = Self> + Send + 'static>>;
    type MapResult<U> = TokioAsyncResult<U>;
    type MapErrResult = TokioAsyncResult<T>;

    fn and_then<U, F, Fut>(self, f: F) -> Self::AndThenFuture<U>
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self::AndThenResult<U>> + Send + 'static,
        U: Send + 'static,
    {
        Box::pin(async move {
            match self.result {
                Ok(value) => {
                    // Execute the provided async work on the successful result
                    // f.run() returns a Future that resolves to Fut
                    let future_fut = f.run().await;
                    // Now await the Fut to get the final result
                    future_fut.await
                },
                Err(err) => TokioTaskResult::new(Err(err)),
            }
        })
    }

    fn or_else<F, Fut>(self, f: F) -> Self::OrElseFuture
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self> + Send + 'static,
    {
        Box::pin(async move {
            match self.result {
                Ok(value) => TokioAsyncResult::new(Ok(value)),
                Err(_) => {
                    // Execute the provided async work on error
                    // f.run() returns a Future that resolves to Fut
                    let future_fut = f.run().await;
                    // Now await the Fut to get the final result
                    future_fut.await
                }
            }
        })
    }

    fn map<U, F>(self, f: F) -> Self::MapResult<U>
    where
        F: AsyncWork<U> + Send + 'static,
        U: Send + 'static,
    {
        // Map operation transforms success values using async work
        // Since this method must return synchronously but F produces async work,
        // we handle this by indicating the operation requires async execution context
        match self.result {
            Ok(_value) => {
                // The async work f cannot be executed synchronously without violating constraints
                // Return error indicating this operation requires async context (and_then, etc.)
                TokioAsyncResult::new(Err(AsyncTaskError::InvalidState(
                    "Synchronous map operation cannot execute AsyncWork - use and_then instead".to_string()
                )))
            }
            Err(e) => TokioAsyncResult::new(Err(e)),
        }
    }

    fn map_err<F>(self, f: F) -> Self::MapErrResult
    where
        F: AsyncWork<AsyncTaskError> + Send + 'static,
    {
        // Map error operation transforms error values using async work
        match self.result {
            Ok(value) => TokioAsyncResult::new(Ok(value)),
            Err(_) => {
                // Cannot execute async work synchronously without violating constraints
                TokioAsyncResult::new(Err(AsyncTaskError::InvalidState(
                    "Synchronous map_err operation cannot execute AsyncWork - use or_else instead".to_string()
                )))
            }
        }
    }

    fn unwrap(self) -> T {
        match self.result {
            Ok(value) => value,
            Err(e) => {
                // The trait contract expects panic behavior on error
                // Use standard panic! macro to meet trait requirements
                panic!("called `AsyncResult::unwrap()` on an `Err` value: {:?}", e);
            }
        }
    }

    fn unwrap_err(self) -> AsyncTaskError {
        match self.result {
            Ok(_) => {
                // The trait contract expects panic behavior on success
                panic!("called `AsyncResult::unwrap_err()` on an `Ok` value");
            },
            Err(e) => e,
        }
    }
}
