use std::future::Future;
use std::time::Duration;
use futures::Stream;
use uuid::Uuid;
use crate::task::emit::ReceiverEvent;

/// Builder for constructing specialized task instances with fluent API
///
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
/// 2. **Stream-based Execution**:
///    - `with_sender()`: Defines how events are produced or sourced
///    - `with_receiver()`: Specifies how to process each event
///    - `execute()`: Starts the queue and returns a QueuingTask
///
/// These two execution models provide all the functionality needed for most task scenarios.
pub trait AsyncTaskBuilder: Sized {
    /// Create a new builder instance (internal method)
    #[doc(hidden)]
    fn new() -> Self;

    /// Set the task's execution priority
    fn with_priority(self, priority: crate::task::TaskPriority) -> Self;
    
    /// Set a descriptive name for the task
    fn with_name(self, name: impl Into<String>) -> Self;
    
    /// Set a timeout duration for task execution
    fn with_timeout(self, duration: Duration) -> Self;
    
    /// Configure the number of retry attempts for failures
    fn with_retry(self, attempts: u8) -> Self;
    
    /// Enable or disable detailed execution tracing
    fn with_tracing(self, enabled: bool) -> Self;
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
pub trait SenderBuilder<T, U>: Sized {
    type Receiver: ReceiverBuilder<T, U>;
    type StreamProcessorReceiver<C, Coll>: ReceiverBuilder<T, Coll>;
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
    fn with_receiver(self, strategy: ReceiverStrategy, receiver: fn(&T, &mut (), Uuid) -> U) -> Self::Receiver;
        
    /// Add a stream processor to process the entire event stream 
    ///
    /// For advanced use cases where you need to process the entire stream at once.
    /// This provides more control but is more complex to implement correctly.
    ///
    /// # Whole-stream processing (advanced)
    /// ```
    /// |events: Stream<ReceiverEvent<User>>, collector: &mut Stats| { /* process stream */ }
    /// ```
    fn with_stream_processor<C, F, S, Coll>(self, processor: F, strategy: ReceiverStrategy) -> Self::StreamProcessorReceiver<C, Coll>
    where
        F: FnOnce(S, &mut Coll) -> C + Send + 'static,
        S: Stream + Send + 'static,
        S::Item: ReceiverEvent<T, C>,
        C: Future<Output = ()> + Send + 'static,
        Coll: Default + Send + 'static;

    type EmittingTask: crate::task::emit::EmittingTask<T, U>;
    fn execute(self) -> Self::EmittingTask;
}

/// Receiver phase of the builder pattern for queue-based tasks
pub trait ReceiverBuilder<T, U>: Sized {
    type Task: crate::task::emit::EmittingTask<T, U>;
    fn start_queue(self) -> Self::Task;
    // Add more methods as needed for results handler, error strategy, etc.
}

/// Strategy for processing events in a queue
/// 
/// Controls how events are processed. Can be applied to both
/// senders (how events are sourced) and receivers (how events are handled).
#[derive(Debug, Clone)]
pub enum ReceiverStrategy {
    /// Process sequentially
    Serial {
        timeout_seconds: u64, // default: 0 (no timeout)
    },
    /// Process in parallel
    Parallel {
        workers: usize,      // default: num_cpus::get()
        rate_limit: f64,     // default: 0.0 (no rate limit)
    },
    /// Process in batches
    Batched {
        batch_size: usize,   // default: 64
        max_delay: Duration, // default: Duration::from_millis(100)
    },
    /// Automatically adjust based on workload
    Adaptive {
        initial_capacity: usize,      // default: 64
        max_concurrency: usize,       // default: num_cpus::get()
        adaptation_window: Duration,  // default: Duration::from_secs(1)
        use_rayon_for_cpu: bool,      // default: false
    },
}

#[derive(Debug, Clone)]
pub enum SenderStrategy {
    Serial {
        timeout_seconds: u64, // default: 0 (no timeout)
    },
    Parallel {
        workers: usize,      // default: num_cpus::get()
        rate_limit: f64,     // default: 0.0 (no rate limit)
    },
    Batched {
        batch_size: usize,   // default: 64
        max_delay: Duration, // default: Duration::from_millis(100)
    },
    Adaptive {
        initial_capacity: usize,      // default: 64
        max_concurrency: usize,       // default: num_cpus::get()
        adaptation_window: Duration,  // default: Duration::from_secs(1)
        use_rayon_for_cpu: bool,      // default: false
    },
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
    Custom(fn(crate::task::AsyncTaskError) -> bool),
}

