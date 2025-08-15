use crate::task::AsyncTaskError;
use crate::task::builder::AsyncWork;
use std::future::Future;
use std::time::Duration;

/// Strategy for retry backoff
#[derive(Clone, Copy, Debug)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed(Duration),
    /// Exponential backoff with base delay
    Exponential {
        base: Duration,
        factor: f64,
        max: Duration,
    },
    /// Linear backoff with increment
    Linear { base: Duration, increment: Duration },
    /// No delay between retries
    Immediate,
}

pub trait RecoverableTask<T: Clone + Send + 'static> {
    /// The type of fallback work that returns Result<T, AsyncTaskError>
    type FallbackWork: AsyncWork<Result<T, AsyncTaskError>> + Send + Sync;

    /// Attempt to recover from an error by executing fallback work
    fn recover(
        &self,
        error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send;

    /// Check if recovery is possible for this error
    fn can_recover_from(&self, error: &AsyncTaskError) -> bool;

    /// Get the fallback work that will be executed on recovery
    fn fallback_work(&self) -> &Self::FallbackWork;

    /// Get the maximum retry count
    fn max_retries(&self) -> u8;

    /// Get the current retry attempt number
    fn current_retry(&self) -> u8;

    /// Get the retry strategy
    fn retry_strategy(&self) -> RetryStrategy;

    /// Check if retries are exhausted
    fn retries_exhausted(&self) -> bool {
        self.current_retry() >= self.max_retries()
    }
}
