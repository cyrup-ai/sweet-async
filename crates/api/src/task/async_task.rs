use crate::api::task::spawn::result::{AsyncResult, TaskResult};
use crate::api::task::ContextualizedTask;
use crate::api::task::builder::OrchestratorBuilder;

use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use futures::Stream;
use std::pin::Pin;
use uuid::Uuid;

use crate::api::task::{
    AsyncTaskError, CancellableTask, PrioritizedTask, 
    RecoverableTask, TaskStatus, TracingTask, TimedTask, StatusEnabledTask, MetricsEnabledTask
};

/// Core trait for all asynchronous tasks
///
/// This trait provides the foundation for all specialized tasks,
/// ensuring they maintain identity, priority, and consistent execution.
pub trait AsyncTask<T: Send + 'static, I: crate::api::task::TaskId>:
    PrioritizedTask<T> + 
    CancellableTask<T> +
    TracingTask<T> + 
    TimedTask<T> +
    ContextualizedTask<T, I> +
    RecoverableTask<T> + 
    StatusEnabledTask<T> +
    MetricsEnabledTask<T>
{
    // For a Task resolving to an awaitable future
    fn resolves_to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>;
    
    // For a Task that sends and receives Stream events via channels
    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>;
}

