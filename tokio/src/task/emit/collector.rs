//! Event collector for Tokio stream processing
//!
//! This module provides components for collecting and aggregating events in stream-based tasks.

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

/// Tokio implementation of event collector
/// T: The type of raw event received.
/// C: The type of successfully processed/collected item.
/// EItemProc: The error type if an individual item processing fails.
/// I: The TaskId type.
pub struct TokioEventCollector<T, C, EItemProc, I: TaskId>
where
    T: Send + 'static,
    C: Clone + Send + Sync + 'static,
    EItemProc: Send + 'static, // Error from receiver_fn
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
    EItemProc: Send + 'static,
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
        EItemProc: Send + 'static,
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
        T: Send + 'static, // Debug constraint removed for now for broader T compatibility
        C: Clone + Send + Sync + 'static,
        EItemProc: Send + 'static,
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
                                    tracing::warn!("SerialCollector: Item processing timed out for event");
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
        EItemProc: Send + 'static,
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
        EItemProc: Send + 'static,
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
    EItemProc: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
