//! Runtime builder implementation for Tokio
//!
//! Implements the RuntimeBuilder trait from the API to provide fluent
//! configuration of Tokio runtime instances.

use sweet_async_api::orchestra::runtime::{Runtime, RuntimeBuilder};
use sweet_async_api::task::TaskId;

use super::runtime_trait::TokioRuntime;

/// Tokio implementation of RuntimeBuilder trait
pub struct TokioRuntimeBuilder<T: Clone + Send + 'static, I: TaskId> {
    worker_threads: Option<usize>,
    stack_size: Option<usize>,
    _phantom: std::marker::PhantomData<(T, I)>,
}

impl<T: Clone + Send + 'static, I: TaskId> RuntimeBuilder<T, I> for TokioRuntimeBuilder<T, I> {
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
        // For now, return TokioRuntime using current handle
        // In a full implementation, we'd use the worker_threads and stack_size
        // to configure a new tokio::runtime::Runtime
        TokioRuntime::new()
    }
}