use crate::task::builder::AsyncWork;
use crate::task::task_error::AsyncTaskError;
use std::future::Future;

/// A specialized AsyncTask that contains a result value
///
/// TaskResult implements both AsyncTask (for task management) and
/// standard Result functionality. This allows continued task management
/// even after the computation has completed while also providing access
/// to the computation result.
#[allow(dead_code)]
pub trait TaskResult<T>: Send + 'static {
    /// Get the result of this task computation
    fn result(&self) -> Result<&T, &AsyncTaskError>;

    /// Consume the task and return just the result value
    fn into_result(self) -> Result<T, AsyncTaskError>
    where
        Self: Sized;

    /// Check if this task's computation succeeded
    fn is_ok(&self) -> bool;

    /// Check if this task's computation failed with an error
    fn is_err(&self) -> bool;

    /// Get a reference to the success value, if available
    fn as_ref(&self) -> Option<&T>;

    /// Get a reference to the error value, if available
    fn as_err(&self) -> Option<&AsyncTaskError>;
}

/// A specialized AsyncTask returned when awaiting a task
///
/// AsyncResult is a specialized AsyncTask that represents the result
/// of awaiting a SpawningTask. It combines task management capabilities
/// with result handling.
#[allow(dead_code)]
pub trait AsyncResult<T>: TaskResult<T> + Send + 'static {
    type AndThenFuture<U>: Future<Output = Self::AndThenResult<U>> + Send + 'static;
    type AndThenResult<U>: TaskResult<U>;
    type OrElseFuture: Future<Output = Self> + Send + 'static;
    type MapResult<U>: AsyncResult<U>;
    type MapErrResult: AsyncResult<T>;

    /// Chain with another operation that returns a TaskResult
    fn and_then<U, F, Fut>(self, f: F) -> Self::AndThenFuture<U>
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self::AndThenResult<U>> + Send + 'static,
        U: Send + 'static;

    /// Chain with a function that handles errors
    fn or_else<F, Fut>(self, f: F) -> Self::OrElseFuture
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self> + Send + 'static;

    /// Map the success value to another type
    fn map<U, F>(self, f: F) -> Self::MapResult<U>
    where
        F: AsyncWork<U> + Send + 'static,
        U: Send + 'static;

    /// Map the error value to another error
    fn map_err<F>(self, f: F) -> Self::MapErrResult
    where
        F: AsyncWork<AsyncTaskError> + Send + 'static;

    /// Unwrap the result, returning the success value or panicking
    fn unwrap(self) -> T
    where
        Self: Sized;

    /// Unwrap the error, returning the error value or panicking
    fn unwrap_err(self) -> AsyncTaskError
    where
        Self: Sized;
}
