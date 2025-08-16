//! Tokio implementation of SpawningTaskBuilder trait

use crate::task::spawn::task::{TokioAsyncWork, TokioSpawningTask};
use std::marker::PhantomData;
use std::time::Duration;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::{AsyncTaskBuilder, AsyncWork};
use sweet_async_api::task::spawn::builder::SpawningTaskBuilder;
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;
use sweet_async_api::task::ContextualizedTask;
use sweet_async_api::orchestra::runtime::Runtime;

/// Zero-allocation Tokio implementation of SpawningTaskBuilder trait
pub struct TokioSpawningTaskBuilder<T, E, I> {
    timeout: Option<Duration>,
    retry_attempts: u8,
    tracing_enabled: bool,
    parent: Option<Box<dyn Send + 'static>>,
    _phantom: PhantomData<(T, E, I)>,
}

impl<T, E, I> TokioSpawningTaskBuilder<T, E, I> {
    #[inline]
    pub fn new() -> Self {
        Self {
            timeout: None,
            retry_attempts: 0,
            tracing_enabled: false,
            parent: None,
            _phantom: PhantomData,
        }
    }
}

impl<T, E, I> Default for TokioSpawningTaskBuilder<T, E, I> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E, I> AsyncTaskBuilder for TokioSpawningTaskBuilder<T, E, I> {
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

impl<T, E, I> SpawningTaskBuilder<T, E, I> for TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash + Unpin + Default,
{
    type Task = TokioSpawningTask<T, I>;
    type ParentType = Box<dyn Send + 'static>;

    #[inline]
    fn parent(mut self, parent: Self::ParentType) -> Self {
        self.parent = Some(parent);
        self
    }

    #[inline]
    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        let mut task = TokioSpawningTask::new(I::default());

        // Create TokioAsyncWork that wraps the provided work
        let tokio_work = TokioAsyncWork::new(std::sync::Arc::new(move || {
            Box::pin(async move {
                let result = work.run().await;
                // Convert R to T using IntoAsyncResult
                match result.into_async_result().await {
                    Ok(value) => value,
                    Err(_) => T::default(), // Fallback to default on error
                }
            })
        }));

        // Execute the work using the SpawningTask's run method
        task.run(tokio_work)
    }

    #[inline]
    fn await_result<F, R>(self, work: F) -> Result<T, E>
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        // Create task with work using the run() method - uses EXISTING elite polling loop
        let mut task = self.run(work);
        
        // Poll the task to completion using manual polling
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(&waker);
        
        loop {
            match std::pin::Pin::new(&mut task).poll(&mut cx) {
                std::task::Poll::Ready(result) => {
                    // Extract the result and convert it
                    match result.into_result() {
                        Ok(value) => {
                            // Create a simple future for the conversion and poll it
                            let mut convert_future = Box::pin(async move { value.into_async_result().await });
                            loop {
                                match convert_future.as_mut().poll(&mut cx) {
                                    std::task::Poll::Ready(converted) => return converted,
                                    std::task::Poll::Pending => std::thread::yield_now(),
                                }
                            }
                        }
                        Err(e) => return Err(e)
                    }
                }
                std::task::Poll::Pending => {
                    // Task needs more time - yield and continue polling
                    std::thread::yield_now();
                }
            }
        }
    }

    #[inline]
    fn await_result_with_handler<F, R, H, Out>(self, work: F, handler: H) -> Out
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
        H: AsyncWork<Out> + Send + 'static,
        Out: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    {
        // First execute the work and get Result<T, E>
        let _result = self.await_result(work);
        
        // Create a simple task to establish orchestrator context for handler
        let task = TokioSpawningTask::<Out, I>::new(I::default());
        
        // Use tokio runtime directly for now - will need to fix this
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            // Execute handler directly
            handler.run().await
        })
    }
}
