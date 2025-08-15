//! Tokio implementation of Orchestrator trait

use sweet_async_api::orchestra::TaskOrchestrator;

/// Tokio implementation of TaskOrchestrator trait
#[derive(Debug)]
pub struct TokioOrchestrator<T, Task, I> {
    _phantom: std::marker::PhantomData<(T, Task, I)>,
}

impl<T, Task, I> TokioOrchestrator<T, Task, I> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, Task, I> TaskOrchestrator<T, Task, I> for TokioOrchestrator<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: sweet_async_api::AsyncTask<T, I>,
    I: sweet_async_api::TaskId,
{
    // TODO: Implement TaskOrchestrator methods exactly as defined in API
}