//! Task result implementation for Tokio
//!
//! This module provides the result type for Tokio task execution.

use std::future::Future;
use std::pin::Pin;

use std::task::{Context, Poll};

use sweet_async_api::task::AsyncTaskError;

use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::{AsyncResult, TaskResult};

/// Wrapper for AndThenFuture that satisfies API bounds conditionally
pub struct TokioAndThenFuture<U> {
    inner: Pin<Box<dyn Future<Output = TokioGenericTaskResult<U>> + Send + 'static>>,
}

impl<U> TokioAndThenFuture<U> {
    pub fn new<F>(inner: F) -> Self 
    where 
        F: Future<Output = TokioGenericTaskResult<U>> + Send + 'static,
    {
        Self { inner: Box::pin(inner) }
    }
}

impl<U> Future for TokioAndThenFuture<U> {
    type Output = TokioGenericTaskResult<U>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

/// Generic task result wrapper that implements TaskResult<U> for any U
#[derive(Debug)]
pub struct TokioGenericTaskResult<U> {
    result: Result<U, AsyncTaskError>,
}

impl<U> TokioGenericTaskResult<U> {
    pub fn new(result: Result<U, AsyncTaskError>) -> Self {
        Self { result }
    }
}

impl<U> TaskResult<U> for TokioGenericTaskResult<U> 
where 
    U: Send + 'static,
{
    fn result(&self) -> Result<&U, &AsyncTaskError> {
        self.result.as_ref()
    }

    fn into_result(self) -> Result<U, AsyncTaskError> {
        self.result
    }

    fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    fn is_err(&self) -> bool {
        self.result.is_err()
    }

    fn as_ref(&self) -> Option<&U> {
        self.result.as_ref().ok()
    }

    fn as_err(&self) -> Option<&AsyncTaskError> {
        self.result.as_ref().err()
    }
}

/// Generic async result wrapper that implements AsyncResult<U> for any U  
#[derive(Debug)]
pub struct TokioGenericAsyncResult<U> {
    result: Result<U, AsyncTaskError>,
}

impl<U> TokioGenericAsyncResult<U> {
    pub fn new(result: Result<U, AsyncTaskError>) -> Self {
        Self { result }
    }
}

impl<U> TaskResult<U> for TokioGenericAsyncResult<U>
where
    U: Send + 'static,
{
    fn result(&self) -> Result<&U, &AsyncTaskError> {
        self.result.as_ref()
    }

    fn into_result(self) -> Result<U, AsyncTaskError> {
        self.result
    }

    fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    fn is_err(&self) -> bool {
        self.result.is_err()
    }

    fn as_ref(&self) -> Option<&U> {
        self.result.as_ref().ok()
    }

    fn as_err(&self) -> Option<&AsyncTaskError> {
        self.result.as_ref().err()
    }
}

impl<U: Send + 'static> AsyncResult<U> for TokioGenericAsyncResult<U>
{
    type AndThenFuture<V> = TokioAndThenFuture<V>;
    type AndThenResult<V> = TokioGenericTaskResult<V>;
    type OrElseFuture = Pin<Box<dyn Future<Output = Self> + Send + 'static>>;
    type MapResult<V> = TokioGenericAsyncResult<V>;
    type MapErrResult = TokioGenericAsyncResult<U>;

    fn and_then<V, F, Fut>(self, f: F) -> Self::AndThenFuture<V>
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self::AndThenResult<V>> + Send + 'static,
        V: Send + 'static,
    {
        TokioAndThenFuture::new(Box::pin(async move {
            match self.result {
                Ok(value) => {
                    let future_fut = f.run().await;
                    future_fut.await
                },
                Err(err) => TokioGenericTaskResult::new(Err(err)),
            }
        }))
    }

    fn or_else<F, Fut>(self, f: F) -> Self::OrElseFuture
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self> + Send + 'static,
    {
        Box::pin(async move {
            match self.result {
                Ok(value) => TokioGenericAsyncResult::new(Ok(value)),
                Err(_) => {
                    let future_fut = f.run().await;
                    future_fut.await
                }
            }
        })
    }

    fn map<V, F>(self, f: F) -> Self::MapResult<V>
    where
        F: AsyncWork<V> + Send + 'static,
        V: Send + 'static,
    {
        match self.result {
            Ok(_value) => {
                TokioGenericAsyncResult::new(Err(AsyncTaskError::InvalidState(
                    "Synchronous map operation cannot execute AsyncWork - use and_then instead".to_string()
                )))
            }
            Err(e) => TokioGenericAsyncResult::new(Err(e)),
        }
    }

    fn map_err<F>(self, f: F) -> Self::MapErrResult
    where
        F: AsyncWork<AsyncTaskError> + Send + 'static,
    {
        match self.result {
            Ok(value) => TokioGenericAsyncResult::new(Ok(value)),
            Err(_) => {
                TokioGenericAsyncResult::new(Err(AsyncTaskError::InvalidState(
                    "Synchronous map_err operation cannot execute AsyncWork - use or_else instead".to_string()
                )))
            }
        }
    }

    fn unwrap(self) -> U {
        match self.result {
            Ok(value) => value,
            Err(e) => {
                panic!("called `AsyncResult::unwrap()` on an `Err` value: {:?}", e);
            }
        }
    }

    fn unwrap_err(self) -> AsyncTaskError {
        match self.result {
            Ok(_) => {
                panic!("called `AsyncResult::unwrap_err()` on an `Ok` value");
            },
            Err(e) => e,
        }
    }
}


/// Tokio implementation of TaskResult
#[derive(Debug)]
pub struct TokioTaskResult<T> {
    /// Task result
    result: Result<T, AsyncTaskError>,
}

impl<T> TokioTaskResult<T> 
where
    T: Send + 'static,
{
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

impl<T> TokioAsyncResult<T> 
where
    T: Send + 'static,
{
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
    type AndThenFuture<U> = Pin<Box<dyn Future<Output = Self::AndThenResult<U>> + Send + 'static>>;
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
