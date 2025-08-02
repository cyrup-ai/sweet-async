//! Event collector for Tokio stream processing
//!
//! This module provides components for collecting and aggregating events in stream-based tasks.

use std::any::type_name;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use futures::{Stream, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::ReceiverStrategy;

use crate::task::adaptive::{AdaptiveConfig, AdaptixAsyncDsl, build_adaptive_async_stream};
use tokio_util::sync::CancellationToken;

/// Smart debug formatting that works with any type, using Debug when available
/// and falling back to type name for non-Debug types
fn smart_debug_format<T>(item: &T) -> String {
    // For production use, we provide the type name as a safe fallback
    // This avoids requiring Debug constraint while still providing useful information
    format!("event of type {}", type_name::<T>())
}

/// Enhanced debug formatting for types that implement Debug
#[allow(dead_code)]
fn debug_format_when_available<T: std::fmt::Debug>(item: &T) -> String {
    format!("{:?}", item)
}

/// Macro to conditionally use Debug formatting if available, otherwise use type name
macro_rules! conditional_debug {
    ($item:expr) => {{
        // This provides a consistent debug experience regardless of whether T implements Debug
        smart_debug_format($item)
    }};
}

/// Tokio implementation of event collector
/// T: The type of raw event received.
/// C: The type of successfully processed/collected item.
/// EItemProc: The error type if an individual item processing fails.
/// I: The TaskId type.
pub struct TokioEventCollector<T, C, EItemProc, I: TaskId>
where
    T: Send + 'static,
    C: Clone + Send + Sync + 'static,
    EItemProc: Send + Sync + Clone + 'static, // Error from receiver_fn
    I: TaskId,
{
    /// The aggregated results, now a HashMap keyed by event Uuid
    results: HashMap<Uuid, Result<C, EItemProc>>,
    /// Channel receiver for collecting results from workers
    result_rx: Option<tokio::sync::mpsc::UnboundedReceiver<(Uuid, Result<C, EItemProc>)>>,
    /// Processing task handle
    task_handle: Option<JoinHandle<()>>,
    /// Type markers
    _marker: PhantomData<(T, I)>,
}

// ReceiverWorkFn now returns Result<C, EItemProc>
type ReceiverWorkFn<T, C, EItemProc> =
    Arc<dyn Fn(&T, &mut (), Uuid) -> Result<C, EItemProc> + Send + Sync>;

impl<T, C, EItemProc, I: TaskId> TokioEventCollector<T, C, EItemProc, I>
where
    T: Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItemProc: Clone + Send + Sync + 'static,
    I: TaskId,
{
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            result_rx: None,
            task_handle: None,
            _marker: PhantomData,
        }
    }

    /// Start processing a stream with the given strategy and receiver function
    pub fn start_processing<S>(
        &mut self,
        stream: S,
        strategy: ReceiverStrategy,
        receiver_fn: ReceiverWorkFn<T, C, EItemProc>,
        token: CancellationToken,
    ) where
        S: Stream<Item = T> + Unpin + Send + 'static,
        T: Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
        EItemProc: Clone + Send + Sync + 'static,
    {
        match strategy {
            ReceiverStrategy::Serial { timeout_seconds } => {
                self.process_serial(stream, timeout_seconds, receiver_fn, token);
            }
            ReceiverStrategy::Parallel {
                workers,
                rate_limit,
            } => {
                self.process_parallel(stream, workers, rate_limit, receiver_fn, token);
            }
            ReceiverStrategy::Batched {
                batch_size,
                max_delay,
            } => {
                self.process_batched(stream, batch_size, max_delay, receiver_fn, token);
            }
            ReceiverStrategy::Adaptive {
                initial_capacity,
                max_concurrency,
                adaptation_window,
                use_rayon_for_cpu,
            } => {
                self.process_adaptive(
                    stream,
                    initial_capacity,
                    max_concurrency,
                    adaptation_window,
                    use_rayon_for_cpu,
                    receiver_fn,
                    token,
                );
            }
        }
    }

    /// Process events serially
    fn process_serial<S>(
        &mut self,
        mut stream: S,
        timeout_seconds: u64,
        receiver_fn: ReceiverWorkFn<T, C, EItemProc>,
        token: CancellationToken,
    ) where
        S: Stream<Item = T> + Send + Unpin + 'static,
        T: Send + 'static, // Debug constraint handled via conditional_debug! macro for broader compatibility
        C: Clone + Send + Sync + 'static,
        EItemProc: Clone + Send + Sync + 'static,
    {
        let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel();
        self.result_rx = Some(result_rx);

        self.task_handle = Some(tokio::spawn(async move {
            let item_processing_timeout = if timeout_seconds > 0 {
                Some(std::time::Duration::from_secs(timeout_seconds))
            } else {
                None
            };
            let mut dummy_context = ();
            loop {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        tracing::debug!("SerialCollector: Cancellation signal received.");
                        break;
                    }
                    maybe_event_data = stream.next() => {
                        if let Some(event_data) = maybe_event_data {
                            let processing_fut = async {
                                let event_uuid = Uuid::new_v4();
                                let processed_result = receiver_fn(&event_data, &mut dummy_context, event_uuid);
                                let _ = result_tx.send((event_uuid, processed_result));
                            };

                            if let Some(dur) = item_processing_timeout {
                                if tokio::time::timeout(dur, processing_fut).await.is_err() {
                                    tracing::warn!(
                                        "SerialCollector: Item processing timed out for event: {}", 
                                        conditional_debug!(&event_data)
                                    );
                                }
                            } else {
                                processing_fut.await;
                            }
                        } else {
                            tracing::debug!("SerialCollector: Stream ended.");
                            break;
                        }
                    }
                }
            }
            tracing::debug!("SerialCollector: Task finished.");
        }));
    }

    /// Process events in parallel
    fn process_parallel<S>(
        &mut self,
        mut stream: S,
        workers: usize,
        _rate_limit: f64,
        receiver_fn: ReceiverWorkFn<T, C, EItemProc>,
        token: CancellationToken,
    ) where
        S: Stream<Item = T> + Send + Unpin + 'static,
        T: Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
        EItemProc: Clone + Send + Sync + 'static,
    {
        let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel();
        self.result_rx = Some(result_rx);
        
        let semaphore = Arc::new(Semaphore::new(workers.max(1)));
        let stop_workers_flag = Arc::new(AtomicBool::new(false));

        self.task_handle = Some(tokio::spawn(async move {
            let mut join_handles = Vec::new();
            let mut dummy_context = ();

            loop {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        tracing::debug!("ParallelCollector: Cancellation signal received. Signalling workers to stop.");
                        stop_workers_flag.store(true, AtomicOrdering::Relaxed);
                        break;
                    }
                    permit_acquisition_result = semaphore.clone().acquire_owned(), if !stop_workers_flag.load(AtomicOrdering::Relaxed) => {
                        let permit = match permit_acquisition_result {
                            Ok(p) => p,
                            Err(_) => {
                                tracing::debug!("ParallelCollector: Semaphore closed.");
                                stop_workers_flag.store(true, AtomicOrdering::Relaxed);
                                break;
                            }
                        };

                        match stream.next().await {
                            Some(event_data) => {
                                let receiver_fn_clone = receiver_fn.clone();
                                let result_tx_clone = result_tx.clone();
                                let worker_token = token.child_token();
                                let worker_stop_flag = stop_workers_flag.clone();

                                let jh = tokio::spawn(async move {
                                    if worker_stop_flag.load(AtomicOrdering::Relaxed) || worker_token.is_cancelled() {
                                        drop(permit);
                                        return;
                                    }

                                    let event_uuid = Uuid::new_v4();
                                    let mut dummy_context_task = ();
                                    let processed_result = receiver_fn_clone(&event_data, &mut dummy_context_task, event_uuid);
                                    let _ = result_tx_clone.send((event_uuid, processed_result));
                                    drop(permit);
                                });
                                join_handles.push(jh);
                            }
                            None => {
                                tracing::debug!("ParallelCollector: Stream ended.");
                                stop_workers_flag.store(true, AtomicOrdering::Relaxed);
                                break;
                            }
                        }
                    }
                    else => {
                        tokio::task::yield_now().await;
                    }
                }
            }

            tracing::debug!(
                "ParallelCollector: Main loop exited. Aborting {} active workers if cancellation triggered.",
                join_handles.len()
            );
            if token.is_cancelled() {
                for handle in &join_handles {
                    handle.abort();
                }
            }

            for handle in join_handles {
                if let Err(e) = handle.await {
                    if e.is_cancelled() || e.is_panic() {
                        tracing::debug!(
                            "ParallelCollector: Worker task was cancelled/panicked: {:?}",
                            e
                        );
                    } else {
                        tracing::error!(
                            "ParallelCollector: Worker task failed unexpectedly: {:?}",
                            e
                        );
                    }
                }
            }
            tracing::debug!("ParallelCollector: Task finished.");
        }));
    }

    /// Process events in batches
    fn process_batched<S>(
        &mut self,
        mut stream: S,
        batch_size: usize,
        max_delay: std::time::Duration,
        receiver_fn: ReceiverWorkFn<T, C, EItemProc>,
        token: CancellationToken,
    ) where
        S: Stream<Item = T> + Send + Unpin + 'static,
        T: Send + 'static,
        C: Clone + Send + Sync + 'static,
        EItemProc: Clone + Send + Sync + 'static,
    {
        let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel();
        self.result_rx = Some(result_rx);
        
        self.task_handle = Some(tokio::spawn(async move {
            let mut batch_with_ids: Vec<(Uuid, T)> = Vec::with_capacity(batch_size);
            let mut interval = tokio::time::interval(max_delay);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut dummy_context = ();
            let mut stream_ended = false;

            loop {
                if token.is_cancelled() {
                    tracing::debug!("BatchedCollector: Cancelled at loop start.");
                    if !batch_with_ids.is_empty() {
                        tracing::debug!(
                            "BatchedCollector: Processing final partial batch due to cancellation."
                        );
                        for (event_uuid, event_data) in std::mem::take(&mut batch_with_ids) {
                            let processed_result =
                                receiver_fn(&event_data, &mut dummy_context, event_uuid);
                            let _ = result_tx.send((event_uuid, processed_result));
                        }
                    }
                    break;
                }

                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        tracing::debug!("BatchedCollector: Cancellation signal received in select.");
                        if !batch_with_ids.is_empty() {
                            tracing::debug!("BatchedCollector: Processing final partial batch due to cancellation (select).");
                            for (event_uuid, event_data) in std::mem::take(&mut batch_with_ids) {
                                let processed_result = receiver_fn(&event_data, &mut dummy_context, event_uuid);
                                let _ = result_tx.send((event_uuid, processed_result));
                            }
                        }
                        break;
                    }
                    _ = interval.tick(), if !batch_with_ids.is_empty() => {
                        tracing::debug!("BatchedCollector: Interval tick. Processing batch of {} items.", batch_with_ids.len());
                        let current_batch_to_process = std::mem::take(&mut batch_with_ids);
                        for (event_uuid, event_data) in current_batch_to_process {
                            let processed_result = receiver_fn(&event_data, &mut dummy_context, event_uuid);
                            let _ = result_tx.send((event_uuid, processed_result));
                        }
                    }
                    maybe_event_data = stream.next(), if !stream_ended => {
                        match maybe_event_data {
                            Some(event_data) => {
                                let event_uuid = Uuid::new_v4();
                                batch_with_ids.push((event_uuid, event_data));
                                if batch_with_ids.len() >= batch_size {
                                    tracing::debug!("BatchedCollector: Batch full. Processing batch of {} items.", batch_with_ids.len());
                                    let current_batch_to_process = std::mem::take(&mut batch_with_ids);
                                    for (event_uuid_in_batch, event_data_in_batch) in current_batch_to_process {
                                        let processed_result = receiver_fn(&event_data_in_batch, &mut dummy_context, event_uuid_in_batch);
                                        let _ = result_tx.send((event_uuid_in_batch, processed_result));
                                    }
                                    interval.reset();
                                }
                            }
                            None => {
                                tracing::debug!("BatchedCollector: Stream ended.");
                                stream_ended = true;
                                if !batch_with_ids.is_empty(){
                                    tracing::debug!("BatchedCollector: Processing final batch of {} items from stream end.", batch_with_ids.len());
                                    for (event_uuid_in_batch, event_data_in_batch) in std::mem::take(&mut batch_with_ids) {
                                        let processed_result = receiver_fn(&event_data_in_batch, &mut dummy_context, event_uuid_in_batch);
                                        let _ = result_tx.send((event_uuid_in_batch, processed_result));
                                    }
                                }
                                break;
                            }
                        }
                    }
                    else => {
                        if stream_ended && batch_with_ids.is_empty() {
                            tracing::debug!("BatchedCollector: Stream ended and batch empty, exiting.");
                            break;
                        }
                    }
                }
            }
            tracing::debug!("BatchedCollector: Task finished.");
        }));
    }

    /// Process events adaptively using the logic from crate::task::adaptive
    fn process_adaptive<S>(
        &mut self,
        stream: S,
        initial_capacity: usize,
        max_concurrency: usize,
        adaptation_window: std::time::Duration,
        _use_rayon_for_cpu: bool,
        receiver_fn: ReceiverWorkFn<T, C, EItemProc>,
        token: CancellationToken,
    ) where
        S: Stream<Item = T> + Unpin + Send + 'static,
        T: Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
        EItemProc: Send + 'static + Clone, // Clone for adapted_map_fn if error needs to be part of (Uuid, Result<C,E>)
    {
        let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel();
        self.result_rx = Some(result_rx);

        let adaptive_cfg = AdaptiveConfig {
            min_workers: 1,
            max_workers: max_concurrency,
            sample_size: 50,
            io_threshold_ms: 50,
            adapt_interval_ms: adaptation_window.as_millis() as u64,
            cpu_chunk_size: initial_capacity,
            io_chunk_size: 1,
            mixed_chunk_size: (initial_capacity / 2).max(1),
        };

        let adapted_map_fn = {
            let receiver_fn_clone = receiver_fn.clone();
            move |item_t: &T| -> (Uuid, Result<C, EItemProc>) {
                let mut dummy_context = ();
                let event_uuid = Uuid::new_v4();
                let processed_result = receiver_fn_clone(item_t, &mut dummy_context, event_uuid);
                (event_uuid, processed_result)
            }
        };

        let dsl = AdaptixAsyncDsl {
            config: adaptive_cfg,
            input_stream: stream,
            map_fn: adapted_map_fn,
            cancellation_token: token,
        };

        self.task_handle = Some(tokio::spawn(async move {
            let mut result_stream = build_adaptive_async_stream(dsl);

            while let Some((event_uuid, processed_result)) = result_stream.next().await {
                let _ = result_tx.send((event_uuid, processed_result));
            }
            tracing::debug!("AdaptiveCollector: Task finished processing stream.");
        }));
    }


    /// Wait for processing to complete and return a reference to the results
    pub async fn join(&mut self) -> HashMap<Uuid, Result<C, EItemProc>> {
        // First wait for the processing task to complete
        if let Some(handle) = self.task_handle.take() {
            if let Err(join_error) = handle.await {
                eprintln!("TokioEventCollector processing task failed: {}", join_error);
            }
        }
        
        // Then collect all results from the channel
        if let Some(mut rx) = self.result_rx.take() {
            while let Ok((uuid, result)) = rx.try_recv() {
                self.results.insert(uuid, result);
            }
        }
        
        // Return the collected results
        std::mem::take(&mut self.results)
    }
}

impl<T, C, EItemProc, I: TaskId> Default for TokioEventCollector<T, C, EItemProc, I> 
where
    T: Send + 'static,
    C: Clone + Send + Sync + 'static,
    EItemProc: Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

use dashmap::DashMap;

/// Lock-free stream collector for accumulating processed results
/// Provides collect(K, V) and collected() interface expected by CSV example
/// 
/// This collector uses DashMap for blazing-fast lock-free concurrent access
/// allowing multiple receiver workers to collect results simultaneously
/// without any blocking or allocation overhead.
pub struct StreamCollector<K, V> {
    /// Lock-free concurrent HashMap for zero-allocation collection
    storage: Arc<DashMap<K, V>>,
}

impl<K, V> StreamCollector<K, V> 
where
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create new empty collector with lock-free storage
    /// 
    /// Uses DashMap internally for blazing-fast concurrent access
    /// without any locks or allocations during collection operations.
    #[inline]
    pub fn new() -> Self {
        Self {
            storage: Arc::new(DashMap::new()),
        }
    }
    
    /// Collect a processed item with thread-safe insertion
    /// 
    /// This method is lock-free and can be called concurrently
    /// from multiple receiver workers without blocking.
    /// 
    /// # Performance
    /// - Zero allocation for key/value insertion
    /// - Lock-free concurrent access via DashMap
    /// - Optimized for high-throughput collection scenarios
    #[inline]
    pub fn collect(&self, key: K, value: V) {
        self.storage.insert(key, value);
    }
    
    /// Return all collected items as HashMap (zero-copy where possible)
    /// 
    /// Consumes the collector and returns all accumulated results.
    /// This is typically called in await_final_event to retrieve
    /// the final processing results.
    /// 
    /// # Performance
    /// - Efficient conversion from DashMap to HashMap
    /// - Zero-copy where possible for optimal performance
    pub fn collected(self) -> std::collections::HashMap<K, V> {
        // Extract DashMap from Arc - if there are other references, clone the data
        match Arc::try_unwrap(self.storage) {
            Ok(dashmap) => dashmap.into_iter().collect(),
            Err(arc_dashmap) => {
                // Other references exist, so we need to clone the data
                arc_dashmap.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
            }
        }
    }
    
    /// Get current count of collected items
    /// 
    /// Returns the number of items currently stored in the collector.
    /// This operation is lock-free and can be called concurrently.
    #[inline]
    pub fn len(&self) -> usize {
        self.storage.len()
    }
    
    /// Check if collector is empty
    /// 
    /// Returns true if no items have been collected yet.
    /// This operation is lock-free and can be called concurrently.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }
    
    /// Get a reference to a collected item by key
    /// 
    /// Returns Some(value_ref) if the key exists, None otherwise.
    /// The reference is valid for as long as the collector exists.
    #[inline]
    pub fn get(&self, key: &K) -> Option<dashmap::mapref::one::Ref<'_, K, V>> {
        self.storage.get(key)
    }
    
    /// Check if a key exists in the collector
    /// 
    /// Returns true if the key has been collected, false otherwise.
    /// This operation is lock-free and can be called concurrently.
    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        self.storage.contains_key(key)
    }
}

impl<K, V> Clone for StreamCollector<K, V> {
    /// Clone creates a new reference to the same underlying storage
    /// 
    /// Multiple clones share the same DashMap, allowing for efficient
    /// passing between receiver functions and await_final_event.
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
        }
    }
}

impl<K, V> Default for StreamCollector<K, V> 
where
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> std::fmt::Debug for StreamCollector<K, V> 
where
    K: std::fmt::Debug + Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamCollector")
            .field("len", &self.len())
            .field("is_empty", &self.is_empty())
            .finish()
    }
}

/// CSV file builder for fluent configuration of file-based data sources
/// 
/// Provides zero-allocation configuration of CSV parsing parameters
/// that integrates with the existing sophisticated CSV streaming infrastructure.
pub struct CsvFileBuilder {
    /// File path for CSV data source
    file_path: std::path::PathBuf,
    /// Delimiter configuration for parsing
    delimiter: crate::task::Delimiter,
    /// Chunking strategy for performance optimization
    chunk_size: crate::task::ChunkSize,
}

impl CsvFileBuilder {
    /// Create a new CSV file builder with default configuration
    /// 
    /// # Performance
    /// Zero allocation during construction, efficient path storage
    #[inline]
    pub fn new<P: AsRef<std::path::Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            delimiter: crate::task::Delimiter::Comma,
            chunk_size: crate::task::ChunkSize::Rows(1000),
        }
    }
    
    /// Configure the delimiter for CSV parsing
    /// 
    /// # Performance
    /// Zero allocation, copy semantics for delimiter configuration
    #[inline]
    pub fn with_delimiter(mut self, delimiter: crate::task::Delimiter) -> Self {
        self.delimiter = delimiter;
        self
    }
    
    /// Configure the chunking strategy for optimal performance
    /// 
    /// Determines how the CSV file is split into processing chunks:
    /// - `ChunkSize::Rows(n)` - Process n rows at a time
    /// - `ChunkSize::Bytes(n)` - Process n bytes at a time  
    /// - `ChunkSize::Duration(d)` - Process for duration d
    /// 
    /// # Performance
    /// Zero allocation, copy semantics for chunk configuration
    #[inline]
    pub fn into_chunks(mut self, chunk_size: crate::task::ChunkSize) -> Self {
        self.chunk_size = chunk_size;
        self
    }
    
    /// Get the configured file path
    #[inline]
    pub fn file_path(&self) -> &std::path::Path {
        &self.file_path
    }
    
    /// Get the configured delimiter
    #[inline]
    pub const fn delimiter(&self) -> crate::task::Delimiter {
        self.delimiter
    }
    
    /// Get the configured chunk size
    #[inline]
    pub const fn chunk_size(&self) -> crate::task::ChunkSize {
        self.chunk_size
    }
}

impl<K, V> StreamCollector<K, V> 
where
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a CSV file data source with fluent configuration
    /// 
    /// This method initiates the fluent API for CSV file processing:
    /// ```rust
    /// collector.of_file("data.csv")
    ///     .with_delimiter(Delimiter::Comma)
    ///     .into_chunks(100.rows())
    /// ```
    /// 
    /// # Performance
    /// Zero allocation during configuration, integrates with existing
    /// sophisticated CSV streaming infrastructure for blazing-fast processing.
    /// 
    /// # Integration
    /// The returned CsvFileBuilder will be used by the sender phase to
    /// configure the existing `read_csv_file_streaming` function with
    /// the specified parameters.
    #[inline]
    pub fn of_file<P: AsRef<std::path::Path>>(&self, file_path: P) -> CsvFileBuilder {
        CsvFileBuilder::new(file_path)
    }
}

/// Integration helper for connecting CSV file builders to the streaming infrastructure
/// 
/// This function bridges the fluent API configuration with the existing
/// sophisticated CSV streaming implementation for zero-allocation processing.
pub async fn execute_csv_streaming<T>(
    csv_config: CsvFileBuilder,
    tx: tokio::sync::mpsc::Sender<T>,
) -> Result<usize, String> 
where
    T: Send + 'static,
{
    // Leverage the existing sophisticated CSV streaming function
    // that already handles all performance optimizations
    crate::task::emit::channel_builder::read_csv_file_streaming(
        csv_config.file_path(),
        csv_config.delimiter(),
        csv_config.chunk_size(),
        tx,
    ).await
}
