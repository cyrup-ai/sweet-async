use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use sweet_async_api::task::AsyncWork;
use sweet_async_api::task::spawn::{AsyncResult, TaskResult};
use sweet_async_api::task::task_error::AsyncTaskError;
// Cannot use tokio directly - must use runtime abstraction
// use tokio::task::JoinHandle;

/// A task result implementation for Tokio-based async tasks
#[derive(Debug)]
pub struct TokioTaskResult<T> {
    pub task_result: Result<T, AsyncTaskError>,
}

impl<T> TokioTaskResult<T> {
    #[inline]
    pub fn new(result: Result<T, AsyncTaskError>) -> Self {
        Self {
            task_result: result,
        }
    }

    #[inline]
    pub fn ok(value: T) -> Self {
        Self::new(Ok(value))
    }

    #[inline]
    pub fn err(error: AsyncTaskError) -> Self {
        Self::new(Err(error))
    }
}

impl<T: Send + 'static> TaskResult<T> for TokioTaskResult<T> {
    #[inline]
    fn result(&self) -> Result<&T, &AsyncTaskError> {
        self.task_result.as_ref()
    }

    #[inline]
    fn into_result(self) -> Result<T, AsyncTaskError> {
        self.task_result
    }

    #[inline]
    fn is_ok(&self) -> bool {
        self.task_result.is_ok()
    }

    #[inline]
    fn is_err(&self) -> bool {
        self.task_result.is_err()
    }

    #[inline]
    fn as_ref(&self) -> Option<&T> {
        self.task_result.as_ref().ok()
    }

    #[inline]
    fn as_err(&self) -> Option<&AsyncTaskError> {
        self.task_result.as_ref().err()
    }
}

/// An async result implementation for Tokio-based async tasks
pub struct TokioAsyncResult<T> {
    handle: JoinHandle<Result<T, AsyncTaskError>>,
}

impl<T> TokioAsyncResult<T> {
    #[inline]
    pub fn new(handle: JoinHandle<Result<T, AsyncTaskError>>) -> Self {
        Self { handle }
    }

    #[inline]
    pub fn from_result(result: Result<T, AsyncTaskError>) -> Self
    where
        T: Send + 'static,
    {
        // Cannot spawn directly - this method should not be used
        // Use runtime handle from context instead
        panic!(
            "from_result requires runtime context - use TokioAsyncResult::new with proper handle"
        )
    }
}

impl<T: Send + 'static> TaskResult<T> for TokioAsyncResult<T> {
    fn result(&self) -> Result<&T, &AsyncTaskError> {
        panic!("Cannot synchronously access async result - use await instead")
    }

    fn into_result(self) -> Result<T, AsyncTaskError> {
        panic!("Cannot synchronously access async result - use await instead")
    }

    #[inline]
    fn is_ok(&self) -> bool {
        false
    }

    #[inline]
    fn is_err(&self) -> bool {
        false
    }

    #[inline]
    fn as_ref(&self) -> Option<&T> {
        None
    }

    #[inline]
    fn as_err(&self) -> Option<&AsyncTaskError> {
        None
    }
}

impl<T: Send + 'static> Future for TokioAsyncResult<T> {
    type Output = Result<T, AsyncTaskError>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.handle).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(join_error)) => Poll::Ready(Err(AsyncTaskError::Failure(format!(
                "Task join error: {}",
                join_error
            )))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future wrapper that resolves to TokioTaskResult
pub struct TokioAndThenFuture<U> {
    inner: Pin<Box<dyn Future<Output = Result<U, AsyncTaskError>> + Send + 'static>>,
}

impl<U> TokioAndThenFuture<U> {
    #[inline]
    pub fn new(
        future: Pin<Box<dyn Future<Output = Result<U, AsyncTaskError>> + Send + 'static>>,
    ) -> Self {
        Self { inner: future }
    }
}

impl<U> Future for TokioAndThenFuture<U> {
    type Output = TokioTaskResult<U>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(result) => Poll::Ready(TokioTaskResult::new(result)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future wrapper that resolves to TokioAsyncResult
pub struct TokioOrElseFuture<T> {
    inner: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>>,
}

impl<T> TokioOrElseFuture<T> {
    #[inline]
    pub fn new(
        future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send + 'static>>,
    ) -> Self {
        Self { inner: future }
    }
}

impl<T: Send + 'static> Future for TokioOrElseFuture<T> {
    type Output = TokioAsyncResult<T>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(result) => Poll::Ready(TokioAsyncResult::from_result(result)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: Send + 'static> AsyncResult<T> for TokioAsyncResult<T> {
    type AndThenFuture<U> = TokioAndThenFuture<U> where U: Send + 'static;
    type AndThenResult<U> = TokioTaskResult<U> where U: Send + 'static;
    type OrElseFuture = TokioOrElseFuture<T>;
    type MapResult<U> = TokioAsyncResult<U>;
    type MapErrResult = TokioAsyncResult<T>;

    #[inline]
    fn and_then<U, F, Fut>(self, f: F) -> Self::AndThenFuture<U>
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self::AndThenResult<U>> + Send + 'static,
    {
        let future = Box::pin(async move {
            match self.await {
                Ok(_value) => {
                    let result = f.run().await;
                    result.await
                }
                Err(error) => TokioTaskResult::err(error),
            }
        });
        TokioAndThenFuture::new(future)
    }

    #[inline]
    fn map<U, F>(self, f: F) -> Self::MapResult<U>
    where
        F: AsyncWork<U> + Send + 'static,
        U: Send + 'static,
    {
        let future = Box::pin(async move {
            match self.await {
                Ok(_value) => {
                    let result = f.run().await;
                    Ok(result)
                }
                Err(error) => TokioTaskResult::err(error),
            }
        });
        // Cannot spawn directly - need runtime handle
        panic!("AsyncResult::map requires runtime context - use proper runtime handle")
    }

    #[inline]
    fn or_else<F, Fut>(self, f: F) -> Self::OrElseFuture
    where
        F: AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = Self> + Send + 'static,
    {
        let future = Box::pin(async move {
            match self.await {
                Ok(value) => Ok(value),
                Err(_error) => {
                    let result = f.run().await;
                    result.await
                }
            }
        });
        TokioOrElseFuture::new(future)
    }

    #[inline]
    fn map_err<F>(self, f: F) -> Self::MapErrResult
    where
        F: AsyncWork<AsyncTaskError> + Send + 'static,
    {
        let future = Box::pin(async move {
            match self.await {
                Ok(value) => Ok(value),
                Err(_error) => Err(f.run().await),
            }
        });
        // Cannot spawn directly - need runtime handle
        panic!("AsyncResult::map_err requires runtime context - use proper runtime handle")
    }

    fn unwrap(self) -> T {
        panic!("Cannot synchronously unwrap async result - use await instead")
    }

    fn unwrap_err(self) -> AsyncTaskError {
        panic!("Cannot synchronously unwrap_err async result - use await instead")
    }
}

impl<T: Send + 'static> From<Result<T, AsyncTaskError>> for TokioTaskResult<T> {
    #[inline]
    fn from(result: Result<T, AsyncTaskError>) -> Self {
        Self::new(result)
    }
}
