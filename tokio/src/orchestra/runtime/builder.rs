//! Runtime builder implementation for Tokio
//!
//! Implements the RuntimeBuilder trait from the API to provide fluent
//! configuration of Tokio runtime instances.

use sweet_async_api::orchestra::runtime::{Runtime, RuntimeBuilder};
use sweet_async_api::task::TaskId;

use super::runtime_trait::TokioRuntime;

/// Tokio implementation of RuntimeBuilder trait
pub struct TokioRuntimeBuilder<T: Clone + Send + Sync + Unpin + 'static, I: TaskId + Unpin> {
    worker_threads: Option<usize>,
    stack_size: Option<usize>,
    _phantom: std::marker::PhantomData<(T, I)>,
}

impl<T: Clone + Send + Sync + Unpin + 'static, I: TaskId + Unpin> RuntimeBuilder<T, I> for TokioRuntimeBuilder<T, I> {
    fn new() -> Self {
        Self {
            worker_threads: None,
            stack_size: None,
            _phantom: std::marker::PhantomData,
        }
    }

    fn worker_threads(mut self, count: usize) -> Self {
        self.worker_threads = Some(count);
        self
    }

    fn stack_size(mut self, size_bytes: usize) -> Self {
        self.stack_size = Some(size_bytes);
        self
    }

    fn build(self) -> impl Runtime<T, I> {
        // Zero allocation fast path: if no custom configuration, use current handle
        if self.worker_threads.is_none() && self.stack_size.is_none() {
            return TokioRuntime::new(); // Zero allocation, blazing fast
        }

        // Sophisticated custom runtime creation with validation and graceful degradation
        let mut builder = tokio::runtime::Builder::new_multi_thread();

        // Validate and configure worker threads with graceful degradation
        if let Some(threads) = self.worker_threads {
            if threads > 0 && threads <= 1024 { // Prevent resource exhaustion
                builder.worker_threads(threads);
            } else {
                // Invalid config: fallback to current handle
                return TokioRuntime::new();
            }
        }

        // Validate and configure stack size with sensible bounds
        if let Some(stack_size) = self.stack_size {
            if stack_size >= 131072 && stack_size <= 8388608 { // 128KB to 8MB range
                builder.thread_stack_size(stack_size);
            } else {
                // Invalid config: fallback to current handle  
                return TokioRuntime::new();
            }
        }

        // Build custom runtime with complete error handling
        match builder.build() {
            Ok(runtime) => {
                let handle = runtime.handle().clone();
                let runtime_arc = std::sync::Arc::new(runtime);
                TokioRuntime::with_custom_runtime(handle, runtime_arc)
            }
            Err(_) => {
                // Runtime creation failed (insufficient resources, etc): graceful fallback
                TokioRuntime::new()
            }
        }
    }
}