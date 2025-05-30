use crate::orchestra::OrchestratorBuilder;
use crate::task::ContextualizedTask;

use crate::task::{
    CancellableTask, MetricsEnabledTask, NamedTask, PrioritizedTask, 
    RecoverableTask, StatusEnabledTask, TimedTask, TracingTask,
};

/// Core trait for all asynchronous tasks
///
/// This trait provides the foundation for all specialized tasks,
/// ensuring they maintain identity, priority, and consistent execution.
pub trait AsyncTask<T: Clone + Send + 'static, I: crate::task::TaskId>:
    PrioritizedTask<T>
    + CancellableTask<T>
    + TracingTask<T>
    + TimedTask<T>
    + ContextualizedTask<T, I>
    + RecoverableTask<T>
    + StatusEnabledTask<T>
    + MetricsEnabledTask<T>
    + NamedTask
{
    // For a Task resolving to an awaitable future
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>;

    // For a Task that sends and receives Stream events via channels
    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>;
}
