//! Task result implementation for Tokio
//!
//! This module provides the result type for Tokio task execution.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::{AsyncResult, TaskResult};

/// Tokio implementation of TaskResult
pub struct TokioTaskResult<T: Send + 'static + std::fmt::Debug> {
    /// Task result
    result: Result<T, AsyncTaskError>,
}

impl<T: Send + 'static + std::fmt::Debug> TokioTaskResult<T> {
    /// Create a new task result
    pub fn new(result: Result<T, AsyncTaskError>) -> Self {
        Self { result }
    }
}

impl<T: Send + 'static + std::fmt::Debug> TaskResult<T> for TokioTaskResult<T> {
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
pub struct TokioAsyncResult<T: Send + 'static + std::fmt::Debug> {
    /// Task result
    result: Result<T, AsyncTaskError>,
}

impl<T: Send + 'static + std::fmt::Debug> TokioAsyncResult<T> {
    /// Create a new async result
    pub fn new(result: Result<T, AsyncTaskError>) -> Self {
        Self { result }
    }
}

impl<T: Send + 'static + std::fmt::Debug> TaskResult<T> for TokioAsyncResult<T> {
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

impl<T: Send + 'static + std::fmt::Debug> AsyncResult<T> for TokioAsyncResult<T> {
    type AndThenFuture<U> = Pin<Box<dyn Future<Output = Self::AndThenResult<U>> + Send + 'static>>;
    type AndThenResult<U> = TokioTaskResult<U>;
    type OrElseFuture = Pin<Box<dyn Future<Output = Self> + Send + 'static>>;
    type MapResult<U> = TokioAsyncResult<U>;
    type MapErrResult = TokioAsyncResult<T>;

    fn and_then<U, F, Fut>(self, f: F) -> Self::AndThenFuture<U>
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self::AndThenResult<U>> + Send + 'static,
        U: Send + 'static + std::fmt::Debug,
    {
        Box::pin(async move {
            match self.result {
                Ok(_) => {
                    // TODO: implement async work execution
                    TokioTaskResult::new(Err(AsyncTaskError::Failure(
                        "and_then not yet implemented".to_string(),
                    )))
                }
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
                    // TODO: implement async work execution
                    TokioAsyncResult::new(Err(AsyncTaskError::Failure(
                        "or_else not yet implemented".to_string(),
                    )))
                }
            }
        })
    }

    fn map<U, F>(self, f: F) -> Self::MapResult<U>
    where
        F: AsyncWork<U> + Send + 'static,
        U: Send + 'static + std::fmt::Debug,
    {
        // TODO: implement async work execution
        TokioAsyncResult::new(Err(AsyncTaskError::Failure(
            "map not yet implemented".to_string(),
        )))
    }

    fn map_err<F>(self, f: F) -> Self::MapErrResult
    where
        F: AsyncWork<AsyncTaskError> + Send + 'static,
    {
        // TODO: implement async work execution
        TokioAsyncResult::new(self.result)
    }

    fn unwrap(self) -> T {
        self.result.unwrap()
    }

    fn unwrap_err(self) -> AsyncTaskError {
        self.result.unwrap_err()
    }
}
