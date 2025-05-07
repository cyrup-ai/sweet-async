use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use futures::Stream;

use crate::api::AsyncTask;
use crate::api::task::{TaskPriority, TaskResult, AsyncTaskError};
use crate::api::task::spawn::SpawningTask;
use crate::api::task::queue::{QueuingTask,  ReceiverEvent};
use crate::api::task::spawn::SpawningTaskBuilder;
use crate::api::task::emit::EmittingTaskBuilder;

/// Builder for constructing specialized task istances with fluent API
//
/// The AsyncTaskBuilder provides a clean, method-chaining API for configuring
/// and creating tasks. It follows the Builder pattern to make task creation
/// expressive and type-safe.
///
/// The builder offers two main execution models:
///
/// 1. **Future-based Execution**:
///    - `spawn()`: Returns a SpawningTask (AsyncTask + Future)
///    - When awaited, produces a TaskResult (AsyncTask + Result)
///
/// 2. **Queue-based Execution**:
///    - `with_sender()`: Defines how events are produced or sourced
///    - `with_receiver()`: Specifies how to process each event
///    - `start_queue()`: Starts the queue and returns a QueuingTask
///
/// These two execution models provide all the functionality needed for most task scenarios.
pub trait AsyncTaskBuilder: Sized {
    /// Create a new builder instance (internal method)
    #[doc(hidden)]
    fn new() -> Self;

    /// Start building a new task
    fn builder() -> Self;
    
    /// Set the task's execution priority
    fn with_priority(self, priority: TaskPriority) -> Self;
    
    /// Set a descriptive name for the task
    fn with_name(self, name: impl Into<String>) -> Self;
    
    /// Set a timeout duration for task execution
    fn with_timeout(self, duration: Duration) -> Self;
    
    /// Configure the number of retry attempts for failures
    fn with_retry(self, attempts: u8) -> Self;
    
    /// Enable or disable detailed execution tracing
    fn with_tracing(self, enabled: bool) -> Self;

    /// Create a task that executes the given work function
    ///
    /// Returns a SpawningTask that implements both AsyncTask and Future.
    /// When awaited, it produces a TaskResult (AsyncTask + Result).
    fn spawn<W>(self, work: W) -> impl SpawningTask<Id, T> + Future<Output = impl TaskResult<Id, T>> + Send + 'static
    where
        W: AsyncWork<T> + Send + 'static;
        
    /// Define a sender for queue-based task execution
    ///
    /// This method transitions the builder to the sender phase.
    /// The sender function generates events that will be processed by the task.
    /// The strategy controls how events are produced (serial, parallel, batched, or adaptive).
    fn with_sender(self, strategy: SenderStrategy, sender: impl FnOnce(&mut Collector<T, Id, U>) + Send + 'static) -> SenderBuilder<Id, T, U>;

    fn resolves_to<T: Send + 'static>() -> SpawningTaskBuilder<T>;
    fn emits<T: Send + 'static, U: Send + 'static>() -> EmittingTaskBuilder<T, U>;
}

/// Trait for asynchronous work that produces a result
///
/// This trait represents work that can be executed asynchronously and
/// produces a result of type R. The work is executed by calling `execute()`
/// which returns a Future that resolves to the result.
pub trait AsyncWork<R> {
    /// Execute the work and return a Future that resolves to the result
    fn execute(self) -> impl Future<Output = R> + Send + 'static;
}

// Implementation for async closures
impl<R, F, Fut> AsyncWork<R> for F 
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
{
    fn execute(self) -> impl Future<Output = R> + Send + 'static {
        self()
    }
}

/// Sender phase of the builder pattern for queue-based tasks
pub trait SenderBuilder<Id: Debug + Send + Sync + 'static, T: Send + 'static, U: Send + 'static>: Sized {
    /// Add a receiver to process task events with a collector
    ///
    /// The receiver is responsible for handling events as they're produced.
    /// The strategy controls how events are consumed (serial, parallel, batched).
    /// This method transitions the builder to the receiver phase.
    ///
    /// # Event-by-event processing (recommended)
    /// ```
    /// |event: &ReceiverEvent<User>, collector: &mut Stats| { /* process one event */ }
    /// ```
    fn with_receiver(self, receiver: impl FnMut(&ReceiverEvent<T>, &mut Collector<T, Id, U>) + Send + 'static, strategy: ReceiverStrategy) -> ReceiverBuilder<Id, T, U>;
        
    /// Add a stream processor to process the entire event stream 
    ///
    /// For advanced use cases where you need to process the entire stream at once.
    /// This provides more control but is more complex to implement correctly.
    ///
    /// # Whole-stream processing (advanced)
    /// ```
    /// |events: Stream<ReceiverEvent<User>>, collector: &mut Stats| { /* process stream */ }
    /// ```
    fn with_stream_processor<C, F, S, Coll>(self, processor: F, strategy: ReceiverStrategy) -> impl ReceiverBuilder<Id, T, Coll>
    where
        F: FnOnce(S, &mut Coll) -> C + Send + 'static,
        S: Stream<Item = ReceiverEvent<T>> + Send + 'static,
        C: Future<Output = ()> + Send + 'static,
        Coll: Default + Send + 'static;

    fn execute(self) -> Box<dyn crate::api::task::emit::EmittingTask<Id, T, U>>;
}

/// Receiver phase of the builder pattern for queue-based tasks
pub trait ReceiverBuilder<Id: Debug + Send + Sync + 'static, T: Send + 'static, U: Send + 'static>: Sized {
    /// Start the queue processing
    ///
    /// This finalizes the builder chain and returns an EmittingTask that
    /// can be awaited or monitored. The task will process events according
    /// to the chosen strategy and collect results into the collector.
    fn start_queue(self) -> impl crate::api::task::queue::EmittingTask<Id, T, C> + Send + 'static;
    
    /// Configure how results are handled when the task is complete
    ///
    /// The handler receives the collector after processing is complete.
    /// If not specified, the collector is returned by await_final_event().
    fn with_results_handler<H, R>(self, handler: H) -> Self
    where
        H: FnOnce(C) -> R + Send + 'static,
        R: Future<Output = ()> + Send + 'static;
        
    /// Configure error handling strategy with custom error handler
    ///
    /// The error handler is called when an error occurs during processing.
    /// It can be used to implement custom error recovery or logging.
    ///
    /// Controls how errors encountered during processing are handled.
    fn with_error_strategy(self, strategy: ErrorStrategy) -> Self;

    fn execute(self) -> Box<dyn crate::api::task::emit::EmittingTask<Id, T, U>>;
}

/// Strategy for processing events in a queue
/// 
/// Controls how events are processed. Can be applied to both
/// senders (how events are sourced) and receivers (how events are handled).
pub enum ReceiverStrategy {
    /// Process sequentially
    Serial,
    
    /// Process in parallel
    Parallel { 
        /// Number of worker threads (defaults to system CPU cores if None)
        workers: Option<usize>,
        /// Optional rate limiting (events per second)
        rate_limit: Option<f64>
    },
    
    /// Process in batches
    Batched { 
        /// Size of each batch
        batch_size: usize,
        /// Maximum delay before processing a partial batch
        max_delay: Duration
    },
    
    /// Automatically adjust based on workload
    Adaptive { 
        /// Initial capacity (items per collection)
        initial_capacity: usize,
        /// Maximum concurrent operations
        max_concurrency: Option<usize>,
        /// Window of time for adaptation decisions
        adaptation_window: Duration,
        /// Use Rayon for CPU-bound tasks
        use_rayon_for_cpu: bool
    }
}

/// Strategies for handling errors in queue processing
pub enum ErrorStrategy {
    /// Stop processing on the first error
    StopOnFirst,
    /// Continue processing despite errors
    ContinueOnError,
    /// Retry failed operations with the specified attempts
    Retry(u8),
    /// Custom error handling logic
    Custom(Box<dyn Fn(AsyncTaskError) -> bool + Send + 'static>),
}
