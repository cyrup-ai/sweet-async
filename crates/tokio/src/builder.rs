//! Convenience builder functions for creating Tokio tasks
//!
//! This module provides simple, ergonomic functions for creating builders
//! without having to import the full builder types directly.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use sweet_async_api::orchestra::OrchestratorBuilder as ApiOrchestratorBuilder; // Renamed for clarity
use sweet_async_api::orchestra::orchestrator::TaskOrchestrator;
use sweet_async_api::task::AsyncTask as ApiAsyncTaskTrait; // Renamed for clarity
use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::builder::AsyncTaskBuilder as ApiAsyncTaskBuilderTrait; // Renamed for clarity
use sweet_async_api::task::builder::{AsyncWork, SenderStrategy}; // Added SenderStrategy
use sweet_async_api::task::emit::EmittingTaskBuilder as ApiEmittingTaskBuilderTrait; // Added EmittingTaskBuilder trait
use sweet_async_api::task::spawn::SpawningTaskBuilder as ApiSpawningTaskBuilderTrait; // Renamed for clarity // Added SenderBuilder trait

use tokio::runtime::Handle;
// Mutex removed - using atomic counters instead
use tokio::task::JoinHandle;

use crate::task::builder::TokioAsyncTaskBuilder; // This is the struct from task/builder.rs
use crate::task::spawn::builder::TokioSpawningTaskBuilder;
// use crate::task::emit::builder::TokioEmittingTaskBuilder; // Will be used by DefaultOrchestratorBuilder for Emitting path
use crate::task::async_task::AsyncTask as TokioAsyncTaskStruct;
use crate::task::emit::builder::TokioSenderBuilder; // Needed for EmittingTaskBuilder impl // Renamed for clarity

/// Creates a new base task builder with default settings
///
/// # Example
///
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
///
/// let builder = builder::<String, MyTaskId>();
/// ```
pub fn builder<T: Send + 'static, I: TaskId>() -> TokioAsyncTaskBuilder<T, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(AtomicUsize::new(0));
    TokioAsyncTaskBuilder::<T, I>::new_with_runtime(runtime, active_tasks) // Use new_with_runtime
}

/// Creates a new spawning task builder with default settings
///
/// This builder is specifically for future-based workflows that execute
/// once and return a result.
///
/// # Example
///
/// ```rust,no_run
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// struct MyTaskId(u64);
/// impl TaskId for MyTaskId {
///     fn to_string(&self) -> String { self.0.to_string() }
///     fn from_string(s: &str) -> Option<Self> { s.parse::<u64>().ok().map(MyTaskId) }
/// }
///
/// let builder = builder::spawning_builder::<String, &'static str, MyTaskId>();
/// let task = builder.run(|| async { "Hello, world!".to_string() });
/// ```
pub fn spawning_builder<T: Clone + Send + 'static, E: Send + 'static, I: TaskId>()
-> TokioSpawningTaskBuilder<T, E, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(AtomicUsize::new(0));
    TokioSpawningTaskBuilder::<T, E, I>::new(runtime, active_tasks)
}

/// Orchestrator builder that can forward method calls
pub enum DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
    Task: ApiAsyncTaskTrait<T, I> + 'static,
    I: TaskId,
{
    Spawning {
        runtime: Handle,
        active_tasks: Arc<AtomicUsize>,
        priority: TaskPriority,
        timeout: Option<Duration>,
        retry_count: u8,
        tracing_enabled: bool,
        name: Option<String>,
        _marker_task: PhantomData<fn() -> Task>, // Use function pointer to avoid 'static requirement
        _marker_t: PhantomData<T>,       // Keep marker for T
        _marker_i: PhantomData<I>,       // Keep marker for I
    },
    Emitting {
        runtime: Handle,
        active_tasks: Arc<AtomicUsize>,
        priority: TaskPriority,
        timeout: Option<Duration>,
        retry_count: u8,
        tracing_enabled: bool,
        name: Option<String>,
        _marker_task: PhantomData<fn() -> Task>, // Use function pointer to avoid 'static requirement
        _marker_t: PhantomData<T>,       // Keep marker for T
        _marker_i: PhantomData<I>,       // Keep marker for I
    },
}

impl<T, Task, I> DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
    Task: ApiAsyncTaskTrait<T, I> + 'static,
    I: TaskId,
{
    /// Create a new orchestrator builder for spawning tasks
    pub fn new_spawning() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(AtomicUsize::new(0));

        Self::Spawning {
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            timeout: None,
            retry_count: 0,
            tracing_enabled: false,
            name: None,
            _marker_task: PhantomData,
            _marker_t: PhantomData,
            _marker_i: PhantomData,
        }
    }

    /// Create a new orchestrator builder for emitting tasks
    pub fn new_emitting() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(AtomicUsize::new(0));

        Self::Emitting {
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            timeout: None,
            retry_count: 0,
            tracing_enabled: false,
            name: None,
            _marker_task: PhantomData,
            _marker_t: PhantomData,
            _marker_i: PhantomData,
        }
    }
}

// Implement ApiOrchestratorBuilder
impl<T, Task, I> ApiOrchestratorBuilder<T, Task, I> for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
    Task: ApiAsyncTaskTrait<T, I> + 'static,
    I: TaskId + 'static,
    // Adding bound that O (TaskOrchestrator) can provide runtime components
    // This is tricky because TaskOrchestrator API doesn't expose runtime directly.
    // We assume our TokioOrchestrator is being passed, or an equivalent one.
    // For now, we'll make it work specifically with TokioOrchestrator by example,
    // or require TaskOrchestrator to have methods to get runtime handle and tasks list.
    // The API for TaskOrchestrator doesn't provide these. Runtime is a separate trait.
    // Let's assume for now that if orchestrator() is called, it's a TokioOrchestrator
    // and we want to use ITS runtime context.
{
    type Next = Self; // OrchestratorBuilder returns Self to allow further chaining of AsyncTaskBuilder methods

    fn orchestrator<
        OImpl: TaskOrchestrator<T, Task, I>,
    >(
        self,
        orchestrator: &OImpl,
    ) -> Self::Next {
        // The sweet_async_api::orchestra::runtime::Runtime trait has:
        // fn runtime_handle(&self) -> &Handle; (assuming Handle is tokio::runtime::Handle via re-export or similar)
        // fn active_tasks(&self) -> Arc<Mutex<Vec<JoinHandle<()>>>>;
        // We need to ensure OImpl actually provides these.
        // The API Runtime trait has:
        //   associated_type ExecutionHandle: Clone + Send + Sync + 'static;
        //   fn get_execution_handle(&self) -> Self::ExecutionHandle;
        // This doesn't directly map to tokio::runtime::Handle or Arc<Mutex<Vec<JoinHandle<()>>>>

        // Let's make a pragmatic adjustment: if an orchestrator is passed,
        // we expect it to be our own TokioOrchestrator or something that can give us
        // a Tokio Handle and the active_tasks list. The traits are a bit abstract here.

        // For the purpose of this file, and given the tokio context,
        // let's assume OImpl is our TokioOrchestrator or provides compatible accessors.
        // This part highlights a potential area where API traits might need to be more concrete
        // or provide clearer pathways to underlying runtime specifics if builders need them.

        // To make this compile and be useful within THIS crate (sweet_async_tokio),
        // we could check if O_Impl is specifically TokioOrchestrator using Any and downcast,
        // or add methods to a tokio-specific extension trait for TaskOrchestrator.

        // Given current API traits, direct access is hard.
        // Let's refine this to use the existing fields if the Orchestrator is *not* specified,
        // and if it *is* specified, it means we should use *its* context for spawning.
        // This implies `O_Impl` must provide access to a tokio::runtime::Handle and the active_tasks list.
        // The sweet_async_api::orchestra::runtime::Runtime trait is generic over ExecutionHandle.

        // Simplification: The OrchestratorBuilder returns Self, and then the Spawning/Emitting builders use the stored context.
        // The `.orchestrator()` call will update the context within DefaultOrchestratorBuilder.

        // Since we don't have get_execution_handle() in the API, we'll use the existing runtime_handle
        // The orchestrator parameter is used to integrate with the orchestration system
        // Let's make the orchestrator method update the internal runtime and active_tasks.
        // The type `Self::Next` for `ApiOrchestratorBuilder` is `AsyncTaskBuilder` from the API.
        // My `DefaultOrchestratorBuilder` already implements `AsyncTaskBuilder`.
        // So `orchestrator` should return `Self` after updating its internal state.

        // Let's assume `O_Impl` is specifically `crate::orchestrator::TokioOrchestrator<T, I>`
        // This is a concrete assumption for this implementation.
        // A more generic solution would require API changes or trait bounds.

        // For the purpose of making progress, if orchestrator() is called,
        // we will try to downcast or expect it to be TokioOrchestrator.
        // This is not ideal for a generic trait implementation.

        // Corrected approach: The orchestrator() method should modify `self` (the DefaultOrchestratorBuilder)
        // to store the new runtime context if the provided orchestrator is a TokioOrchestrator.
        // The return type of `ApiOrchestratorBuilder::orchestrator` is `Self::Next` which is `ApiAsyncTaskBuilderTrait`.
        // `DefaultOrchestratorBuilder` *is* an `ApiAsyncTaskBuilderTrait`.

        // Let's refine: The `orchestrator` method itself doesn't return a `TokioAsyncTaskBuilder` directly,
        // but `Self` (which is `DefaultOrchestratorBuilder`), which then acts as an `ApiAsyncTaskBuilderTrait`.
        // The previous implementation for `orchestrator` returning `TokioAsyncTaskBuilder` was more for a later stage.

        // The API is: OrchestratorBuilder::orchestrator() -> impl AsyncTaskBuilder
        // So, this method should update the runtime/active_tasks in self, and then return self.

        match self {
            Self::Spawning {
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => {
                // Attempt to get Tokio-specific runtime components from orchestrator
                // This requires O_Impl to be compatible with TokioRuntime structure or provide accessors.
                // For now, we'll assume O_Impl is effectively a reference to our TokioRuntime context provider.
                // This is a HACK due to trait abstraction gaps. A real scenario would need better trait design for this.
                // Let's assume `orchestrator` is of a type that we can get `Handle` and `active_tasks_list` from.
                // This would typically be done by adding methods to sweet_async_api::orchestra::runtime::Runtime
                // or having a more specific trait bound on O_Impl.

                // If O_Impl is TokioOrchestrator, we can access its runtime.
                // This is still an assumption about O_Impl. To make it concrete and safe:
                // We would need to change O_Impl bound or how we get these.
                // For now, let this be a conceptual update, actual extraction logic is complex with current traits.
                // What if we *don't* change the runtime here, but ensure the *final task* is registered/uses it?
                // The OrchestratorBuilder is more about *linking* to an orchestrator for task registration.
                // The actual spawning might still use a local/current runtime unless specified by orchestrator.

                // Let's stick to the original design where `orchestrator()` modifies the DefaultOrchestratorBuilder internal context
                // IF the provided orchestrator is a TokioOrchestrator.
                // This is a concrete implementation detail for this tokio crate.

                // This downcast is specific to this tokio crate and assumes the user passes TokioOrchestrator
                // when using this concrete implementation of the API traits.
                // For now, keep the existing runtime context since we can't downcast without the bounds.
                // In a production system, this would require a more sophisticated approach
                // such as having the orchestrator provide runtime access methods.
                let (new_runtime_handle, new_active_tasks) = match self {
                    Self::Spawning {
                        runtime,
                        active_tasks,
                        ..
                    } => (runtime, active_tasks),
                    Self::Emitting {
                        runtime,
                        active_tasks,
                        ..
                    } => (runtime, active_tasks),
                };

                DefaultOrchestratorBuilder::Spawning {
                    runtime: new_runtime_handle,
                    active_tasks: new_active_tasks,
                    priority,
                    timeout,
                    retry_count,
                    tracing_enabled,
                    name,
                    _marker_task,
                    _marker_t,
                    _marker_i,
                }
            }
            Self::Emitting {
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => {
                // For now, keep the existing runtime context since we can't downcast without the bounds.
                let (new_runtime_handle, new_active_tasks) = match self {
                    Self::Spawning {
                        runtime,
                        active_tasks,
                        ..
                    } => (runtime, active_tasks),
                    Self::Emitting {
                        runtime,
                        active_tasks,
                        ..
                    } => (runtime, active_tasks),
                };
                DefaultOrchestratorBuilder::Emitting {
                    runtime: new_runtime_handle,
                    active_tasks: new_active_tasks,
                    priority,
                    timeout,
                    retry_count,
                    tracing_enabled,
                    name,
                    _marker_task,
                    _marker_t,
                    _marker_i,
                }
            }
        }
    }
}

// Implement ApiAsyncTaskBuilderTrait to allow method chaining directly on DefaultOrchestratorBuilder
impl<T, Task, I> ApiAsyncTaskBuilderTrait for DefaultOrchestratorBuilder<T, Task, I>
where
    T: Send + 'static,
    Task: ApiAsyncTaskTrait<T, I> + 'static, // Added 'static bound
    I: TaskId + 'static,                     // Added 'static bound
{
    fn new() -> Self {
        // Default to spawning, matches API trait expectations
        Self::new_spawning()
    }


    fn timeout(self, duration: std::time::Duration) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                retry_count,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout: Some(duration),
                retry_count,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                retry_count,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout: Some(duration),
                retry_count,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
            },
        }
    }

    fn retry(self, attempts: u8) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: attempts,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count: attempts,
                tracing_enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
            },
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
            },
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
                ..
            } => Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled: enabled,
                name,
                _marker_task,
                _marker_t,
                _marker_i,
            },
        }
    }
}

// Implement ApiSpawningTaskBuilderTrait for the Spawning variant
impl<T, I> ApiSpawningTaskBuilderTrait<T, AsyncTaskError, I>
    for DefaultOrchestratorBuilder<T, TokioAsyncTaskStruct<T, I>, I>
// Task is TokioAsyncTaskStruct
where
    T: Clone + Send + Sync + 'static,
    I: TaskId,
{
    type Task = TokioAsyncTaskStruct<T, I>; // The concrete Tokio AsyncTask struct

    fn run<F, R>(self, work: F) -> Self::Task
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
    {
        match self {
            Self::Spawning {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                ..
            } => {
                // Generate a unique task ID if not named, or use name
                // For simplicity, using a default or random ID generation logic here.
                // The API's TaskId trait allows for from_string, which can be used.
                let task_id_str = name
                    .clone()
                    .unwrap_or_else(|| format!("task-{}", uuid::Uuid::new_v4()));
                let task_id = I::from_string(&task_id_str).unwrap_or_else(|| {
                    // Fallback if from_string fails for the name or generated UUID string.
                    // This depends on how I is implemented. For uuid::Uuid, parse_str should work.
                    // A truly robust solution might require I: Default or a specific constructor.
                    let fallback_id_str = format!("fallback-task-{}", uuid::Uuid::new_v4());
                    I::from_string(&fallback_id_str)
                        .expect("Failed to create TaskId even with fallback UUID")
                });

                let mut task = TokioAsyncTaskStruct::new(
                    task_id,
                    priority,
                    runtime.clone(),
                    active_tasks.clone(),
                );

                if let Some(n) = name {
                    task = task.with_name(n);
                }
                if let Some(d) = timeout {
                    task = task.with_timeout(d);
                }
                if retry_count > 0 {
                    task = task.with_retry(retry_count);
                }
                task = task.with_tracing(tracing_enabled);

                // Create the future that will execute the work and convert its result
                let final_future = async move {
                    let work_result = work.run().await;
                    match work_result.into_async_result().await {
                        // into_async_result might be async itself
                        Ok(value) => Ok(value),
                        Err(err) => Err(err), // err is already AsyncTaskError
                    }
                };
                task.with_future(Box::pin(final_future))
            }
            Self::Emitting { .. } => panic!(
                "Cannot call run() on an emitting task builder from SpawningTaskBuilder context"
            ),
        }
    }

    fn await_result<F, R>(
        self,
        work: F,
    ) -> impl std::future::Future<Output = Result<T, AsyncTaskError>> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
    {
        let task = self.run(work); // This now correctly configures the task
        async move { task.await } // AsyncTask itself is Future
    }

    fn await_result_with_handler<F, R, H, Out>(
        self,
        work: F,
        handler: H,
    ) -> impl std::future::Future<Output = Out> + Send
    where
        F: sweet_async_api::task::builder::AsyncWork<R> + Send + 'static,
        R: sweet_async_api::task::spawn::into_async_result::IntoAsyncResult<T, AsyncTaskError>
            + Send
            + 'static,
        H: sweet_async_api::task::builder::AsyncWork<Out> + Send + 'static,
    {
        // This should work like calling .await on the future returned by run()
        // then passing that result to the handler
        let task = self.run(work);
        async move {
            let result = task.await;
            // Now pass this result to the handler which is AsyncWork<Out>
            handler.run().await
        }
    }
}

// Implement ApiEmittingTaskBuilderTrait for the Emitting variant
impl<T, C, E, I> ApiEmittingTaskBuilderTrait<T, C, E, I>
    for DefaultOrchestratorBuilder<T, TokioAsyncTaskStruct<T, I>, I>
// Assuming Task is TokioAsyncTaskStruct for context
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static, // Collected item type
    E: Send + Sync + 'static, // Error type for emitting
    I: TaskId,
{
    type SenderBuilder = TokioSenderBuilder<T, C, E, E, I>; // The Tokio concrete SenderBuilder (EItem=E, EOverall=E)

    fn sender<F>(self, sender_work_fn: F, strategy: SenderStrategy) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static, // F produces the events of type T
    {
        match self {
            Self::Emitting {
                runtime,
                active_tasks,
                priority,
                timeout,
                retry_count,
                tracing_enabled,
                name,
                ..
            } => {
                // Create the base TokioAsyncTaskBuilder and configure it
                let mut base_builder_config =
                    TokioAsyncTaskBuilder::new_with_runtime(runtime.clone(), active_tasks.clone());
                if let Some(n) = name {
                    base_builder_config = base_builder_config.name(&n);
                }
                if let Some(d) = timeout {
                    base_builder_config =
                        <TokioAsyncTaskBuilder<T, I> as ApiAsyncTaskBuilderTrait>::timeout(
                            base_builder_config,
                            d,
                        );
                }
                if retry_count > 0 {
                    base_builder_config =
                        <TokioAsyncTaskBuilder<T, I> as ApiAsyncTaskBuilderTrait>::retry(
                            base_builder_config,
                            retry_count,
                        );
                }
                base_builder_config =
                    <TokioAsyncTaskBuilder<T, I> as ApiAsyncTaskBuilderTrait>::tracing(
                        base_builder_config,
                        tracing_enabled,
                    );

                // TokioSenderBuilder::new expects specific arguments
                TokioSenderBuilder::new(
                    base_builder_config, // This now carries the common configs
                    runtime,
                    active_tasks,
                    priority,
                    sender_work_fn, // Pass the provided sender_work function
                    strategy,
                )
            }
            Self::Spawning { .. } => panic!("Cannot call sender() on a spawning task builder"),
        }
    }
}
