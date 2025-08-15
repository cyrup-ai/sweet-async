//! Tokio implementation of EmittingTaskBuilder trait

use std::pin::Pin;
use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy,
};

/// AsyncWork implementation for boxed futures in emit tasks
pub struct TokioEmitAsyncWork<T> {
    future: Pin<Box<dyn std::future::Future<Output = T> + Send + 'static>>,
}

impl<T> std::fmt::Debug for TokioEmitAsyncWork<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioEmitAsyncWork")
            .field("future", &"<boxed future>")
            .finish()
    }
}

impl<T> TokioEmitAsyncWork<T> {
    pub fn new(future: Pin<Box<dyn Future<Output = T> + Send + 'static>>) -> Self {
        Self { future }
    }
}

impl<T> AsyncWork<T> for TokioEmitAsyncWork<T>
where
    T: Send + 'static,
{
    async fn run(self) -> T {
        self.future.await
    }
}
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::emit::EmittingTask;
use sweet_async_api::task::emit::builder::{EmittingTaskBuilder, ReceiverBuilder, SenderBuilder};

/// Zero-allocation Tokio implementation of EmittingTaskBuilder trait
#[derive(Debug)]
pub struct TokioEmittingTaskBuilder<T, C, E, I> {
    timeout: Option<Duration>,
    retry_attempts: u8,
    tracing_enabled: bool,
    _phantom: PhantomData<(T, C, E, I)>,
}

impl<T, C, E, I> TokioEmittingTaskBuilder<T, C, E, I> {
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

impl<T, C, E, I> Default for TokioEmittingTaskBuilder<T, C, E, I> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, C, E, I> AsyncTaskBuilder for TokioEmittingTaskBuilder<T, C, E, I> {
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

/// Zero-allocation Tokio sender builder
#[derive(Debug)]
pub struct TokioSenderBuilder<T, C, E, I> {
    sender_work: Option<TokioEmitAsyncWork<T>>,
    strategy: SenderStrategy,
    batch_size: Option<usize>,
    _phantom: PhantomData<(C, E, I)>,
}

impl<T, C, E, I> TokioSenderBuilder<T, C, E, I> {
    #[inline]
    pub fn new(strategy: SenderStrategy) -> Self {
        Self {
            sender_work: None,
            strategy,
            batch_size: None,
            _phantom: PhantomData,
        }
    }
}

/// Zero-allocation Tokio receiver builder
#[derive(Debug)]
pub struct TokioReceiverBuilder<T, C, E, I> {
    receiver_work: Option<TokioEmitAsyncWork<C>>,
    strategy: ReceiverStrategy,
    _phantom: PhantomData<(T, E, I)>,
}

impl<T, C, E, I> TokioReceiverBuilder<T, C, E, I> {
    #[inline]
    pub fn new(strategy: ReceiverStrategy) -> Self {
        Self {
            receiver_work: None,
            strategy,
            _phantom: PhantomData,
        }
    }
}

impl<T, C, E, I> EmittingTaskBuilder<T, C, E, I> for TokioEmittingTaskBuilder<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + Default + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Default + Unpin,
{
    type SenderBuilder = TokioSenderBuilder<T, C, E, I>;

    #[inline]
    fn sender<F>(self, _sender: F, strategy: SenderStrategy) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static,
    {
        TokioSenderBuilder::new(strategy)
    }

    #[inline]
    fn sequence<F>(self, _sender: F) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static,
    {
        TokioSenderBuilder::new(SenderStrategy::Serial { timeout_seconds: 0 })
    }
}

impl<T, C, E, I> SenderBuilder<T, C, E, I> for TokioSenderBuilder<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + Default + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Default + Unpin,
{
    type ReceiverBuilder = TokioReceiverBuilder<T, C, E, I>;

    #[inline]
    fn with<D>(self, _dependency: D) -> Self
    where
        D: Clone + Send + 'static,
    {
        self
    }

    #[inline]
    fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = Some(batch_size);
        self
    }

    #[inline]
    fn receiver<F>(self, _receiver: F, strategy: ReceiverStrategy) -> Self::ReceiverBuilder
    where
        F: AsyncWork<C> + Send + 'static,
    {
        TokioReceiverBuilder::new(strategy)
    }
}

impl<T, C, E, I> ReceiverBuilder<T, C, E, I> for TokioReceiverBuilder<T, C, E, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    C: Send + Sync + Clone + Default + 'static,
    E: Send + 'static,
    I: TaskId + std::hash::Hash + Default + Unpin,
{
    type Task = crate::task::emit::task::TokioEmittingTask<T, C, E, I>;

    #[inline]
    fn run(self) -> Self::Task {
        crate::task::emit::task::TokioEmittingTask::new(Default::default())
    }

    #[inline]
    fn await_result(
        self,
    ) -> impl Future<Output = (C, <Self::Task as EmittingTask<T, C, E, I>>::Final)> + Send {
        async move {
            let task = self.run();
            let final_event = task.await_final_event(Default::default()).await;
            (Default::default(), final_event)
        }
    }
}
