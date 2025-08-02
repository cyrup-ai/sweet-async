use std::future::Future;
use std::pin::Pin;

use sweet_async_api::task::{AsyncTaskError, builder::AsyncWork};

/// Fallback implementation for error recovery
#[derive(Clone, Debug, Default)]
pub struct ErrorFallback<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ErrorFallback<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + Send + 'static> AsyncWork<Result<T, AsyncTaskError>> for ErrorFallback<T> {
    fn run(self) -> impl Future<Output = Result<T, AsyncTaskError>> + Send + 'static {
        async move {
            Err(AsyncTaskError::Failure(
                "Task failed with no recovery".to_string(),
            ))
        }
    }
}

// Explicitly implement Send and Sync for ErrorFallback
unsafe impl<T> Send for ErrorFallback<T> {}
unsafe impl<T> Sync for ErrorFallback<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_fallback() {
        let fallback = ErrorFallback::<i32>::default();
        let result = fallback.run().await;
        assert!(matches!(result, Err(AsyncTaskError::Failure(_))));
    }
}
