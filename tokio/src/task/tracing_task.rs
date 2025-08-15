//! High-performance tracing task implementation with zero allocation

use std::sync::atomic::{AtomicBool, Ordering};
use sweet_async_api::task::{AsyncTaskError, TracingTask};

/// Zero-allocation tracing task implementation
#[derive(Debug)]
pub struct TokioTracingTask {
    tracing_enabled: AtomicBool,
}

impl TokioTracingTask {
    #[inline]
    pub fn new() -> Self {
        Self {
            tracing_enabled: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn enable_tracing(&self) {
        self.tracing_enabled.store(true, Ordering::Relaxed);
    }

    #[inline]
    pub fn disable_tracing(&self) {
        self.tracing_enabled.store(false, Ordering::Relaxed);
    }
}

impl Default for TokioTracingTask {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TracingTask<T> for TokioTracingTask
where
    T: Send + 'static,
{
    #[inline]
    fn is_tracing_enabled(&self) -> bool {
        self.tracing_enabled.load(Ordering::Relaxed)
    }

    #[inline]
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        if <TokioTracingTask as TracingTask<T>>::is_tracing_enabled(self) {
            tracing::error!("Task error: {:?}", error);
        }
        Err(error)
    }

    #[inline]
    fn record_error(&self, error: &AsyncTaskError) {
        if <TokioTracingTask as TracingTask<T>>::is_tracing_enabled(self) {
            tracing::error!("Recording task error: {:?}", error);
        }
    }
}
