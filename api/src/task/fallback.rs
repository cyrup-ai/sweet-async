use crate::task::AsyncTaskError;
use crate::task::builder::AsyncWork;
use std::future::Future;
use std::pin::Pin;
use std::marker::PhantomData;

/// Default fallback that just returns the error
pub struct DefaultFallback<T> {
    _phantom: PhantomData<T>,
}

impl<T> Default for DefaultFallback<T> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: Clone + Send + 'static> AsyncWork<Result<T, AsyncTaskError>> for DefaultFallback<T> {
    fn run(self) -> Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>> {
        Box::pin(async move {
            Err(AsyncTaskError::Failure("No fallback available".to_string()))
        })
    }
}