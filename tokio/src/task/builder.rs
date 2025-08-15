//! High-performance task builder that stores configuration for run()

use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;
use sweet_async_api::task::builder::{AsyncTaskBuilder, AsyncWork};

/// Zero-allocation task builder that stores closures and config for run()
#[derive(Debug)]
pub struct TokioAsyncTaskBuilder<T, I> {
    timeout: Option<Duration>,
    retry_attempts: u8,
    tracing_enabled: bool,
    _phantom: PhantomData<(T, I)>,
}

impl<T, I> TokioAsyncTaskBuilder<T, I> {
    #[inline]
    pub fn new() -> Self {
        Self {
            timeout: None,
            retry_attempts: 0,
            tracing_enabled: false,
            _phantom: PhantomData,
        }
    }
}

impl<T, I> Default for TokioAsyncTaskBuilder<T, I> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, I> AsyncTaskBuilder for TokioAsyncTaskBuilder<T, I> {
    #[inline]
    fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    #[inline]
    fn retry(mut self, attempts: u8) -> Self {
        self.retry_attempts = attempts;
        self
    }

    #[inline]
    fn tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }

    #[inline]
    fn new() -> Self {
        Self::new()
    }
}

/// High-performance async work wrapper that stores closure for run()
#[derive(Debug)]
pub struct TokioAsyncWork<R, F> {
    work: F,
    _phantom: PhantomData<R>,
}

impl<R, F> TokioAsyncWork<R, F>
where
    F: FnOnce() -> R + Send + 'static,
{
    #[inline]
    pub fn new(work: F) -> Self {
        Self {
            work,
            _phantom: PhantomData,
        }
    }
}

impl<R, F> AsyncWork<R> for TokioAsyncWork<R, F>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    #[inline]
    fn run(self) -> impl Future<Output = R> + Send + 'static {
        async move { (self.work)() }
    }
}

/// Newtype wrapper for closure-based async work to avoid orphan rule
#[derive(Debug)]
pub struct TokioClosureWork<F> {
    closure: F,
}

impl<F> TokioClosureWork<F> {
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

impl<R, F, Fut> AsyncWork<R> for TokioClosureWork<F>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    #[inline]
    async fn run(self) -> R {
        (self.closure)().await
    }
}
