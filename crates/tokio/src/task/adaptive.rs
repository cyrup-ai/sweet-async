//! Adaptive concurrency system that automatically selects between:
//!   - CPU-bound mode -> Uses Rayon + chunking
//!   - IO-bound mode  -> Uses Tokio one-by-one tasks
//!   - Mixed mode     -> Uses Tokio, moderate chunk size
//!
//! We hide all async behind normal methods and a `Stream` so that
//! the user doesn't need to deal with `async_trait` or pinned futures.

use futures::{Stream, StreamExt};
use num_cpus;
use rayon::ThreadPool;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

// =============== Workload Classification ===============
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkloadType {
    CpuBound,
    IoBound,
    Mixed,
}

// =============== Adaptix Configuration ===============
#[derive(Debug, Clone)]
pub struct AdaptixConfig {
    pub min_workers: usize,
    pub max_workers: usize,
    pub sample_size: usize,
    pub io_threshold_ms: u64,
    pub adapt_interval_ms: u64,

    /// When CPU-bound, how many items to process in a single chunk job?
    pub cpu_chunk_size: usize,

    /// When IO-bound, how many items to process in a single chunk job?
    /// Usually 1, but you could tweak if you want some micro-batching.
    pub io_chunk_size: usize,

    /// For "Mixed" workloads, chunk size can be somewhere in between.
    pub mixed_chunk_size: usize,
}

impl Default for AdaptixConfig {
    fn default() -> Self {
        let cpus = num_cpus::get().max(1);
        Self {
            min_workers: 1,
            max_workers: cpus.saturating_mul(2),
            sample_size: 50,
            io_threshold_ms: 50,
            adapt_interval_ms: 1000,
            cpu_chunk_size: 64,
            io_chunk_size: 1,
            mixed_chunk_size: 16,
        }
    }
}

// =============== ConcurrencyEngine Enum ===============
//  - Either use Rayon or use Tokio.
//  - We'll embed a thread pool for Rayon for CPU-bound tasks.

#[derive(Clone)]
enum ConcurrencyEngine {
    Rayon { pool: Arc<ThreadPool> },
    Tokio,
}

impl ConcurrencyEngine {
    /// Build a new Rayon engine with `threads` number of threads.
    fn new_rayon(threads: usize) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("Failed to build Rayon ThreadPool");
        ConcurrencyEngine::Rayon {
            pool: Arc::new(pool),
        }
    }

    /// Build a new Tokio engine (just an enum variant for identification).
    fn new_tokio() -> Self {
        ConcurrencyEngine::Tokio
    }

    /// Spawn a "chunk job" returning a Vec of outputs. We unify the interface
    /// so that CPU-bound uses parallel iteration and IO-bound uses normal iteration.
    /// We still submit the entire chunk to either the Rayon pool or a Tokio task.
    fn spawn_chunk<Item, Out, F>(
        &self,
        chunk: Vec<Item>,
        map_fn: Arc<F>,
        cpu_bound: bool,
    ) -> EngineTask<Vec<Out>>
    where
        Item: Send + Sync + 'static,
        Out: Send + 'static,
        F: Fn(&Item) -> Out + Send + Sync + 'static,
    {
        match self {
            ConcurrencyEngine::Rayon { pool } if cpu_bound => {
                // CPU-bound => parallel map using rayon
                let (tx, rx) = mpsc::channel(1);
                let pool_clone = Arc::clone(pool);
                pool_clone.spawn_fifo(move || {
                    let result: Vec<Out> = chunk.iter().map(|item| map_fn(item)).collect();
                    let _ = tx.blocking_send(result);
                });
                EngineTask::Rayon(rx)
            }
            ConcurrencyEngine::Rayon { pool } => {
                // If we ended up here but it's not "strictly" CPU-bound, we can still do parallel
                // or we could do a normal serial iteration.
                // We'll just do parallel anyway for brevity.
                let (tx, rx) = mpsc::channel(1);
                let pool_clone = Arc::clone(pool);
                pool_clone.spawn_fifo(move || {
                    let result: Vec<Out> = chunk.iter().map(|item| map_fn(item)).collect();
                    let _ = tx.blocking_send(result);
                });
                EngineTask::Rayon(rx)
            }
            ConcurrencyEngine::Tokio => {
                // IO-bound => spawn an async job that processes the chunk in normal iteration.
                let (tx, rx) = mpsc::channel(1);
                tokio::spawn(async move {
                    let result: Vec<Out> = chunk.into_iter().map(|item| map_fn(&item)).collect();
                    if let Err(e) = tx.send(result).await {
                        warn!("Tokio chunk job failed to send: {e}");
                    }
                });
                EngineTask::Tokio(rx) // Use appropriate variant - was incorrectly using Rayon
            }
        }
    }
}

/// A unified "handle" for a spawned chunk job from either engine.
enum EngineTask<T> {
    /// For Rayon, we send the result through an mpsc channel.
    Rayon(mpsc::Receiver<T>),
    /// For Tokio, we send the result through an mpsc channel.
    Tokio(mpsc::Receiver<T>),
}

impl<T: Send + 'static> EngineTask<T> {
    /// Wait for the chunk result asynchronously.
    pub async fn wait(self) -> Result<T, String> {
        match self {
            EngineTask::Rayon(mut rx) => rx
                .recv()
                .await
                .ok_or_else(|| "Rayon EngineTask: Channel closed unexpectedly".to_string()),
            EngineTask::Tokio(mut rx) => rx
                .recv()
                .await
                .ok_or_else(|| "Tokio EngineTask: Channel closed unexpectedly".to_string()),
        }
    }
}

// =============== Stats & Adaptation ===============

struct AdaptixStats {
    task_durations: Vec<Duration>,
    last_adapt: Instant,
    current_workers: usize, // Target number of workers
    workload_type: WorkloadType,
    engine: ConcurrencyEngine,
    chunk_size: usize,
    concurrency_sema: Arc<Semaphore>, // Moved Semaphore here
}

impl AdaptixStats {
    fn new(initial_workers: usize, initial_chunk_size: usize) -> Self {
        // Accept initial config
        let cpus = num_cpus::get().max(1);
        Self {
            task_durations: Vec::new(),
            last_adapt: Instant::now(),
            current_workers: initial_workers.max(1),
            workload_type: WorkloadType::Mixed, // Start with mixed
            engine: ConcurrencyEngine::new_tokio(), // Default to Tokio engine
            chunk_size: initial_chunk_size.max(1),
            concurrency_sema: Arc::new(Semaphore::new(initial_workers.max(1))),
        }
    }

    fn engine_clone(&self) -> ConcurrencyEngine {
        match &self.engine {
            ConcurrencyEngine::Rayon { pool } => ConcurrencyEngine::Rayon {
                pool: Arc::clone(pool),
            },
            ConcurrencyEngine::Tokio => ConcurrencyEngine::Tokio,
        }
    }

    // Method to update the semaphore when current_workers changes
    fn update_semaphore(&mut self) {
        // Create a new semaphore with the updated number of workers.
        // Existing tasks holding permits from the old semaphore will complete normally.
        // New tasks will acquire permits from this new semaphore.
        self.concurrency_sema = Arc::new(Semaphore::new(self.current_workers.max(1)));
        debug!("Semaphore updated to {} permits", self.current_workers);
    }
}

/// Decide if we should switch between CPU-bound (Rayon) or IO-bound (Tokio),
/// possibly adjusting concurrency and chunk sizes for the next batch of chunks.
fn adapt_concurrency(st: &mut AdaptixStats, cfg: &AdaptixConfig) {
    let total: Duration = st.task_durations.iter().sum();
    let count = st.task_durations.len();
    if count == 0 {
        return;
    }
    let avg = total / (count as u32);
    let io_like = st
        .task_durations
        .iter()
        .filter(|d| d.as_millis() > cfg.io_threshold_ms as u128)
        .count();
    let ratio = (io_like as f64) / (count as f64);

    let new_type = if ratio > 0.8 {
        WorkloadType::IoBound
    } else if ratio < 0.2 {
        WorkloadType::CpuBound
    } else {
        WorkloadType::Mixed
    };
    st.workload_type = new_type;

    let cpus = num_cpus::get().max(1);
    let prev_workers = st.current_workers;
    match new_type {
        WorkloadType::CpuBound => {
            st.engine = ConcurrencyEngine::new_rayon(cpus);
            st.current_workers = cpus.clamp(cfg.min_workers, cfg.max_workers);
            st.chunk_size = cfg.cpu_chunk_size;
        }
        WorkloadType::IoBound => {
            st.engine = ConcurrencyEngine::new_tokio();
            st.current_workers = (cpus * 2).clamp(cfg.min_workers, cfg.max_workers);
            st.chunk_size = cfg.io_chunk_size;
        }
        WorkloadType::Mixed => {
            st.engine = ConcurrencyEngine::new_tokio();
            st.current_workers = ((cpus * 3) / 2).clamp(cfg.min_workers, cfg.max_workers);
            st.chunk_size = cfg.mixed_chunk_size;
        }
    }

    if st.current_workers != prev_workers {
        st.update_semaphore(); // Update semaphore if worker count changed
    }

    st.task_durations.clear();
    st.last_adapt = Instant::now();
    debug!(
        "Adapt concurrency -> new_mode={:?}, chunk_size={}, concurrency={}, avg_chunk_time={:?}",
        new_type, st.chunk_size, st.current_workers, avg
    );
}

// =============== The DSL and build_adaptix_stream ===============

/// DSL inputs for building a parallel (or chunked) stream.
pub struct AdaptixDsl<I, F> {
    pub config: AdaptixConfig,
    pub items: I,
    pub map_fn: F,
}

/// Build the adaptive chunked stream. The returned `impl Stream<Item=Out>`
/// yields items in the same order that chunk tasks complete. If chunk tasks
/// complete out of order, items could appear out of order—**this example**
/// just forwards chunk results as soon as they're done.
/// You can make it more complex to preserve original ordering if needed.
pub fn build_adaptix_stream<It, Item, MapFn, Out>(
    dsl: AdaptixDsl<It, MapFn>,
) -> impl Stream<Item = Out>
where
    // We accept any IntoIterator for the input
    It: IntoIterator<Item = Item> + Send + 'static,
    <It as IntoIterator>::IntoIter: Send,
    Item: Send + Sync + 'static,
    // The map_fn is a standard Fn(&Item)->Out (sync).
    MapFn: Fn(&Item) -> Out + Send + Sync + 'static,
    Out: Send + 'static,
{
    let AdaptixDsl {
        config,
        items,
        map_fn,
    } = dsl;

    let (tx_out, rx_out) = mpsc::channel::<Out>(config.max_workers.saturating_mul(2));
    let map_fn = Arc::new(map_fn);

    // We'll spawn a single aggregator that:
    //   1) Takes items from the iterator
    //   2) Batches them into "chunk_size"
    //   3) Spawns chunk jobs on either Rayon or Tokio
    //   4) Limits concurrency with a semaphore
    //   5) Measures time, updates stats, possibly calls adapt_concurrency

    tokio::spawn(async move {
        let mut stats = AdaptixStats::new(config.max_workers, config.cpu_chunk_size);
        let concurrency_sema = Arc::new(Semaphore::new(stats.current_workers));

        // We store items in a local Vec until we reach chunk_size, then spawn a job.
        let mut chunk_buf = Vec::with_capacity(stats.chunk_size);

        // We'll turn the input iterator into an explicit list so we can .into_iter().
        // If we had a streaming input, we'd adapt differently.
        let mut input_iter = items.into_iter();

        // Helper closure for spawning chunk jobs
        let spawn_chunk_job = |chunk: Vec<Item>, st: &mut AdaptixStats| {
            if chunk.is_empty() {
                return None;
            }

            // Acquire concurrency permit
            let sema_cl = Arc::clone(&concurrency_sema);
            let map_cl = Arc::clone(&map_fn);
            let engine_clone = st.engine_clone();

            let start = Instant::now();
            let cpu_bound = matches!(st.workload_type, WorkloadType::CpuBound);

            // Spawn the chunk
            let task = engine_clone.spawn_chunk(chunk, map_cl, cpu_bound);

            // Now we create an async block that awaits the chunk's completion,
            // then updates stats, possibly adapts concurrency, and sends outputs.
            let join_handle = tokio::spawn(async move {
                // Wait for permit
                let _permit = sema_cl.acquire_owned().await.unwrap();

                let result_future = task.wait();
                let outputs = match result_future.await {
                    Ok(o) => o,
                    Err(e) => {
                        warn!("Chunk job failed: {e}");
                        Vec::new()
                    }
                };

                // Return timing and outputs
                (outputs, start.elapsed())
            });

            Some(join_handle)
        };

        // We'll keep a small list of in-flight chunk job handles.
        // When each finishes, we push its results into tx_out.
        // Then we might adapt concurrency if needed.
        let mut in_flight = Vec::new();

        // PROCESS INPUT
        loop {
            // If we need to fetch the next item (non-blocking in a real scenario),
            // do so. If we have no more input, we break out.
            let next_item = input_iter.next();
            match next_item {
                Some(item) => {
                    chunk_buf.push(item);
                    // If we have a full chunk, spawn a chunk job
                    if chunk_buf.len() >= stats.chunk_size {
                        let chunk =
                            std::mem::replace(&mut chunk_buf, Vec::with_capacity(stats.chunk_size));
                        if let Some(fut) = spawn_chunk_job(chunk, &mut stats) {
                            in_flight.push(fut);
                        }
                    }
                }
                None => {
                    // End of input
                    if !chunk_buf.is_empty() {
                        let chunk = std::mem::take(&mut chunk_buf);
                        if let Some(fut) = spawn_chunk_job(chunk, &mut stats) {
                            in_flight.push(fut);
                        }
                    }
                    break;
                }
            }
        }

        // Now we have in_flight tasks that eventually produce (Vec<Out>, Duration).
        // We drain them in the order they complete.
        for handle in in_flight {
            match handle.await {
                Ok((outputs, duration)) => {
                    // Update stats
                    stats.task_durations.push(duration);
                    // Send each item
                    for out in outputs {
                        if tx_out.send(out).await.is_err() {
                            // receiver closed
                            break;
                        }
                    }

                    // Possibly adapt concurrency
                    if stats.task_durations.len() >= config.sample_size
                        && stats.last_adapt.elapsed().as_millis() > config.adapt_interval_ms as u128
                    {
                        adapt_concurrency(&mut stats, &config);
                        // Update the concurrency semaphore size
                        concurrency_sema.close();
                        concurrency_sema.add_permits(
                            stats
                                .current_workers
                                .saturating_sub(concurrency_sema.available_permits()),
                        );
                    }
                }
                Err(e) => {
                    warn!("Chunk aggregator join error: {e}");
                }
            }
        }

        // Done sending
        drop(tx_out);
    });

    // The aggregator runs in the background. We return a `ReceiverStream`
    // that yields all the items the aggregator produces.
    ReceiverStream::new(rx_out)
}

// =============== NEW: Async Stream Adaptive Processing ===============

/// DSL inputs for building an adaptive stream from an ASYNC input stream.
pub struct AdaptixAsyncDsl<S, F> {
    pub config: AdaptixConfig,
    pub input_stream: S,
    pub map_fn: F,
    pub cancellation_token: CancellationToken,
}

/// Build an adaptive chunked stream from an ASYNC input stream.
/// The returned `impl Stream<Item=Out>` yields items.
/// Order depends on chunk completion; this version forwards results as they complete.
pub fn build_adaptive_async_stream<S, Item, MapFn, Out>(
    dsl: AdaptixAsyncDsl<S, MapFn>,
) -> impl Stream<Item = Out>
where
    S: Stream<Item = Item> + Unpin + Send + 'static,
    Item: Send + Sync + 'static,
    MapFn: Fn(&Item) -> Out + Send + Sync + 'static,
    Out: Send + 'static,
{
    let AdaptixAsyncDsl {
        config,
        mut input_stream,
        map_fn,
        cancellation_token,
    } = dsl;

    let (tx_out, rx_out) = mpsc::channel::<Out>(config.max_workers.saturating_mul(2));
    let map_fn_arc = Arc::new(map_fn);

    tokio::spawn(async move {
        let mut stats = AdaptixStats::new(config.max_workers, config.cpu_chunk_size);
        let concurrency_sema = Arc::new(Semaphore::new(stats.current_workers.max(1)));

        let mut chunk_buf = Vec::with_capacity(stats.chunk_size);

        // Helper closure for spawning chunk jobs
        let spawn_chunk_job = |chunk: Vec<Item>, st: &AdaptixStats, map_fn_cloned: Arc<MapFn>| {
            if chunk.is_empty() {
                return None;
            }
            let permit_sema = st.concurrency_sema.clone();
            let engine = st.engine_clone();
            let start_time = Instant::now();
            let cpu_bound_hint = matches!(st.workload_type, WorkloadType::CpuBound);
            let chunk_processing_engine_task =
                engine.spawn_chunk(chunk, map_fn_cloned, cpu_bound_hint);

            let join_handle: JoinHandle<Result<(Vec<Out>, Duration), tokio::task::JoinError>> =
                tokio::spawn(async move {
                    let _permit = permit_sema
                        .acquire_owned()
                        .await
                        .map_err(|e| format!("Semaphore acquire error: {}", e))
                        .expect("Semaphore closed unexpectedly");
                    match chunk_processing_engine_task.wait().await {
                        Ok(outputs) => Ok((outputs, start_time.elapsed())),
                        Err(e) => {
                            warn!("Adaptive async stream: Chunk job processing failed: {}", e);
                            Ok((Vec::new(), start_time.elapsed()))
                        }
                    }
                });
            Some(join_handle)
        };

        let mut in_flight_chunk_futures: Vec<
            JoinHandle<Result<(Vec<Out>, Duration), tokio::task::JoinError>>,
        > = Vec::new();
        let mut item_received_in_batch_or_stream_active = true;

        while item_received_in_batch_or_stream_active {
            if cancellation_token.is_cancelled() {
                tracing::debug!(
                    "AdaptiveStream: Cancellation detected at loop start. Aborting in-flight tasks."
                );
                break;
            }

            let mut current_chunk_items_count = 0;
            while chunk_buf.len() < stats.chunk_size {
                tokio::select! {
                    biased;
                    _ = cancellation_token.cancelled() => {
                        tracing::debug!("AdaptiveStream: Cancellation detected while filling chunk.");
                        item_received_in_batch_or_stream_active = false;
                        break;
                    }
                    maybe_item = input_stream.next() => {
                        if let Some(item) = maybe_item {
                            chunk_buf.push(item);
                            current_chunk_items_count +=1;
                        } else {
                            item_received_in_batch_or_stream_active = false;
                            break;
                        }
                    }
                }
                if !item_received_in_batch_or_stream_active {
                    break;
                }
            }

            if !chunk_buf.is_empty() {
                let current_chunk_to_process =
                    std::mem::replace(&mut chunk_buf, Vec::with_capacity(stats.chunk_size));
                if let Some(fut) =
                    spawn_chunk_job(current_chunk_to_process, &stats, map_fn_arc.clone())
                {
                    in_flight_chunk_futures.push(fut);
                }
            }

            if !item_received_in_batch_or_stream_active && chunk_buf.is_empty() {
                break;
            }
        }

        if cancellation_token.is_cancelled() {
            tracing::debug!(
                "AdaptiveStream: Aborting {} in-flight chunk futures due to cancellation.",
                in_flight_chunk_futures.len()
            );
            for fut_handle in &in_flight_chunk_futures {
                fut_handle.abort();
            }
        }

        for chunk_future_handle in in_flight_chunk_futures {
            match chunk_future_handle.await {
                Ok(Ok((outputs, duration))) => {
                    stats.task_durations.push(duration);
                    for out_item in outputs {
                        if tx_out.send(out_item).await.is_err() {
                            warn!("Adaptive async stream: Output channel closed.");
                            drop(tx_out);
                            return;
                        }
                    }
                    if !cancellation_token.is_cancelled()
                        && stats.task_durations.len() >= config.sample_size
                        && stats.last_adapt.elapsed().as_millis() as u64 >= config.adapt_interval_ms
                    {
                        adapt_concurrency(&mut stats, &config);
                    }
                }
                Ok(Err(_)) => { /* This case should not be hit if spawn_chunk_job always returns Ok((Vec, Duration)) */
                }
                Err(join_error) => {
                    if join_error.is_cancelled() {
                        tracing::debug!(
                            "AdaptiveStream: In-flight chunk future was cancelled (aborted)."
                        );
                    } else {
                        warn!(
                            "AdaptiveStream: In-flight chunk future join error (panic): {}",
                            join_error
                        );
                    }
                }
            }
        }
        drop(tx_out);
        tracing::debug!("AdaptiveStream: Aggregator task finished.");
    });

    ReceiverStream::new(rx_out)
}

// =============== DSL Macros (optional) ===============

#[macro_export]
macro_rules! adaptix {
    ( $($body:tt)* ) => {
        $crate::adaptix_parse!( $($body)* )
    };
}

#[macro_export]
macro_rules! adaptix_parse {
    (
        config {
            $($cfg_key:ident = $cfg_val:expr),* $(,)?
        },
        items($items:expr),
        map $map_expr:tt
    ) => {{
        let mut cfg = $crate::AdaptixConfig::default();
        $(
            cfg.$cfg_key = $cfg_val;
        )*
        fn local_map() -> impl Fn(&_) -> _ + Copy + Send + Sync + 'static {
            $crate::adaptix_map! $map_expr
        }
        $crate::build_adaptix_stream($crate::AdaptixDsl {
            config: cfg,
            items: $items,
            map_fn: local_map(),
        })
    }};
}

#[macro_export]
macro_rules! adaptix_map {
    (|$val:ident| { $($body:tt)* }) => {
        move |$val| {
            $($body)*
        }
    };
    // or if you want an async block, you can do something else, but
    // in CPU-bound we typically won't do async.
}

/// Export adaptive config as AdaptiveConfig for API compatibility
pub use AdaptixConfig as AdaptiveConfig;

/// Export build_adaptix_stream with a more consistent API name
pub fn adaptive_stream<It, Item, MapFn, Out>(
    items: It,
    map_fn: MapFn,
    config: AdaptiveConfig,
) -> impl Stream<Item = Out>
where
    It: IntoIterator<Item = Item> + Send + 'static,
    <It as IntoIterator>::IntoIter: Send,
    Item: Send + Sync + 'static,
    MapFn: Fn(&Item) -> Out + Send + Sync + 'static,
    Out: Send + 'static,
{
    build_adaptix_stream(AdaptixDsl {
        config,
        items,
        map_fn,
    })
}

/// Process a stream using adaptive concurrency
pub async fn process_adaptive<It, Item, F, Out>(
    items: It,
    map_fn: F,
    config: Option<AdaptiveConfig>,
) -> Vec<Out>
where
    It: IntoIterator<Item = Item> + Send + 'static,
    <It as IntoIterator>::IntoIter: Send,
    Item: Send + Sync + 'static,
    F: Fn(&Item) -> Out + Send + Sync + 'static,
    Out: Send + 'static,
{
    use futures::StreamExt;

    let cfg = config.unwrap_or_default();
    let stream = adaptive_stream(items, map_fn, cfg);

    stream.collect().await
}
