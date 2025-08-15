pub mod orchestrator;
pub use orchestrator::{OrchestratorError, TaskOrchestrator};
pub mod runtime;
pub use runtime::*;

// TaskOrchestrator is now publicly exported above
use crate::task::AsyncTask;
use crate::task::builder::AsyncTaskBuilder;
use crate::task::{TaskId, TaskPriority};

//
// Stateful Task Orchestration Runtime and Context
// ----------------------
//
/// Orchestra combines runtime capabilities with task orchestration
///
/// The Orchestra trait provides a unified interface for task execution and orchestration,
/// combining the capabilities of Runtime and TaskOrchestrator with AsyncTask functionality.
/// This enables complex task workflows with dependency tracking and group operations.
///
/// By implementing AsyncTask, the Orchestra itself can be spawned as a task in another runtime,
/// creating a powerful composable pattern for nested task management.
pub trait Orchestra<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId>:
    Runtime<T, I> + TaskOrchestrator<T, Task, I> + AsyncTask<T, I>
{
    /// Initialize a new task execution context
    ///
    /// Creates a fresh execution context with the given configuration options.
    /// The context tracks all tasks spawned within it and provides isolation.
    ///
    fn create_context(&self, name: &str) -> impl Orchestra<T, Task, I> + 'static;

    /// Get the current task execution statistics
    ///
    /// Returns statistics about task execution, including counts of completed,
    /// failed, and pending tasks, as well as average execution times.
    ///
    type Stats: ExecutionStats + Clone + Send + 'static;
    type StatsTask: AsyncTask<Self::Stats, I>;
    fn execution_stats(&self) -> Self::StatsTask;

    /// Clear all completed tasks from the orchestra's tracking
    ///
    /// Removes all completed tasks from memory, freeing resources while
    /// preserving running and pending tasks.
    ///
    fn clear_completed_tasks(&self) -> usize;

    /// Override the default task priority for all tasks in this context
    ///
    /// Sets a new default priority that will be applied to all tasks spawned
    /// in this context that don't explicitly specify a different priority.
    ///
    fn set_default_priority(&self, priority: TaskPriority);

    /// Get the name of this orchestra context
    ///
    /// Returns the name assigned to this execution context.
    ///
    fn context_name(&self) -> &str;
}

/// Statistics about task execution within an Orchestra
pub trait ExecutionStats {
    /// Number of tasks that have completed successfully
    fn completed_count(&self) -> usize;

    /// Number of tasks that have failed
    fn failed_count(&self) -> usize;

    /// Number of tasks currently running
    fn running_count(&self) -> usize;

    /// Number of tasks waiting to start
    fn pending_count(&self) -> usize;

    /// Average execution time across all completed tasks
    fn avg_execution_time_ms(&self) -> f64;

    /// Maximum execution time observed
    fn max_execution_time_ms(&self) -> u64;

    /// Minimum execution time observed
    fn min_execution_time_ms(&self) -> u64;
}

/// Initial builder for setting the orchestrator before configuring the task
pub trait OrchestratorBuilder<
    T: Clone + Send + 'static,
    Task: crate::task::AsyncTask<T, I>,
    I: crate::task::TaskId,
>
{
    type Next: AsyncTaskBuilder;
    fn orchestrator<O: crate::orchestra::TaskOrchestrator<T, Task, I>>(
        self,
        orchestrator: &O,
    ) -> Self::Next;
}

#[allow(unused)]
pub use orchestrator::*;

#[allow(unused)]
pub use runtime::*;
