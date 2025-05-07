use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::result::Result;
use std::task::{Context, Poll};

use crate::api::task::AsyncTaskError;
use crate::api::task::spawn::SpawningTask;

/// A specialized AsyncTask that contains a result value
///
/// TaskResult implements both AsyncTask (for task management) and
/// standard Result functionality. This allows continued task management
/// even after the computation has completed while also providing access
/// to the computation result.
pub trait TaskResult<Id: Debug + Send + Sync + 'static, T: Send + 'static>: 
    AsyncTask<Id, T> + Send + 'static
{
    /// Get the result of this task computation
    fn result(&self) -> Result<&T, &AsyncTaskError>;
    
    /// Consume the task and return just the result value
    fn into_result(self) -> Result<T, AsyncTaskError> where Self: Sized;
    
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
pub trait AsyncResult<Id: Debug + Send + Sync + 'static, T: Send + 'static>: 
    TaskResult<Id, T> + Send + 'static
{
    /// Chain with another operation that returns a TaskResult
    fn and_then<U, F, Fut>(self, f: F) -> impl AsyncResult<Id, U> + Send + 'static
    where
        F: FnOnce(T) -> Fut + Send + 'static,
        Fut: Future<Output = impl TaskResult<Id, U>> + Send + 'static,
        U: Send + 'static;
    
    /// Chain with a function that handles errors
    fn or_else<F, Fut>(self, f: F) -> impl AsyncResult<Id, T> + Send + 'static
    where
        F: FnOnce(AsyncTaskError) -> Fut + Send + 'static,
        Fut: Future<Output = impl TaskResult<Id, T>> + Send + 'static;
    
    /// Map the success value to another type
    fn map<U, F>(self, f: F) -> impl AsyncResult<Id, U> + Send + 'static
    where
        F: FnOnce(T) -> U + Send + 'static,
        U: Send + 'static;
    
    /// Map the error value to another error
    fn map_err<F>(self, f: F) -> impl AsyncResult<Id, T> + Send + 'static
    where
        F: FnOnce(AsyncTaskError) -> AsyncTaskError + Send + 'static;
    
    /// Unwrap the result, returning the success value or panicking
    fn unwrap(self) -> T where Self: Sized;
    
    /// Unwrap the error, returning the error value or panicking
    fn unwrap_err(self) -> AsyncTaskError where Self: Sized;
}

