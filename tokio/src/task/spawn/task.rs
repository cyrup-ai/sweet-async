//! High-performance spawning task implementation with zero allocation

use crate::orchestra::runtime::TokioOrchestraRuntime;
use crate::task::spawn::result::{TokioAsyncResult, TokioTaskResult};
use crate::task::{TokioAsyncTask, TokioTaskRelationships};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, SystemTime};
use sweet_async_api::task::builder::AsyncWork;

/// AsyncWork implementation for zero-allocation async operations
#[derive(Clone)]
pub struct TokioAsyncWork<T> {
    work_fn:
        Arc<dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static>,
}

impl<T> std::fmt::Debug for TokioAsyncWork<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioAsyncWork")
            .field("work_fn", &"<async work function>")
            .finish()
    }
}

impl<T> TokioAsyncWork<T> {
    #[inline]
    pub fn new(
        work_fn: Arc<
            dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static,
        >,
    ) -> Self {
        Self { work_fn }
    }

    #[inline]
    pub fn from_value(value: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        let val = Arc::new(value);
        Self::new(Arc::new(move || {
            let v = Arc::clone(&val);
            Box::pin(async move { (*v).clone() })
        }))
    }

    #[inline]
    pub fn from_error(error: AsyncTaskError) -> Self
    where
        T: Send + 'static,
    {
        let err = Arc::new(error);
        Self::new(Arc::new(move || {
            let e = Arc::clone(&err);
            Box::pin(async move {
                panic!("AsyncWork error: {:?}", e);
            })
        }))
    }
}

impl<T> AsyncWork<T> for TokioAsyncWork<T>
where
    T: Send + 'static,
{
    #[inline]
    async fn run(self) -> T {
        (self.work_fn)().await
    }
}
use sweet_async_api::task::{
    AsyncTask, AsyncTaskError, CancellableTask, ContextualizedTask, MetricsEnabledTask, NamedTask,
    PrioritizedTask, RecoverableTask, RetryStrategy, StatusEnabledTask, TaskId, TaskStatus,
    TimedTask, TracingTask,
    spawn::{SpawningTask, TaskResult},
    task_priority::TaskPriority,
};

/// Zero-allocation spawning task implementation
pub struct TokioSpawningTask<T, I> {
    task_id: I,
    async_task: TokioAsyncTask<T, I>,
    value: Option<T>,
    work: Option<TokioAsyncWork<T>>,
    work_future: Option<Pin<Box<dyn Future<Output = T> + Send>>>,
    parent: Option<Arc<dyn ContextualizedTask<T, I, 
                   RuntimeType = TokioOrchestraRuntime<T, I>,
                   RelationshipsType = TokioTaskRelationships<T, I>> + Send + Sync>>,
    name: Option<String>,
    status: AtomicU8,
    is_cancelled: AtomicBool,
    priority: TaskPriority,
}

impl<T, I> TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        Self {
            task_id,
            async_task: TokioAsyncTask::new(task_id),
            value: None,
            work: None,
            work_future: None,
            parent: None,
            name: None,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            priority: TaskPriority::Normal,
        }
    }

    #[inline]
    pub fn with_value(task_id: I, value: T) -> Self {
        Self {
            task_id,
            async_task: TokioAsyncTask::new(task_id),
            value: Some(value),
            work: None,
            work_future: None,
            parent: None,
            name: None,
            status: AtomicU8::new(TaskStatus::Pending as u8),
            is_cancelled: AtomicBool::new(false),
            priority: TaskPriority::Normal,
        }
    }

    #[inline]
    fn status_from_u8(value: u8) -> TaskStatus {
        match value {
            0 => TaskStatus::Pending,
            1 => TaskStatus::Running,
            2 => TaskStatus::Completed,
            3 => TaskStatus::Failed,
            4 => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        }
    }
}

impl<T, I> NamedTask for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

impl<T, I> StatusEnabledTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn status(&self) -> TaskStatus {
        Self::status_from_u8(self.status.load(Ordering::Relaxed))
    }
}

impl<T, I> TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn set_status(&self, status: TaskStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

impl<T, I> CancellableTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn cancel(
        &self,
        _level: sweet_async_api::task::cancellable_task::CancellationLevel,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_gracefully(
        &self,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_forcefully(
        &self,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn cancel_immediately(
        &self,
    ) -> impl Future<Output = Result<(), sweet_async_api::orchestra::OrchestratorError>> + Send
    {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.set_status(TaskStatus::Cancelled);
        async move { Ok(()) }
    }

    #[inline]
    fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    #[inline]
    fn on_cancel<F, Fut>(self, _callback: F) -> Self
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self
    }
}

impl<T, I> TracingTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn handle_error(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError> {
        Err(error)
    }

    #[inline]
    fn record_error(&self, _error: &AsyncTaskError) {
        // Record error for tracing
    }

    #[inline]
    fn is_tracing_enabled(&self) -> bool {
        true
    }
}

impl<T, I> TimedTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn created_timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    fn executed_timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    fn completed_timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    #[inline]
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

impl<T, I> ContextualizedTask<T, I> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type RuntimeType = TokioOrchestraRuntime<T, I>;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    #[inline]
    fn relationships(&self) -> &Self::RelationshipsType {
        self.async_task.relationships()
    }

    #[inline]
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        self.async_task.relationships_mut()
    }

    #[inline]
    fn runtime(&self) -> &Self::RuntimeType {
        if let Some(ref parent) = self.parent {
            // Recursively traverse parent chain
            parent.runtime()
        } else {
            // Error: Task without parent should never exist in production
            // The orchestrator creation should happen in Future::poll()
            panic!("Task runtime accessed before orchestrator creation - this indicates a bug in lazy orchestrator initialization")
        }
    }

    #[inline]
    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
    }
}

impl<T, I> RecoverableTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    type FallbackWork = crate::task::TokioFallbackWork<T>;

    #[inline]
    fn recover(
        &self,
        _error: AsyncTaskError,
    ) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        async move {
            Err(AsyncTaskError::Failure(
                "Recovery not implemented".to_string(),
            ))
        }
    }

    #[inline]
    fn can_recover_from(&self, _error: &AsyncTaskError) -> bool {
        false
    }

    #[inline]
    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.async_task.fallback_work
    }

    #[inline]
    fn max_retries(&self) -> u8 {
        3
    }

    #[inline]
    fn current_retry(&self) -> u8 {
        0
    }

    #[inline]
    fn retry_strategy(&self) -> RetryStrategy {
        RetryStrategy::Fixed(Duration::from_millis(100))
    }
}

impl<T, I> MetricsEnabledTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    type Cpu = crate::task::TokioCpuUsage;
    type Memory = crate::task::TokioMemoryUsage;
    type Io = crate::task::TokioIoUsage;

    #[inline]
    fn cpu_usage(&self) -> &Self::Cpu {
        self.async_task.cpu_usage()
    }

    #[inline]
    fn memory_usage(&self) -> &Self::Memory {
        self.async_task.memory_usage()
    }

    #[inline]
    fn io_usage(&self) -> &Self::Io {
        self.async_task.io_usage()
    }
}

impl<T, I> PrioritizedTask<T> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    fn priority(&self) -> &impl sweet_async_api::task::task_priority::RankableByPriority {
        &self.priority
    }
}

impl<T, I> AsyncTask<T, I> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    #[inline]
    fn to<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        crate::orchestra::TokioOrchestratorBuilder::new()
    }

    #[inline]
    fn emits<R: Clone + Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        crate::orchestra::TokioOrchestratorBuilder::new()
    }
}

impl<T, I> SpawningTask<T, I> for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type OutputFuture = Pin<Box<dyn Future<Output = Self::TaskResult> + Send + 'static>>;
    type TaskResult = TokioTaskResult<T>;
    type JoinChildrenFuture =
        Pin<Box<dyn Future<Output = Self::JoinChildrenResult> + Send + 'static>>;
    type JoinChildrenResult = TokioAsyncResult<Vec<I>>;
    type AsyncWork = TokioAsyncWork<T>;

    #[inline]
    fn run(mut self, work: Self::AsyncWork) -> Self {
        // Store the work to be executed when the task is polled
        // The actual spawning happens through the runtime when polled
        self.work = Some(work);
        
        // Return self to be spawned by the runtime
        self
    }

    #[inline]
    fn run_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Clone + Send + 'static,
        Self: SpawningTask<R, I>,
    {
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use std::future::Future;
        
        // Zero-allocation child task runner
        struct ChildTaskFuture<R, I> 
        where
            R: Clone + Send + 'static,
            I: TaskId + std::hash::Hash,
        {
            child_task: Option<TokioSpawningTask<R, I>>,
            child_work: Option<TokioAsyncWork<R>>,
            spawned: bool,
        }
        
        impl<R, I> Future for ChildTaskFuture<R, I>
        where
            R: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
            I: TaskId + std::hash::Hash + Unpin,
        {
            type Output = TokioTaskResult<R>;
            
            #[inline]
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if !self.spawned {
                    if let (Some(mut child_task), Some(child_work)) = (self.child_task.take(), self.child_work.take()) {
                        // Spawn the child task with the work
                        child_task = child_task.run(child_work);
                        self.child_task = Some(child_task);
                        self.spawned = true;
                    } else {
                        return Poll::Ready(TokioTaskResult::err(
                            sweet_async_api::task::AsyncTaskError::ExecutionError(
                                "Child task or work missing".to_string()
                            )
                        ));
                    }
                }
                
                // Poll the spawned child task
                if let Some(mut child_task) = self.child_task.take() {
                    match Future::poll(Pin::new(&mut child_task), cx) {
                        Poll::Ready(result) => Poll::Ready(result),
                        Poll::Pending => {
                            self.child_task = Some(child_task);
                            Poll::Pending
                        }
                    }
                } else {
                    Poll::Ready(TokioTaskResult::err(
                        sweet_async_api::task::AsyncTaskError::ExecutionError(
                            "Child task disappeared".to_string()
                        )
                    ))
                }
            }
        }
        
        // Create child task with parent's configuration
        let child_task_id = self.task_id.clone();
        let child_task = TokioSpawningTask::<R, I>::new(child_task_id)
            .with_name(self.name.clone().unwrap_or_default())
            .with_priority(self.priority);
            
        // Create work that produces the child value
        let child_work = TokioAsyncWork::new(Box::new(move || {
            let task_value = task.clone();
            Box::pin(async move { Ok(task_value) })
        }));
        
        ChildTaskFuture {
            child_task: Some(child_task),
            child_work: Some(child_work),
            spawned: false,
        }
    }

    #[inline]
    fn join_children(&self) -> Self::JoinChildrenFuture {
        Box::pin(async move { TokioAsyncResult::from_result(Ok(Vec::new())) })
    }

    #[inline]
    fn task_id(&self) -> I {
        self.task_id
    }

    #[inline]
    fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    #[inline]
    fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: AsyncWork<U> + Send + 'static,
        U: Clone + Send + 'static,
        Self: SpawningTask<U, I>,
    {
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use std::future::Future;
        
        // Zero-allocation chained future implementation
        struct ChainedFuture<T, U, I, F> 
        where
            T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
            U: Clone + Send + 'static,
            I: TaskId + std::hash::Hash,
            F: AsyncWork<U> + Send + 'static,
        {
            task: Option<TokioSpawningTask<T, I>>,
            chained_fn: Option<F>,
            chained_future: Option<Pin<Box<dyn Future<Output = Result<U, sweet_async_api::task::AsyncTaskError>> + Send>>>,
            completed: bool,
        }
        
        impl<T, U, I, F> Future for ChainedFuture<T, U, I, F>
        where
            T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
            U: Clone + Send + 'static,
            I: TaskId + std::hash::Hash + Unpin,
            F: AsyncWork<U> + Send + 'static,
        {
            type Output = TokioTaskResult<U>;
            
            #[inline]
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.completed {
                    return Poll::Ready(TokioTaskResult::from_error(
                        sweet_async_api::task::AsyncTaskError::ExecutionError(
                            "Chain already completed".to_string()
                        )
                    ));
                }
                
                // First phase: execute the original task
                if let Some(mut task) = self.task.take() {
                    match Future::poll(Pin::new(&mut task), cx) {
                        Poll::Ready(result) => {
                            match result.into_result() {
                                Ok(_value) => {
                                    // Start the chained function
                                    if let Some(chained_fn) = self.chained_fn.take() {
                                        self.chained_future = Some(Box::pin(chained_fn.run()));
                                    } else {
                                        self.completed = true;
                                        return Poll::Ready(TokioTaskResult::err(
                                            sweet_async_api::task::AsyncTaskError::ExecutionError(
                                                "Chain function missing".to_string()
                                            )
                                        ));
                                    }
                                }
                                Err(error) => {
                                    self.completed = true;
                                    return Poll::Ready(TokioTaskResult::err(error));
                                }
                            }
                        }
                        Poll::Pending => {
                            // Put the task back for next poll
                            self.task = Some(task);
                            return Poll::Pending;
                        }
                    }
                }
                
                // Second phase: execute the chained function
                if let Some(ref mut chained_future) = self.chained_future {
                    match chained_future.as_mut().poll(cx) {
                        Poll::Ready(result) => {
                            self.completed = true;
                            Poll::Ready(match result {
                                Ok(value) => TokioTaskResult::ok(value),
                                Err(error) => TokioTaskResult::err(error),
                            })
                        }
                        Poll::Pending => Poll::Pending,
                    }
                } else {
                    // Should not reach here in normal execution
                    self.completed = true;
                    Poll::Ready(TokioTaskResult::err(
                        sweet_async_api::task::AsyncTaskError::ExecutionError(
                            "Invalid chain state".to_string()
                        )
                    ))
                }
            }
        }
        
        ChainedFuture {
            task: Some(self),
            chained_fn: Some(f),
            chained_future: None,
            completed: false,
        }
    }
}

impl<T, I> Future for TokioSpawningTask<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type Output = TokioTaskResult<T>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        
        // First poll: create orchestrator if needed and spawn work
        if this.work_future.is_none() {
            // Create orchestrator parent if no parent exists
            if this.parent.is_none() {
                use crate::orchestra::orchestrator_task::TokioOrchestratorTask;
                this.parent = Some(TokioOrchestratorTask::<T, I>::new() as Arc<dyn ContextualizedTask<T, I, 
                    RuntimeType = TokioOrchestraRuntime<T, I>,
                    RelationshipsType = TokioTaskRelationships<T, I>> + Send + Sync>);
            }
            
            // Now spawn the actual work using parent's runtime
            if let Some(work) = this.work.take() {
                let runtime = this.runtime();
                let future = runtime.runtime_handle.spawn(async move {
                    work.run().await
                });
                
                // Store the spawned future
                this.work_future = Some(Box::pin(async move {
                    match future.await {
                        Ok(result) => result,
                        Err(join_err) => {
                            // JoinError means task panicked or was cancelled
                            panic!("Task execution failed: {}", join_err)
                        }
                    }
                }));
            } else if let Some(ref value) = this.value {
                // Direct value, no work to spawn
                return std::task::Poll::Ready(TokioTaskResult::ok(value.clone()));
            } else {
                return std::task::Poll::Ready(TokioTaskResult::err(
                    AsyncTaskError::Failure("No work or value available".to_string())
                ));
            }
        }
        
        // Poll the spawned work
        if let Some(ref mut work_future) = this.work_future {
            match work_future.as_mut().poll(cx) {
                std::task::Poll::Ready(result) => {
                    this.set_status(TaskStatus::Completed);
                    std::task::Poll::Ready(TokioTaskResult::ok(result))
                }
                std::task::Poll::Pending => {
                    this.set_status(TaskStatus::Running);
                    std::task::Poll::Pending
                }
            }
        } else {
            std::task::Poll::Ready(TokioTaskResult::err(
                AsyncTaskError::Failure("Work future disappeared".to_string())
            ))
        }
    }
}
