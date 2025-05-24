use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::SpawningTaskBuilder;
use sweet_async_api::task::spawn::into_async_result::IntoAsyncResult;

use crate::task::async_task::AsyncTask;
use crate::task::builder::TokioAsyncTaskBuilder;

/// Builder for creating and configuring spawning tasks
///
/// A SpawningTaskBuilder is used to create future-based tasks that
/// execute once and return a result.
pub struct TokioSpawningTaskBuilder<T, E, I>
where
    T: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// The base builder with common configuration
    base: TokioAsyncTaskBuilder<T, I>,
    /// Task priority
    priority: TaskPriority,
    /// Phantom data for error type parameter
    _phantom_e: PhantomData<E>,
}

impl<T, E, I> TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Create a new spawning task builder
    pub fn new(
        runtime: tokio::runtime::Handle,
        active_tasks: Arc<tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    ) -> Self {
        Self {
            base: TokioAsyncTaskBuilder::new_with_runtime(runtime, active_tasks),
            priority: TaskPriority::Normal,
            _phantom_e: PhantomData,
        }
    }

    /// Set the task priority
    pub fn priority(self, priority: TaskPriority) -> Self {
        Self { priority, ..self }
    }

    /// Get the configured task name
    pub fn get_name(&self) -> Option<String> {
        self.base.get_name()
    }

    /// Get the configured timeout
    pub fn get_timeout(&self) -> std::time::Duration {
        self.base.get_timeout()
    }

    /// Get the configured retry attempts
    pub fn get_retry_attempts(&self) -> u8 {
        self.base.get_retry_attempts()
    }

    /// Check if tracing is enabled
    pub fn is_tracing_enabled(&self) -> bool {
        self.base.is_tracing_enabled()
    }

    /// Get the task priority
    pub fn get_priority(&self) -> TaskPriority {
        self.priority
    }

    /// Set a descriptive name for the task
    pub fn name(self, name: &str) -> Self {
        Self {
            base: self.base.name(name),
            ..self
        }
    }
}

impl<T, E, I> AsyncTaskBuilder for TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    fn new() -> Self {
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        Self::new(runtime, active_tasks)
    }

    // Note: name is not part of the API trait, implement on struct directly

    fn timeout(self, duration: std::time::Duration) -> Self {
        Self {
            base: self.base.timeout(duration),
            ..self
        }
    }

    fn retry(self, attempts: u8) -> Self {
        Self {
            base: self.base.retry(attempts),
            ..self
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        Self {
            base: self.base.tracing(enabled),
            ..self
        }
    }
}

impl<T, E, I> SpawningTaskBuilder<T, E, I> for TokioSpawningTaskBuilder<T, E, I>
where
    T: Clone + Send + 'static,
    E: std::fmt::Display + Send + 'static,
    I: TaskId,
    E: Into<AsyncTaskError> + From<AsyncTaskError>,
{
    type Task = AsyncTask<T, I>;

    /// Create a task with the given work function
    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        // Generate a unique task ID
        let random_id = format!(
            "task-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let id = I::from_string(&random_id).unwrap_or_else(|| {
            // Create a fallback ID string using a timestamp with a different prefix
            let fallback_id = format!(
                "fallback-task-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            );
            I::from_string(&fallback_id).expect("Failed to create task ID even with fallback")
        });

        // Create the AsyncTask with the future
        let mut task = AsyncTask::new(
            id,
            self.priority,
            self.base.runtime().clone(),
            self.base.active_tasks().clone(),
        );

        // Apply configuration
        if let Some(name) = self.base.get_name() {
            task = task.with_name(name);
        }

        let timeout_duration = self.base.get_timeout();
        if timeout_duration > std::time::Duration::from_secs(0) {
            task = task.with_timeout(timeout_duration);
        }

        let retry_count = self.base.get_retry_attempts();
        if retry_count > 0 {
            task = task.with_retry(retry_count);
        }

        let tracing_enabled = self.base.is_tracing_enabled();
        if tracing_enabled {
            task = task.with_tracing(tracing_enabled);
        }

        // Create the future that will execute the work
        let runtime = self.base.runtime().clone();

        // First create the future that runs the original work
        let work_future = work.run();

        // Then convert it to the expected result type
        let result_future = async move {
            // Execute the work and get the result
            let result = work_future.await;

            // Convert the result to the expected type
            result.into_async_result()
        };

        // This gives us a future that returns a future that returns the result
        // We need to await that inner future too
        let final_future = async move {
            // Await the inner future to get the final Result<T, E>
            let result_future = result_future.await;
            let final_result = result_future.await;

            // Convert the error type if needed
            match final_result {
                Ok(value) => Ok(value),
                Err(err) => Err(AsyncTaskError::Failure(format!("Task failed: {}", err))),
            }
        };

        // Add the future to the task
        let boxed_future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>> =
            Box::pin(final_future);
        task = task.with_future(boxed_future);

        task
    }

    /// Create and immediately await a task
    fn await_result<F, R>(self, work: F) -> impl Future<Output = Result<T, E>> + Send
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
    {
        let task = self.run(work);

        // Return a future that awaits the task and converts errors
        Box::pin(async move {
            match task.await {
                Ok(value) => Ok(value),
                Err(err) => Err(E::from(err)),
            }
        })
    }

    /// Create, await, and process with a handler
    fn await_result_with_handler<F, R, H, Out>(
        self,
        work: F,
        handler: H,
    ) -> impl Future<Output = Out> + Send
    where
        F: AsyncWork<R> + Send + 'static,
        R: IntoAsyncResult<T, E> + Send + 'static,
        H: AsyncWork<Out> + Send + 'static,
    {
        let await_result_future = self.await_result(work);

        Box::pin(async move {
            let _result = await_result_future.await;
            handler.run().await
        })
    }
}
