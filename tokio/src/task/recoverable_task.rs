//! High-performance recoverable task implementation with zero allocation

use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::{AsyncTaskError, RecoverableTask, RetryStrategy};

/// Zero-allocation fallback work implementation
#[derive(Debug)]
pub struct TokioFallbackWork<T> {
    value: Option<T>,
}

impl<T> TokioFallbackWork<T>
where
    T: Clone + Send + Sync + 'static,
{
    #[inline]
    pub fn new(value: Option<T>) -> Self {
        Self { value }
    }
}

impl<T> AsyncWork<Result<T, AsyncTaskError>> for TokioFallbackWork<T>
where
    T: Clone + Send + Sync + 'static,
{
    #[inline]
    fn run(self) -> impl Future<Output = Result<T, AsyncTaskError>> + Send + 'static {
        async move {
            match self.value {
                Some(v) => Ok(v),
                None => Err(AsyncTaskError::RecoveryFailed(
                    "No fallback value".to_string(),
                )),
            }
        }
    }
}

/// Zero-allocation recoverable task implementation
#[derive(Debug)]
pub struct TokioRecoverableTask<T> {
    fallback_work: TokioFallbackWork<T>,
    max_retries: u8,
    current_retry: AtomicU8,
    retry_strategy: RetryStrategy,
}

impl<T> TokioRecoverableTask<T>
where
    T: Clone + Send + Sync + 'static,
{
    #[inline]
    pub fn new(value: Option<T>) -> Self {
        Self {
            fallback_work: TokioFallbackWork::new(value),
            max_retries: 3,
            current_retry: AtomicU8::new(0),
            retry_strategy: RetryStrategy::Fixed(Duration::from_millis(100)),
        }
    }

    #[inline]
    pub fn with_retries(value: Option<T>, max_retries: u8) -> Self {
        Self {
            fallback_work: TokioFallbackWork::new(value),
            max_retries,
            current_retry: AtomicU8::new(0),
            retry_strategy: RetryStrategy::Fixed(Duration::from_millis(100)),
        }
    }
}

impl<T> Default for TokioRecoverableTask<T>
where
    T: Clone + Send + Sync + 'static,
{
    #[inline]
    fn default() -> Self {
        Self::new(None)
    }
}

impl<T> RecoverableTask<T> for TokioRecoverableTask<T>
where
    T: Clone + Send + Sync + 'static,
{
    type FallbackWork = TokioFallbackWork<T>;

    #[inline]
    fn recover(
        &self,
        _error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        let fallback = TokioFallbackWork::new(None);
        AsyncWork::run(fallback)
    }

    #[inline]
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        !self.retries_exhausted()
    }

    #[inline]
    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.fallback_work
    }

    #[inline]
    fn max_retries(&self) -> u8 {
        self.max_retries
    }

    #[inline]
    fn current_retry(&self) -> u8 {
        self.current_retry.load(Ordering::Relaxed)
    }

    #[inline]
    fn retry_strategy(&self) -> RetryStrategy {
        self.retry_strategy
    }
}
