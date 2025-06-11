use crate::task::emit::ReceiverEvent;
use futures::Stream;
use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;
use uuid::Uuid;

/// Represents a range with minimum and maximum values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MinMax<T>(pub T, pub T)
where
    T: PartialOrd + Copy + Debug;

impl<T> MinMax<T>
where
    T: PartialOrd + Copy + Debug,
{
    /// Create a new range with minimum and maximum values
    pub fn new(min: T, max: T) -> Self {
        debug_assert!(
            min <= max,
            "MinMax: min ({:?}) must be <= max ({:?})",
            min,
            max
        );
        Self(min, max)
    }

    /// Get the minimum value
    pub fn min(&self) -> T {
        self.0
    }

    /// Get the maximum value
    pub fn max(&self) -> T {
        self.1
    }
}

/// Builder for constructing specialized task instances with fluent API
///
/// The AsyncTaskBuilder provides a clean, method-chaining API for configuring
/// and creating tasks. It follows the Builder pattern to make task creation
/// expressive and type-safe.
///
/// The builder offers two main execution models:
///
/// 1. **Future-based Execution**:
///    - When awaited, produces a TaskResult (AsyncTask + Result)
///
/// 2. **Stream-based Execution**:
///    - `sender()`: Defines how events are produced or sourced
///    - `receiver()`: Specifies how to process each event
///    - `run()`: Starts the task and returns a handle
///
/// These two execution models provide all the functionality needed for most task scenarios.
pub trait AsyncTaskBuilder: Sized {
    /// Sets a descriptive name for the task.

    /// Sets a timeout duration for task execution.
    fn timeout(self, duration: std::time::Duration) -> Self;
    /// Sets the task's execution priority.

    /// Configure the number of retry attempts for failures
    fn retry(self, attempts: u8) -> Self;
    /// Enable or disable detailed execution tracing
    fn tracing(self, enabled: bool) -> Self;

    /// Create a new builder instance (internal method)
    #[doc(hidden)]
    fn new() -> Self;
}

pub trait AsyncWork<R> {
    /// Execute the work and return a Future that resolves to the result
    fn run(self) -> impl Future<Output = R> + Send + 'static;
}

// Implementation for async closures
impl<R, F, Fut> AsyncWork<R> for F
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
{
    fn run(self) -> impl Future<Output = R> + Send + 'static {
        self()
    }
}

/// Sender phase of the builder pattern for queue-based tasks
pub trait SenderBuilder<
    T: Clone + Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: crate::task::task_id::TaskId,
>: Sized
{
    type Receiver: ReceiverBuilder<T, U, E, I>;
    type StreamProcessorReceiver<C: Send + 'static, Coll: Send + 'static>: ReceiverBuilder<T, Coll, E, I>;
    /// Add a receiver to process task events with a collector
    fn receiver(
        self,
        strategy: ReceiverStrategy,
        receiver: fn(&T, &mut (), Uuid) -> U,
    ) -> Self::Receiver;

    /// Add a stream processor to process the entire event stream
    fn stream_processor<C: Send + 'static, F, S, Coll: Send + 'static>(
        self,
        processor: F,
        strategy: ReceiverStrategy,
    ) -> Self::StreamProcessorReceiver<C, Coll>
    where
        F: AsyncWork<C> + Send + 'static,
        S: Stream + Send + 'static,
        S::Item: ReceiverEvent<T, C>,
        C: Future<Output = ()> + Send + 'static,
        Coll: Default + Send + 'static;

    type EmittingTask: crate::task::emit::EmittingTask<T, U, E, I>;
    fn run(self) -> Self::EmittingTask;
}

pub trait ReceiverBuilder<
    T: Clone + Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: crate::task::task_id::TaskId,
>: Sized
{
    type Task: crate::task::emit::EmittingTask<T, U, E, I>;
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
        workers: usize,  // default: num_cpus::get()
        rate_limit: f64, // default: 0.0 (no rate limit)
    },
    /// Process in batches
    Batched {
        batch_size: usize,   // default: 64
        max_delay: Duration, // default: Duration::from_millis(100)
    },
    /// Automatically adjust based on workload
    Adaptive {
        initial_capacity: usize,     // default: 64
        max_concurrency: usize,      // default: num_cpus::get()
        adaptation_window: Duration, // default: Duration::from_secs(1)
        use_rayon_for_cpu: bool,     // default: false
    },
}

#[derive(Debug, Clone)]
pub enum SenderStrategy {
    Serial {
        timeout_seconds: u64, // default: 0 (no timeout)
    },
    Parallel {
        workers: MinMax<usize>, // default: MinMax(1, num_cpus::get())
        rate_limit: f64,        // default: 0.0 (no rate limit)
    },
    Batched {
        batch_size: usize,   // default: 64
        max_delay: Duration, // default: Duration::from_millis(100)
    },
    Adaptive {
        initial_capacity: usize,     // default: 64
        max_concurrency: usize,      // default: num_cpus::get()
        adaptation_window: Duration, // default: Duration::from_secs(1)
        use_rayon_for_cpu: bool,     // default: false
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
