//! High-performance cancellable task implementation with zero allocation

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::{CancellableTask, cancellable_task::CancellationLevel};

/// Zero-allocation cancellable task implementation
#[derive(Debug)]
pub struct TokioCancellableTask {
    is_cancelled: AtomicBool,
}

impl TokioCancellableTask {
    #[inline]
    pub fn new() -> Self {
        Self {
            is_cancelled: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::Relaxed);
    }
}

impl Default for TokioCancellableTask {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CancellableTask<T> for TokioCancellableTask
where
    T: Send + 'static,
{
    #[inline]
    fn cancel(
        &self,
        _level: CancellationLevel,
    ) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.is_cancelled.store(true, Ordering::Relaxed);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.is_cancelled.store(true, Ordering::Relaxed);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.is_cancelled.store(true, Ordering::Relaxed);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.is_cancelled.store(true, Ordering::Relaxed);
        async move { Ok(()) }
    }

    #[inline]
    fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    #[inline]
    fn on_cancel<F, Fut>(self, _callback: F) -> Self
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self
    }
}
