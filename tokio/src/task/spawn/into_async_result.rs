//! Tokio implementation of IntoAsyncResult trait

use std::future::Future;
use std::pin::Pin;
use sweet_async_api::AsyncTaskError;
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;

/// Newtype wrapper to avoid orphan rule violations
#[derive(Debug, Clone)]
pub struct TokioIntoAsyncResult<T>(pub T);

impl<T> IntoAsyncResult<T, AsyncTaskError> for TokioIntoAsyncResult<T>
where
    T: Send + 'static,
{
    fn into_async_result(self) -> Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>> {
        Box::pin(async move { Ok(self.0) })
    }
}
