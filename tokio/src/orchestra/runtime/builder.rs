//! Tokio implementation of RuntimeBuilder trait

use super::orchestra_runtime::TokioOrchestraRuntime;
use sweet_async_api::orchestra::runtime::RuntimeBuilder;

/// Zero-allocation Tokio runtime builder with configuration storage
#[derive(Debug)]
pub struct TokioOrchestraRuntimeBuilder<T, I> {
    worker_threads: Option<usize>,
    stack_size: Option<usize>,
    _phantom: std::marker::PhantomData<(T, I)>,
}

impl<T, I> TokioOrchestraRuntimeBuilder<T, I> {
    #[inline]
    pub fn new() -> Self {
        Self {
            worker_threads: None,
            stack_size: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, I> Default for TokioOrchestraRuntimeBuilder<T, I> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, I> RuntimeBuilder<T, I> for TokioOrchestraRuntimeBuilder<T, I>
where
    T: Clone + Send + Sync + Default + Unpin + 'static,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Unpin,
{
    #[inline]
    fn new() -> Self {
        Self::new()
    }

    #[inline]
    fn worker_threads(mut self, count: usize) -> Self {
        self.worker_threads = Some(count);
        self
    }

    #[inline]
    fn stack_size(mut self, size_bytes: usize) -> Self {
        self.stack_size = Some(size_bytes);
        self
    }

    #[inline]
    fn build(self) -> impl sweet_async_api::orchestra::runtime::Runtime<T, I> {
        // Configuration is stored but Tokio runtime creation is handled internally
        // The worker_threads and stack_size would be used when creating the underlying Tokio runtime
        TokioOrchestraRuntime::<T, I>::new()
    }
}
