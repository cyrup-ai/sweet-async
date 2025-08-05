//! Tokio implementation of the Orchestra trait
//!
//! This module provides the TokioOrchestra struct that implements the Orchestra trait
//! from the API, combining Runtime, TaskOrchestrator, and AsyncTask capabilities.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use sweet_async_api::orchestra::{Orchestra, ExecutionStats, OrchestratorError};
use sweet_async_api::orchestra::{Runtime, TaskOrchestrator};
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority};

use crate::orchestra::{TokioRuntime, TokioOrchestrator, TokioExecutionStats};
use crate::task::{TokioAsyncTask, TokioTaskId};

/// Tokio implementation of the Orchestra trait
/// 
/// Combines runtime execution with task orchestration capabilities.
/// Can spawn tasks, manage their lifecycle, and track execution statistics.
#[derive(Clone)]
pub struct TokioOrchestra<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> {
    runtime: TokioRuntime,
    orchestrator: TokioOrchestrator<T, I>,
    context_name: String,
    default_priority: Arc<std::sync::Mutex<TaskPriority>>,
    stats: TokioExecutionStats,
    _phantom: std::marker::PhantomData<Task>,
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> TokioOrchestra<T, Task, I> {
    /// Create a new TokioOrchestra with the given context name
    pub fn new(context_name: String) -> Self {
        Self {
            runtime: TokioRuntime::new(),
            orchestrator: TokioOrchestrator::new(),
            context_name,
            default_priority: Arc::new(std::sync::Mutex::new(TaskPriority::Normal)),
            stats: TokioExecutionStats::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create a new TokioOrchestra with default settings
    pub fn default() -> Self {
        Self::new("default".to_string())
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> Runtime<T, I> for TokioOrchestra<T, Task, I> {
    type SpawnedTask = <TokioRuntime as Runtime<T, I>>::SpawnedTask;
    
    fn spawn(
        &self,
        task: impl sweet_async_api::task::spawn::SpawningTask<T, I> + 'static,
        priority: TaskPriority,
    ) -> Self::SpawnedTask {
        self.runtime.spawn(task, priority)
    }
    
    fn block_on<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send + 'static,
    {
        self.runtime.block_on(future)
    }
    
    fn active_task_count(&self) -> usize {
        self.runtime.active_task_count()
    }
    
    fn shutdown(&self, timeout: Duration) -> Result<(), OrchestratorError> {
        self.runtime.shutdown(timeout)
    }
    
    fn is_running(&self) -> bool {
        self.runtime.is_running()
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> TaskOrchestrator<T, Task, I> for TokioOrchestra<T, Task, I> {
    type Group = <TokioOrchestrator<T, I> as TaskOrchestrator<T, Task, I>>::Group;
    
    fn orchestrate(
        &self,
        tasks: Vec<Task>,
    ) -> impl Future<Output = Result<Vec<T>, OrchestratorError>> + Send {
        self.orchestrator.orchestrate(tasks)
    }
    
    fn orchestrate_with_dependencies(
        &self,
        tasks: Vec<(Task, Vec<I>)>,
    ) -> impl Future<Output = Result<Vec<T>, OrchestratorError>> + Send {
        self.orchestrator.orchestrate_with_dependencies(tasks)
    }
    
    fn create_group(&self, name: &str) -> Self::Group {
        self.orchestrator.create_group(name)
    }
    
    fn add_to_group(&self, group: &Self::Group, task: Task) -> Result<(), OrchestratorError> {
        self.orchestrator.add_to_group(group, task)
    }
    
    fn execute_group(
        &self,
        group: Self::Group,
    ) -> impl Future<Output = Result<Vec<T>, OrchestratorError>> + Send {
        self.orchestrator.execute_group(group)
    }
    
    fn cancel_group(
        &self,
        group: &Self::Group,
    ) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        self.orchestrator.cancel_group(group)
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> AsyncTask<T, I> for TokioOrchestra<T, Task, I> {
    type TaskId = I;
    
    fn task_id(&self) -> Self::TaskId {
        // Generate a unique task ID for this orchestra instance
        I::from_string("orchestra").unwrap_or_else(|| {
            // Fallback if from_string fails
            unsafe { std::mem::zeroed() }
        })
    }
    
    fn run(
        self,
        work: impl sweet_async_api::task::builder::AsyncWork<T> + Send + 'static,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        async move {
            work.run().await.map_err(|e| AsyncTaskError::ExecutionFailed(format!("Orchestra execution failed: {}", e)))
        }
    }
    
    fn cancel(
        &self,
        level: sweet_async_api::task::cancellable_task::CancellationLevel,
    ) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        async move {
            self.shutdown(Duration::from_secs(30)).map_err(|_| OrchestratorError)
        }
    }
    
    fn cancel_gracefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        async move {
            self.shutdown(Duration::from_secs(60)).map_err(|_| OrchestratorError)
        }
    }
    
    fn cancel_forcefully(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        async move {
            self.shutdown(Duration::from_secs(5)).map_err(|_| OrchestratorError)
        }
    }
    
    fn cancel_immediately(&self) -> impl Future<Output = Result<(), OrchestratorError>> + Send {
        async move {
            self.shutdown(Duration::from_millis(100)).map_err(|_| OrchestratorError)
        }
    }
    
    fn to<R: Clone + Send + 'static, NewTask: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, NewTask, I> {
        crate::task::builder::DefaultOrchestratorBuilder::new()
    }
    
    fn emits<R: Clone + Send + 'static, NewTask: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, NewTask, I> {
        crate::task::builder::DefaultOrchestratorBuilder::new()
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> Orchestra<T, Task, I> for TokioOrchestra<T, Task, I> {
    fn create_context(&self, name: &str) -> impl Orchestra<T, Task, I> + 'static {
        TokioOrchestra::new(name.to_string())
    }
    
    type Stats = TokioExecutionStats;
    type StatsTask = TokioAsyncTask<Self::Stats, I>;
    
    fn execution_stats(&self) -> Self::StatsTask {
        let stats = self.stats.clone();
        TokioAsyncTask::new(async move { Ok(stats) }, TokioTaskId::new().into())
    }
    
    fn clear_completed_tasks(&self) -> usize {
        // In this simple implementation, we'll return 0 since we don't track completed tasks
        0
    }
    
    fn set_default_priority(&self, priority: TaskPriority) {
        if let Ok(mut current_priority) = self.default_priority.lock() {
            *current_priority = priority;
        }
    }
    
    fn context_name(&self) -> &str {
        &self.context_name
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId> Future for TokioOrchestra<T, Task, I> {
    type Output = Result<T, AsyncTaskError>;
    
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Orchestra as a Future completes immediately with an error since it's meant to be used as a runtime
        Poll::Ready(Err(AsyncTaskError::ExecutionFailed("Orchestra cannot be awaited directly".to_string())))
    }
}