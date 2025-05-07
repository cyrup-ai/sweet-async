use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use crate::api::task::{
    AsyncTask, AsyncTaskError, TaskHandle, TaskId, TaskPriority, AsyncResult
};
use crate::api::Runtime;
use crate::api::TaskOrchestrator;

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
pub trait Orchestra<Id: Debug + Send + Sync + 'static, T: Send + 'static>: 
    Runtime<Id, T> + 
    TaskOrchestrator<Id, T> +
    AsyncTask<Id, T>
{
    /// Initialize a new task execution context
    ///
    /// Creates a fresh execution context with the given configuration options.
    /// The context tracks all tasks spawned within it and provides isolation.
    ///
    fn create_context(&self, name: &str) -> impl Orchestra<Id, T> + 'static;
    
    /// Get the current task execution statistics
    ///
    /// Returns statistics about task execution, including counts of completed,
    /// failed, and pending tasks, as well as average execution times.
    ///
    fn execution_stats(&self) -> ExecutionStats;
    
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
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// Number of tasks that have completed successfully
    pub completed_count: usize,
    /// Number of tasks that have failed
    pub failed_count: usize,
    /// Number of tasks currently running
    pub running_count: usize,
    /// Number of tasks waiting to start
    pub pending_count: usize,
    /// Average execution time across all completed tasks
    pub avg_execution_time_ms: f64,
    /// Maximum execution time observed
    pub max_execution_time_ms: u64,
    /// Minimum execution time observed
    pub min_execution_time_ms: u64,
}
