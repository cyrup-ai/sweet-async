use std::fmt::Debug;
use std::future::Future;

use crate::api::task::MetricsEnabledTask;

/// Cancellation severity levels for escalation
///
/// This enum represents the different levels of urgency for cancelling a task.
/// The cancellation system supports escalation - starting with a graceful request
/// and progressively becoming more forceful if the task doesn't respond.
///
/// # Cancellation Levels
///
/// ## Graceful
/// 
/// The most cooperative level of cancellation. The task is notified to stop, but
/// is given time to:
///
/// - Complete current critical operations
/// - Save partial progress
/// - Release resources properly
/// - Run registered cleanup handlers
///
/// ## Kill
///
/// A more urgent cancellation that only allows minimal cleanup:
///
/// - No new operations should start
/// - Cleanup is limited to essential resource release
/// - Long-running operations should be aborted
/// - Only highest priority cleanup handlers run
///
/// ## KillHard
///
/// The most aggressive cancellation level for emergencies:
///
/// - Task is terminated immediately
/// - No cleanup operations are allowed
/// - Resources may be released forcefully
/// - May leave resources in an inconsistent state
/// - Only use when task cannot be stopped by other means
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationLevel {
    /// Allow cleanup, wait for completion
    Graceful,
    /// Minimal cleanup, then terminate
    Kill,
    /// Immediate termination, no cleanup
    KillHard,
}

/// Result of a task cancellation operation
///
/// This trait defines the interface for querying the outcome of a task
/// cancellation request. It provides methods to check the final state
/// of the task after cancellation was attempted.
///
/// # Cancellation States
///
/// A task cancellation attempt can result in several possible states:
///
/// - **Success**: The task was successfully completed before cancellation took effect
/// - **Timeout**: The task did not respond to cancellation within the time limit
/// - **Failure**: The task encountered an error during execution
/// - **Cancelled**: The task was successfully cancelled
/// - **Running**: The task is still running despite cancellation attempt
///
/// # Usage Example
///
/// ```rust,no_run
/// async fn handle_cancellation(task: &impl AsyncTask<MyTaskId, Output>) {
///     // Attempt to cancel the task gracefully
///     let result = task.cancel_gracefully().await;
///     
///     // Check the result to determine next actions
///     if result.is_success() {
///         println!("Task completed successfully before cancellation");
///     } else if result.is_cancelled() {
///         println!("Task was cancelled at level: {:?}", result.cancellation_level());
///     } else if result.is_timeout() {
///         // Task didn't respond to graceful cancellation, escalate
///         println!("Task didn't respond to graceful cancellation, escalating");
///         task.cancel_forcefully().await;
///     } else if result.is_failure() {
///         println!("Task failed during cancellation");
///     } else if result.is_running() {
///         println!("Task is still running, might need more forceful cancellation");
///     }
/// }
/// ```
pub trait CancellationResult {
    /// Check if the cancellation was successful
    fn is_success(&self) -> bool;
    
    /// Check if the task timed out
    fn is_timeout(&self) -> bool;
    
    /// Check if the task failed
    fn is_failure(&self) -> bool;
    
    /// Check if the task was cancelled
    fn is_cancelled(&self) -> bool;
    
    /// Check if the task is still running
    fn is_running(&self) -> bool;
    
    /// Get the level at which the task was cancelled
    fn cancellation_level(&self) -> CancellationLevel;
}

/// Trait for tasks that can be cancelled
///
/// This trait provides methods for cancelling tasks at various levels
/// of urgency, from graceful shutdown to immediate termination.
///
/// # Cancellation System Design
///
/// The cancellation system is designed with these principles:
///
/// 1. **Progressive Escalation**: Start with gentle cancellation, escalate as needed
/// 2. **Resource Safety**: Ensure resources are properly released when possible
/// 3. **Structured Propagation**: Cancellation flows from parent to child tasks
/// 4. **Customizable Behavior**: Tasks control how they respond to cancellation
/// 5. **Observable Status**: Cancellation state is trackable
///
/// # Structured Concurrency
///
/// CancellableTask is a key component in implementing structured concurrency:
///
/// - Parent cancellation automatically propagates to children
/// - Child tasks inherit their parent's cancellation state
/// - Task hierarchies maintain consistent cancellation states
///
/// # Implementation Example
///
/// ```rust,no_run
/// impl<Id, T> CancellableTask<Id, T> for MyTask<Id, T>
/// where
///     Id: TaskId,
///     T: Send + 'static,
/// {
///     async fn cancel(&self, level: CancellationLevel) -> bool {
///         // Set the cancellation flag atomically
///         self.set_cancelled(true);
///         
///         // Store the cancellation level
///         self.set_cancellation_level(level);
///         
///         // Propagate cancellation to all child tasks
///         for child in self.children() {
///             child.cancel(level).await;
///         }
///         
///         // Run appropriate cleanup based on level
///         match level {
///             CancellationLevel::Graceful => self.run_all_cleanup_handlers().await,
///             CancellationLevel::Kill => self.run_essential_cleanup().await,
///             CancellationLevel::KillHard => (), // No cleanup
///         }
///         
///         // Notify listeners
///         self.notify_cancelled();
///         
///         true
///     }
///     
///     async fn cancel_gracefully(&self) -> bool {
///         self.cancel(CancellationLevel::Graceful).await
///     }
///     
///     async fn cancel_forcefully(&self) -> bool {
///         self.cancel(CancellationLevel::Kill).await
///     }
///     
///     async fn cancel_immediately(&self) -> bool {
///         self.cancel(CancellationLevel::KillHard).await
///     }
///     
///     fn is_cancelled(&self) -> bool {
///         self.cancellation_flag()
///     }
///     
///     fn on_cancel<F, Fut>(&self, callback: F)
///     where
///         F: FnOnce() -> Fut + Send + 'static,
///         Fut: Future<Output = ()> + Send + 'static
///     {
///         self.register_cancellation_callback(callback);
///     }
/// }
/// ```
pub trait CancellableTask<T: Send + 'static>: MetricsEnabledTask<T> {
    /// Cancel the task with the given level of severity
    ///
    /// Allows specifying exactly how aggressively the task should
    /// be terminated.
    fn cancel(&self, level: CancellationLevel) -> impl Future<Output = bool> + Send;
    
    /// Gracefully cancel the task, allowing it to clean up
    ///
    /// This is the preferred cancellation method when time permits,
    /// as it allows the task to release resources properly.
    fn cancel_gracefully(&self) -> impl Future<Output = bool> + Send;
    
    /// Forcefully cancel the task with minimal cleanup
    ///
    /// For when graceful cancellation is taking too long or
    /// when more urgent cancellation is needed.
    fn cancel_forcefully(&self) -> impl Future<Output = bool> + Send;
    
    /// Immediately terminate the task with no cleanup
    ///
    /// Only use this in emergency situations where the task
    /// must be stopped immediately regardless of consequences.
    fn cancel_immediately(&self) -> impl Future<Output = bool> + Send;
    
    /// Check if the task has been cancelled
    ///
    /// Returns whether a cancellation has been requested for this task.
    fn is_cancelled(&self) -> bool;
    
    /// Register a callback to be executed when the task is cancelled
    ///
    /// Allows registering cleanup or notification code to run when
    /// cancellation occurs.
    fn on_cancel<F, Fut>(&self, callback: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static;
}