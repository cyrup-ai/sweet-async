use std::future::Future;
use std::pin::Pin;

/// Trait to normalize any value, Result, or Future into a Future<Output = Result<T, E>>
pub trait IntoAsyncResult<T, E>: Send + 'static {
    fn into_async_result(self) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;
}

// For Future<Output = Result<T, E>>
impl<T, E, F> IntoAsyncResult<T, E> for F
where
    F: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    fn into_async_result(self) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send>> {
        Box::pin(self)
    }
}
