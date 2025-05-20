//! Event collector for Tokio stream processing
//!
//! This module provides components for collecting and aggregating events in stream-based tasks.

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use std::collections::HashMap;

use futures::{Stream, StreamExt};
use tokio::task::JoinHandle;
use tokio::sync::Semaphore;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::ReceiverStrategy;

use crate::task::adaptive::{build_adaptive_async_stream, AdaptixAsyncDsl, AdaptiveConfig};

/// Tokio implementation of event collector
pub struct TokioEventCollector<T: Send + 'static, C: Clone + Send + 'static, I: TaskId> {
    /// The aggregated results, now a HashMap keyed by event Uuid
    results: Arc<Mutex<HashMap<Uuid, C>>>,
    /// Processing task handle
    task_handle: Option<JoinHandle<()>>,
    /// Type markers
    _marker: PhantomData<(T, I)>,
}

// Define a type alias for the function pointer to make signatures cleaner
type ReceiverWorkFn<T, C> = Arc<dyn Fn(&T, &mut (), Uuid) -> C + Send + Sync>;

impl<T: Send + 'static, C: Clone + Send + 'static, I: TaskId> TokioEventCollector<T, C, I> {
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(HashMap::new())),
            task_handle: None,
            _marker: PhantomData,
        }
    }

    /// Start processing a stream with the given strategy and receiver function
    pub fn start_processing<S>(
        &mut self, 
        stream: S, 
        strategy: ReceiverStrategy, 
        receiver_fn: ReceiverWorkFn<T, C>
    )
    where
        S: Stream<Item = T> + Send + 'static,
    {
        match strategy {
            ReceiverStrategy::Serial { timeout_seconds } => {
                self.process_serial(stream, timeout_seconds, receiver_fn);
            }
            ReceiverStrategy::Parallel { workers, rate_limit } => {
                self.process_parallel(stream, workers, rate_limit, receiver_fn);
            }
            ReceiverStrategy::Batched { batch_size, max_delay } => {
                self.process_batched(stream, batch_size, max_delay, receiver_fn);
            }
            ReceiverStrategy::Adaptive { 
                initial_capacity, 
                max_concurrency, 
                adaptation_window, 
                use_rayon_for_cpu 
            } => {
                self.process_adaptive(
                    stream, 
                    initial_capacity, 
                    max_concurrency, 
                    adaptation_window, 
                    use_rayon_for_cpu,
                    receiver_fn
                );
            }
        }
    }

    /// Process events serially
    fn process_serial<S>(
        &mut self, 
        mut stream: S, 
        timeout_seconds: u64,
        receiver_fn: ReceiverWorkFn<T,C>
    )
    where
        S: Stream<Item = T> + Send + Unpin + 'static,
    {
        let results_arc = self.results.clone();
        
        self.task_handle = Some(tokio::spawn(async move {
            let timeout_duration = if timeout_seconds > 0 {
                Some(std::time::Duration::from_secs(timeout_seconds))
            } else {
                None
            };
            
            let mut dummy_context = ();
            while let Some(event_data) = if let Some(duration) = timeout_duration {
                tokio::time::timeout(duration, stream.next())
                    .await
                    .unwrap_or(None)
            } else {
                stream.next().await
            } {
                let event_uuid = Uuid::new_v4();
                let processed_item = receiver_fn(&event_data, &mut dummy_context, event_uuid);
                results_arc.lock().unwrap().insert(event_uuid, processed_item);
            }
        }));
    }

    /// Process events in parallel
    fn process_parallel<S>(
        &mut self, 
        stream: S, 
        workers: usize, 
        _rate_limit: f64,
        receiver_fn: ReceiverWorkFn<T,C>
    )
    where
        S: Stream<Item = T> + Send + 'static,
        T: 'static,
        C: 'static,
    {
        let results_arc = self.results.clone();
        let semaphore = Arc::new(Semaphore::new(workers.max(1)));
        let mut stream = Box::pin(stream);

        self.task_handle = Some(tokio::spawn(async move {
            let mut join_handles = Vec::new();
            let mut dummy_context = ();

            while let Some(event_data) = stream.next().await {
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("TokioEventCollector: Semaphore closed, stopping parallel processing.");
                        break;
                    }
                };
                
                let receiver_fn_clone = receiver_fn.clone();
                let results_inner_arc = results_arc.clone();
                
                let task_join_handle = tokio::spawn(async move {
                    let event_uuid = Uuid::new_v4();
                    let mut dummy_context_task = ();
                    let processed_item = receiver_fn_clone(&event_data, &mut dummy_context_task, event_uuid);
                    results_inner_arc.lock().unwrap().insert(event_uuid, processed_item);
                    drop(permit);
                });
                join_handles.push(task_join_handle);
            }

            for handle in join_handles {
                if let Err(e) = handle.await {
                    eprintln!("TokioEventCollector: A parallel processing task failed: {}", e);
                }
            }
        }));
    }

    /// Process events in batches
    fn process_batched<S>(
        &mut self, 
        stream: S, 
        batch_size: usize, 
        max_delay: std::time::Duration,
        receiver_fn: ReceiverWorkFn<T,C>
    )
    where
        S: Stream<Item = T> + Send + 'static,
    {
        let results_arc = self.results.clone();
        
        self.task_handle = Some(tokio::spawn(async move {
            let mut stream = Box::pin(stream);
            let mut batch_with_ids = Vec::with_capacity(batch_size);
            let mut interval = tokio::time::interval(max_delay);
            let mut dummy_context = ();
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if !batch_with_ids.is_empty() {
                            let current_batch_to_process = std::mem::take(&mut batch_with_ids);
                            for (event_uuid, event_data) in current_batch_to_process {
                                let processed_item = receiver_fn(&event_data, &mut dummy_context, event_uuid);
                                results_arc.lock().unwrap().insert(event_uuid, processed_item);
                            }
                        }
                    }
                    event_opt = stream.next() => {
                        match event_opt {
                            Some(event_data) => {
                                let event_uuid = Uuid::new_v4();
                                batch_with_ids.push((event_uuid, event_data));
                                
                                if batch_with_ids.len() >= batch_size {
                                    let current_batch_to_process = std::mem::take(&mut batch_with_ids);
                                    for (event_uuid_in_batch, event_data_in_batch) in current_batch_to_process {
                                        let processed_item = receiver_fn(&event_data_in_batch, &mut dummy_context, event_uuid_in_batch);
                                        results_arc.lock().unwrap().insert(event_uuid_in_batch, processed_item);
                                    }
                                    interval.reset();
                                }
                            }
                            None => {
                                if !batch_with_ids.is_empty(){
                                    let current_batch_to_process = std::mem::take(&mut batch_with_ids);
                                    for (event_uuid_in_batch, event_data_in_batch) in current_batch_to_process {
                                        let processed_item = receiver_fn(&event_data_in_batch, &mut dummy_context, event_uuid_in_batch);
                                        results_arc.lock().unwrap().insert(event_uuid_in_batch, processed_item);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
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
        receiver_fn: ReceiverWorkFn<T,C>
    )
    where
        S: Stream<Item = T> + Unpin + Send + 'static,
        T: Send + Sync + 'static,
        C: Send + 'static,
    {
        let results_arc = self.results.clone();

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
            move |item_t: &T| -> (Uuid, C) {
                let mut dummy_context = ();
                let event_uuid = Uuid::new_v4();
                let processed_item = receiver_fn_clone(item_t, &mut dummy_context, event_uuid);
                (event_uuid, processed_item)
            }
        };

        let dsl = AdaptixAsyncDsl {
            config: adaptive_cfg,
            input_stream: stream,
            map_fn: adapted_map_fn,
        };
            
        self.task_handle = Some(tokio::spawn(async move {
            let mut result_stream = build_adaptive_async_stream(dsl);

            while let Some((event_uuid, processed_item)) = result_stream.next().await {
                results_arc.lock().unwrap().insert(event_uuid, processed_item);
            }
        }));
    }

    /// Get a clone of the collected results HashMap
    pub fn get_results_map(&self) -> HashMap<Uuid, C> {
        self.results.lock().unwrap().clone()
    }

    /// Wait for processing to complete and return the HashMap of results
    pub async fn join(&mut self) -> HashMap<Uuid, C> {
        if let Some(handle) = self.task_handle.take() {
            if let Err(join_error) = handle.await {
                eprintln!("TokioEventCollector processing task failed: {}", join_error);
            }
        }
        self.get_results_map()
    }
}

impl<T: Send + 'static, C: Clone + Send + 'static, I: TaskId> Default for TokioEventCollector<T, C, I> {
    fn default() -> Self {
        Self::new()
    }
}