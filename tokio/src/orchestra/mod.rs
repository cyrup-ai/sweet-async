//! High-performance orchestra implementation with zero allocation

use crate::task::builder::TokioAsyncTaskBuilder;
use std::marker::PhantomData;
use sweet_async_api::orchestra::{OrchestratorBuilder, TaskOrchestrator};
use sweet_async_api::task::AsyncTask;

pub mod runtime;

/// Zero-allocation orchestra builder that stores configuration for run()
#[derive(Debug)]
pub struct TokioOrchestratorBuilder<T, Task, I> {
    _phantom: PhantomData<(T, Task, I)>,
}

impl<T, Task, I> TokioOrchestratorBuilder<T, Task, I> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T, Task, I> Default for TokioOrchestratorBuilder<T, Task, I> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Task, I> OrchestratorBuilder<T, Task, I> for TokioOrchestratorBuilder<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: AsyncTask<T, I>,
    I: sweet_async_api::TaskId,
{
    type Next = TokioAsyncTaskBuilder<T, I>;

    #[inline]
    fn orchestrator<O: TaskOrchestrator<T, Task, I>>(self, _orchestrator: &O) -> Self::Next {
        TokioAsyncTaskBuilder::new()
    }
}

/// Zero-allocation orchestra implementation
#[derive(Debug)]
pub struct TokioOrchestra<T, Task, I> {
    _phantom: PhantomData<(T, Task, I)>,
}

impl<T, Task, I> TokioOrchestra<T, Task, I> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T, Task, I> Default for TokioOrchestra<T, Task, I> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
