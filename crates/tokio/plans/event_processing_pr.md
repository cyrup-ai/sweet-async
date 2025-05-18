# Event Processing Implementation PR

![Sweet Async Logo](/assets/sweet_async.png)

This document outlines the implementation plan for the Event Processing system in the Tokio implementation of Sweet Async, which will build on the foundation established by the previous PRs.

## Overview

Event processing is a powerful feature of the Sweet Async API that provides a stream-based alternative to future-based task execution. This PR will implement the complete event processing system, including event types, processing strategies, collectors, and the emitting task builder.

## Implementation Plan

### 1. Create Basic Event Types

First, create the base event implementation files:

**`src/task/emit/event.rs`**:
```rust
use std::fmt::Debug;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use sweet_async_api::task::emit::{StreamingEvent, StreamingEventType, SenderEvent, SenderEventBuilder, ReceiverEvent};

/// Base event implementation for the Tokio runtime
#[derive(Debug, Clone)]
pub struct TokioEvent<T: Send + 'static> {
    /// When the event was created
    created: DateTime<Utc>,
    /// When the event started processing
    started: DateTime<Utc>,
    /// When the event completed processing
    completed: DateTime<Utc>,
    /// Unique event identifier
    event_id: Uuid,
    /// Task identifier
    task_id: Uuid,
    /// Event payload
    data: T,
    /// Type of this event
    event_type: StreamingEventType<T>,
    /// Whether this is a final event
    is_final_event: bool,
}

impl<T: Send + 'static> TokioEvent<T> {
    /// Create a new event
    pub fn new(task_id: Uuid, data: T) -> Self {
        let now = Utc::now();
        Self {
            created: now,
            started: now,
            completed: now,
            event_id: Uuid::new_v4(),
            task_id,
            data,
            event_type: StreamingEventType::Continue,
            is_final_event: false,
        }
    }
    
    /// Create a new final event
    pub fn new_final(task_id: Uuid, data: T) -> Self {
        let now = Utc::now();
        Self {
            created: now,
            started: now,
            completed: now,
            event_id: Uuid::new_v4(),
            task_id,
            data,
            event_type: StreamingEventType::Final(data.clone()),
            is_final_event: true,
        }
    }
    
    /// Create a new error event
    pub fn new_error(task_id: Uuid, data: T, error: String) -> Self {
        let now = Utc::now();
        Self {
            created: now,
            started: now,
            completed: now,
            event_id: Uuid::new_v4(),
            task_id,
            data,
            event_type: StreamingEventType::Error(error),
            is_final_event: true,
        }
    }
    
    /// Create a new cancellation event
    pub fn new_cancellation(task_id: Uuid, data: T) -> Self {
        let now = Utc::now();
        Self {
            created: now,
            started: now,
            completed: now,
            event_id: Uuid::new_v4(),
            task_id,
            data,
            event_type: StreamingEventType::Cancellation,
            is_final_event: true,
        }
    }
}

impl<T: Send + 'static> StreamingEvent<T> for TokioEvent<T> {
    fn created_timestamp(&self) -> &DateTime<Utc> {
        &self.created
    }
    
    fn started_timestamp(&self) -> &DateTime<Utc> {
        &self.started
    }
    
    fn completed_timestamp(&self) -> &DateTime<Utc> {
        &self.completed
    }
    
    fn event_id(&self) -> &Uuid {
        &self.event_id
    }
    
    fn task_id(&self) -> &Uuid {
        &self.task_id
    }
    
    fn data(&self) -> &T {
        &self.data
    }
    
    fn event_type(&self) -> &StreamingEventType<T> {
        &self.event_type
    }
    
    fn is_final(&self) -> bool {
        self.is_final_event
    }
}

/// Builder for TokioEvent
pub struct TokioEventBuilder<T: Send + 'static> {
    /// Task identifier
    task_id: Uuid,
    /// Event identifier
    event_id: Uuid,
    /// Event payload
    data: T,
    /// Event type
    event_type: StreamingEventType<T>,
    /// Whether this is a final event
    is_final_event: bool,
    /// Creation timestamp
    created: DateTime<Utc>,
    /// Start timestamp
    started: DateTime<Utc>,
    /// Completion timestamp
    completed: DateTime<Utc>,
}

impl<T: Send + 'static> SenderEventBuilder<T> for TokioEventBuilder<T> {
    fn new(task_id: Uuid, event_id: Uuid, data: T) -> Self {
        let now = Utc::now();
        Self {
            task_id,
            event_id,
            data,
            event_type: StreamingEventType::Continue,
            is_final_event: false,
            created: now,
            started: now,
            completed: now,
        }
    }
    
    fn event_type(self, event_type: StreamingEventType<T>) -> Self {
        Self {
            event_type,
            ..self
        }
    }
    
    fn data(self, data: T) -> Self {
        Self {
            data,
            ..self
        }
    }
    
    fn is_final(self) -> Self {
        Self {
            is_final_event: true,
            ..self
        }
    }
}

impl<T: Send + 'static> SenderEvent<T> for TokioEvent<T> {
    type Builder = TokioEventBuilder<T>;
    
    fn builder() -> Self::Builder {
        TokioEventBuilder::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            unsafe { std::mem::zeroed() } // This is a placeholder, actual data must be provided
        )
    }
    
    fn event_type(event_type: StreamingEventType<T>) -> Self::Builder {
        let builder = Self::builder();
        builder.event_type(event_type)
    }
    
    fn data(data: T) -> Self::Builder {
        let builder = Self::builder();
        builder.data(data)
    }
    
    fn is_final() -> Self::Builder {
        let builder = Self::builder();
        builder.is_final()
    }
}

impl<T: Send + 'static> From<TokioEventBuilder<T>> for TokioEvent<T> {
    fn from(builder: TokioEventBuilder<T>) -> Self {
        Self {
            created: builder.created,
            started: builder.started,
            completed: builder.completed,
            event_id: builder.event_id,
            task_id: builder.task_id,
            data: builder.data,
            event_type: builder.event_type,
            is_final_event: builder.is_final_event,
        }
    }
}

/// Receiver event implementation
/// Adapts a streaming event for use in receivers
pub struct TokioReceiverEvent<T, C> {
    inner: Arc<TokioEvent<T>>,
    _phantom: std::marker::PhantomData<C>,
}

impl<T: Send + 'static, C: Send + 'static> TokioReceiverEvent<T, C> {
    /// Create a new receiver event wrapping a streaming event
    pub fn new(event: TokioEvent<T>) -> Self {
        Self {
            inner: Arc::new(event),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Get the inner event
    pub fn inner(&self) -> &TokioEvent<T> {
        &self.inner
    }
}

impl<T: Send + 'static, C: Send + 'static> ReceiverEvent<T, C> for TokioReceiverEvent<T, C> {
    fn event_id(&self) -> &Uuid {
        self.inner.event_id()
    }
    
    fn data(&self) -> &T {
        self.inner.data()
    }
    
    fn is_final(&self) -> bool {
        self.inner.is_final()
    }
}

/// Implementation of the final event for collector
pub struct TokioFinalEvent<T, C, F> {
    /// The inner event
    inner: TokioEvent<T>,
    /// The collected results
    collection: Arc<Mutex<HashMap<Uuid, C>>>,
    /// The final result
    result: Option<F>,
}

impl<T: Send + 'static, C: Send + 'static, F: Send + 'static> sweet_async_api::task::emit::FinalEvent<T, C, F> for TokioFinalEvent<T, C, F> {
    fn event(&self) -> &dyn StreamingEvent<T> {
        &self.inner
    }
    
    fn collection(&self) -> &dyn sweet_async_api::task::emit::Collection<Uuid, C> {
        unimplemented!("Collection trait needs to be implemented")
    }
    
    fn result(&self) -> Option<&F> {
        self.result.as_ref()
    }
}
```

### 2. Implement Event Collector

**`src/task/emit/collector.rs`**:
```rust
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use sweet_async_api::task::emit::Collector;

/// Collector for event results
pub struct TokioEventCollector<K, V> 
where
    K: Into<Uuid> + Clone + Hash + Eq + Send + 'static,
    V: Clone + Send + 'static,
{
    /// The collected items
    items: Arc<Mutex<HashMap<Uuid, V>>>,
}

impl<K, V> TokioEventCollector<K, V>
where
    K: Into<Uuid> + Clone + Hash + Eq + Send + 'static,
    V: Clone + Send + 'static,
{
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get the inner storage
    pub fn inner(&self) -> Arc<Mutex<HashMap<Uuid, V>>> {
        self.items.clone()
    }
}

impl<K, V> Default for TokioEventCollector<K, V>
where
    K: Into<Uuid> + Clone + Hash + Eq + Send + 'static,
    V: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Collector<K, V> for TokioEventCollector<K, V>
where
    K: Into<Uuid> + Clone + Hash + Eq + Send + 'static,
    V: Clone + Send + 'static,
{
    fn collect<KI>(&mut self, key: KI, item: V)
    where
        KI: Into<Uuid>,
    {
        let key_uuid = key.into();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut items = self.items.lock().await;
                items.insert(key_uuid, item);
            });
        });
    }
    
    fn collect_item(&mut self, item: V) {
        let key = Uuid::new_v4();
        self.collect(key, item);
    }
    
    fn collect_items(&mut self, items: Vec<V>) {
        for item in items {
            self.collect_item(item);
        }
    }
    
    fn collected(&self) -> &HashMap<Uuid, V> {
        unimplemented!("Cannot directly access the HashMap due to async mutex. Use inner() instead.")
    }
}

impl<K, V> sweet_async_api::task::emit::Collection<Uuid, V> for TokioEventCollector<K, V>
where
    K: Into<Uuid> + Clone + Hash + Eq + Send + 'static,
    V: Clone + Send + 'static,
{
    fn get(&self, key: &Uuid) -> Option<V> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let items = self.items.lock().await;
                items.get(key).cloned()
            })
        })
    }
    
    fn len(&self) -> usize {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let items = self.items.lock().await;
                items.len()
            })
        })
    }
    
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

### 3. Implement Processing Strategies

Create a new directory and files for processing strategies:

**`src/task/emit/strategy/mod.rs`**:
```rust
//! Processing strategies for event streams
//! 
//! This module contains implementations of different strategies for
//! processing event streams, including serial, parallel, and batched.

mod serial;
mod parallel;
mod batched;
mod adaptive;

pub use serial::SerialStrategy;
pub use parallel::ParallelStrategy;
pub use batched::BatchedStrategy;
pub use adaptive::AdaptiveStrategy;

use std::fmt::Debug;
use sweet_async_api::task::emit::ReceiverEvent;

/// Base trait for processing strategies
pub trait ProcessingStrategy<T, C>: Send + Sync + 'static
where
    T: Send + 'static,
    C: Send + 'static,
{
    /// Process a batch of events
    async fn process_batch<E, F>(&self, events: Vec<E>, processor: F) -> Vec<C>
    where
        E: ReceiverEvent<T, C> + Send + 'static,
        F: Fn(&T) -> C + Send + Sync + 'static + Clone;
    
    /// Get the name of this strategy
    fn name(&self) -> &'static str;
}
```

**`src/task/emit/strategy/serial.rs`**:
```rust
use super::ProcessingStrategy;
use sweet_async_api::task::emit::ReceiverEvent;

/// Serial processing strategy
/// 
/// Processes events one at a time in the order they are received.
pub struct SerialStrategy {
    /// Maximum time in milliseconds to process each event
    pub timeout_ms: u64,
}

impl SerialStrategy {
    /// Create a new serial strategy
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl<T, C> ProcessingStrategy<T, C> for SerialStrategy
where
    T: Send + 'static,
    C: Send + 'static,
{
    async fn process_batch<E, F>(&self, events: Vec<E>, processor: F) -> Vec<C>
    where
        E: ReceiverEvent<T, C> + Send + 'static,
        F: Fn(&T) -> C + Send + Sync + 'static + Clone,
    {
        let mut results = Vec::with_capacity(events.len());
        
        for event in events {
            // Process with timeout if configured
            let data = event.data();
            
            if self.timeout_ms > 0 {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(self.timeout_ms),
                    async { processor(data) }
                ).await {
                    Ok(result) => results.push(result),
                    Err(_) => {
                        // Timeout occurred, skip this event
                        tracing::warn!("Event processing timed out after {}ms", self.timeout_ms);
                    }
                }
            } else {
                // No timeout
                results.push(processor(data));
            }
        }
        
        results
    }
    
    fn name(&self) -> &'static str {
        "Serial"
    }
}
```

**`src/task/emit/strategy/parallel.rs`**:
```rust
use super::ProcessingStrategy;
use sweet_async_api::task::builder::MinMax;
use sweet_async_api::task::emit::ReceiverEvent;
use std::sync::Arc;
use futures::stream::{self, StreamExt};

/// Parallel processing strategy
/// 
/// Processes events concurrently using a configurable number of workers.
pub struct ParallelStrategy {
    /// Worker count range (min, max)
    pub workers: MinMax<usize>,
}

impl ParallelStrategy {
    /// Create a new parallel strategy
    pub fn new(min_workers: usize, max_workers: usize) -> Self {
        Self {
            workers: MinMax::new(min_workers, max_workers),
        }
    }
    
    /// Create a new parallel strategy from an existing MinMax
    pub fn from_minmax(workers: MinMax<usize>) -> Self {
        Self { workers }
    }
}

impl<T, C> ProcessingStrategy<T, C> for ParallelStrategy
where
    T: Send + 'static,
    C: Send + 'static,
{
    async fn process_batch<E, F>(&self, events: Vec<E>, processor: F) -> Vec<C>
    where
        E: ReceiverEvent<T, C> + Send + 'static,
        F: Fn(&T) -> C + Send + Sync + 'static + Clone,
    {
        // Determine worker count based on event count and configured range
        let worker_count = self.workers.min().max(1).min(
            self.workers.max().min(events.len())
        );
        
        // Create a shareable processor function
        let processor = Arc::new(processor);
        
        // Process events in parallel with the determined concurrency
        stream::iter(events)
            .map(|event| {
                let processor = processor.clone();
                async move {
                    let data = event.data();
                    processor(data)
                }
            })
            .buffer_unordered(worker_count)
            .collect::<Vec<_>>()
            .await
    }
    
    fn name(&self) -> &'static str {
        "Parallel"
    }
}
```

**`src/task/emit/strategy/batched.rs`**:
```rust
use super::ProcessingStrategy;
use sweet_async_api::task::emit::ReceiverEvent;

/// Batched processing strategy
/// 
/// Processes events in batches of configurable size.
pub struct BatchedStrategy {
    /// The size of each batch
    pub batch_size: usize,
    /// The maximum time to wait for a batch to fill in milliseconds
    pub batch_timeout_ms: u64,
}

impl BatchedStrategy {
    /// Create a new batched strategy
    pub fn new(batch_size: usize, batch_timeout_ms: u64) -> Self {
        Self {
            batch_size,
            batch_timeout_ms,
        }
    }
}

impl<T, C> ProcessingStrategy<T, C> for BatchedStrategy
where
    T: Send + 'static,
    C: Send + 'static,
{
    async fn process_batch<E, F>(&self, events: Vec<E>, processor: F) -> Vec<C>
    where
        E: ReceiverEvent<T, C> + Send + 'static,
        F: Fn(&T) -> C + Send + Sync + 'static + Clone,
    {
        let mut results = Vec::with_capacity(events.len());
        let mut current_batch = Vec::with_capacity(self.batch_size);
        
        for event in events {
            current_batch.push(event);
            
            if current_batch.len() >= self.batch_size {
                // Process the full batch
                let batch_data: Vec<&T> = current_batch.iter().map(|e| e.data()).collect();
                
                // Process each item in the batch
                for data in batch_data {
                    results.push(processor(data));
                }
                
                // Clear the batch
                current_batch.clear();
            }
        }
        
        // Process any remaining items
        if !current_batch.is_empty() {
            let batch_data: Vec<&T> = current_batch.iter().map(|e| e.data()).collect();
            
            for data in batch_data {
                results.push(processor(data));
            }
        }
        
        results
    }
    
    fn name(&self) -> &'static str {
        "Batched"
    }
}
```

**`src/task/emit/strategy/adaptive.rs`**:
```rust
use super::{ProcessingStrategy, ParallelStrategy, SerialStrategy, BatchedStrategy};
use sweet_async_api::task::emit::ReceiverEvent;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Adaptive processing strategy
/// 
/// Automatically adapts between serial, parallel, and batched processing
/// based on runtime characteristics of the events.
pub struct AdaptiveStrategy {
    /// The serial strategy to use when appropriate
    serial: SerialStrategy,
    /// The parallel strategy to use when appropriate
    parallel: ParallelStrategy,
    /// The batched strategy to use when appropriate
    batched: BatchedStrategy,
    /// Number of samples to collect before adapting
    sample_size: usize,
    /// Execution times for recent events
    execution_times: Arc<Mutex<Vec<u128>>>,
    /// Current strategy in use
    current_strategy: Arc<Mutex<AdaptiveStrategyType>>,
}

/// Types of strategies that can be used adaptively
enum AdaptiveStrategyType {
    Serial,
    Parallel,
    Batched,
}

impl AdaptiveStrategy {
    /// Create a new adaptive strategy
    pub fn new(
        serial: SerialStrategy,
        parallel: ParallelStrategy,
        batched: BatchedStrategy,
        sample_size: usize,
    ) -> Self {
        Self {
            serial,
            parallel,
            batched,
            sample_size,
            execution_times: Arc::new(Mutex::new(Vec::with_capacity(sample_size))),
            current_strategy: Arc::new(Mutex::new(AdaptiveStrategyType::Serial)),
        }
    }
    
    /// Analyze execution times and select the best strategy
    async fn adapt(&self) {
        let times = {
            let mut times = self.execution_times.lock().await;
            let times_copy = times.clone();
            times.clear();
            times_copy
        };
        
        if times.len() < self.sample_size {
            return;
        }
        
        // Calculate average execution time
        let avg_time = times.iter().sum::<u128>() / times.len() as u128;
        
        // Determine strategy based on execution characteristics
        let new_strategy = if avg_time < 10 {
            // Very fast operations - use parallel
            AdaptiveStrategyType::Parallel
        } else if avg_time < 50 {
            // Medium speed - use batched
            AdaptiveStrategyType::Batched
        } else {
            // Slow operations - use serial
            AdaptiveStrategyType::Serial
        };
        
        // Update current strategy
        let mut current = self.current_strategy.lock().await;
        *current = new_strategy;
    }
}

impl<T, C> ProcessingStrategy<T, C> for AdaptiveStrategy
where
    T: Send + 'static,
    C: Send + 'static,
{
    async fn process_batch<E, F>(&self, events: Vec<E>, processor: F) -> Vec<C>
    where
        E: ReceiverEvent<T, C> + Send + 'static,
        F: Fn(&T) -> C + Send + Sync + 'static + Clone,
    {
        // Determine which strategy to use
        let strategy_type = {
            let strategy = self.current_strategy.lock().await;
            match *strategy {
                AdaptiveStrategyType::Serial => "Serial",
                AdaptiveStrategyType::Parallel => "Parallel",
                AdaptiveStrategyType::Batched => "Batched",
            }
        };
        
        tracing::info!("Using {} strategy for batch of {} events", strategy_type, events.len());
        
        // Clone processor for timing
        let processor_clone = processor.clone();
        
        // Process with the current strategy
        let start = Instant::now();
        let results = match strategy_type {
            "Serial" => self.serial.process_batch(events, processor).await,
            "Parallel" => self.parallel.process_batch(events, processor).await,
            "Batched" => self.batched.process_batch(events, processor).await,
            _ => self.serial.process_batch(events, processor).await,
        };
        let elapsed = start.elapsed().as_millis();
        
        // Record execution time
        {
            let mut times = self.execution_times.lock().await;
            times.push(elapsed);
            
            // If we have enough samples, adapt the strategy
            if times.len() >= self.sample_size {
                drop(times);
                self.adapt().await;
            }
        }
        
        results
    }
    
    fn name(&self) -> &'static str {
        "Adaptive"
    }
}
```

### 4. Implement Emitting Task

**`src/task/emit/task.rs`**:
```rust
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Sender, Receiver};

use sweet_async_api::orchestra::OrchestratorError;
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskStatus};
use sweet_async_api::task::emit::{EmittingTask, FinalEvent};

use crate::task::tokio_task::AsyncTask;
use crate::task::emit::event::{TokioEvent, TokioReceiverEvent, TokioFinalEvent};
use crate::task::emit::collector::TokioEventCollector;
use crate::task::emit::strategy::ProcessingStrategy;

use tokio::time::timeout;

/// Task that emits events for processing
pub struct TokioEmittingTask<T, C, E, I>
where
    T: Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// The underlying task
    task: AsyncTask<T, I>,
    /// Event sender
    sender: Option<Sender<TokioEvent<T>>>,
    /// Event receiver
    receiver: Option<Receiver<TokioEvent<T>>>,
    /// Event collector
    collector: Arc<Mutex<TokioEventCollector<uuid::Uuid, C>>>,
    /// Phantom data for error type
    _phantom_e: PhantomData<E>,
}

impl<T, C, E, I> TokioEmittingTask<T, C, E, I>
where
    T: Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Create a new emitting task
    pub fn new(task: AsyncTask<T, I>) -> Self {
        // Create the channel for events
        let (sender, receiver) = mpsc::channel(100);
        
        Self {
            task,
            sender: Some(sender),
            receiver: Some(receiver),
            collector: Arc::new(Mutex::new(TokioEventCollector::new())),
            _phantom_e: PhantomData,
        }
    }
    
    /// Start processing events with the given strategy and processor function
    pub async fn start_processing<F, S>(
        &mut self,
        strategy: S,
        processor: F,
    ) -> Result<(), AsyncTaskError>
    where
        F: Fn(&T) -> C + Send + Sync + 'static + Clone,
        S: ProcessingStrategy<T, C> + 'static,
    {
        let mut receiver = self.receiver.take()
            .ok_or_else(|| AsyncTaskError::Failure("Receiver already consumed".to_string()))?;
        
        let strategy = Arc::new(strategy);
        let processor = Arc::new(processor);
        let collector = self.collector.clone();
        
        // Start a task to process events
        let process_task = tokio::spawn(async move {
            let mut batch = Vec::new();
            let batch_size = 10; // Configurable
            
            while let Some(event) = receiver.recv().await {
                // Convert to receiver event
                let receiver_event = TokioReceiverEvent::new(event.clone());
                batch.push(receiver_event);
                
                // Process batch if it's full or we received a final event
                if batch.len() >= batch_size || event.is_final() {
                    let process_batch = batch.clone();
                    batch.clear();
                    
                    let strategy_clone = strategy.clone();
                    let processor_clone = processor.clone();
                    
                    // Process the batch
                    let results = strategy_clone.process_batch(process_batch, processor_clone.as_ref().clone()).await;
                    
                    // Store results in collector
                    let mut collector_lock = collector.lock().await;
                    for result in results {
                        collector_lock.collect_item(result);
                    }
                    
                    if event.is_final() {
                        break;
                    }
                }
            }
        });
        
        // Wait for processing to complete with timeout
        if let Some(timeout_duration) = self.task.timeout() {
            if timeout_duration > Duration::from_secs(0) {
                match timeout(timeout_duration, process_task).await {
                    Ok(result) => {
                        if let Err(e) = result {
                            return Err(AsyncTaskError::Failure(format!("Event processing failed: {}", e)));
                        }
                    }
                    Err(_) => {
                        return Err(AsyncTaskError::Timeout(timeout_duration));
                    }
                }
            } else {
                // No timeout
                if let Err(e) = process_task.await {
                    return Err(AsyncTaskError::Failure(format!("Event processing failed: {}", e)));
                }
            }
        } else {
            // No timeout
            if let Err(e) = process_task.await {
                return Err(AsyncTaskError::Failure(format!("Event processing failed: {}", e)));
            }
        }
        
        Ok(())
    }
    
    /// Send an event to the processing pipeline
    pub async fn send_event(&self, event: TokioEvent<T>) -> Result<(), AsyncTaskError> {
        if let Some(sender) = &self.sender {
            sender.send(event).await
                .map_err(|_| AsyncTaskError::Failure("Failed to send event".to_string()))
        } else {
            Err(AsyncTaskError::Failure("Sender has been consumed".to_string()))
        }
    }
    
    /// Get the collector
    pub fn collector(&self) -> Arc<Mutex<TokioEventCollector<uuid::Uuid, C>>> {
        self.collector.clone()
    }
}

impl<T, C, E, I> EmittingTask<T, C, E, I> for TokioEmittingTask<T, C, E, I>
where
    T: Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    type Final = TokioFinalEvent<T, C, C>;
    
    fn is_complete(&self) -> bool {
        matches!(self.task.status(), TaskStatus::Completed)
    }
    
    fn cancel(&self) -> Result<(), OrchestratorError> {
        futures::executor::block_on(self.task.cancel(sweet_async_api::task::CancellationLevel::Graceful))
    }
}

impl<T, C, E, I> AsyncTask<T, I> for TokioEmittingTask<T, C, E, I>
where
    T: Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        self.task.to::<R, Task>()
    }
    
    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        self.task.emits::<R, Task>()
    }
}

// Forward all AsyncTask trait methods to the inner task
// (Implement all the other required traits)
```

### 5. Implement Emitting Task Builder

**`src/task/emit/builder.rs`**:
```rust
use std::marker::PhantomData;
use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::{AsyncTaskBuilder, ReceiverBuilder, ReceiverStrategy, SenderBuilder};
use sweet_async_api::task::emit::EmittingTask;

use crate::task::builder::AsyncTaskBuilder;
use crate::task::tokio_task::AsyncTask;
use crate::task::emit::task::TokioEmittingTask;
use crate::task::emit::strategy::{SerialStrategy, ParallelStrategy, BatchedStrategy, AdaptiveStrategy};

/// Builder for tasks that emit events
pub struct TokioEmittingTaskBuilder<T, U, E, I>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// The base builder
    base: AsyncTaskBuilder<T, I>,
    /// Phantom data for event types
    _phantom: PhantomData<(U, E)>,
}

impl<T, U, E, I> TokioEmittingTaskBuilder<T, U, E, I>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Create a new emitting task builder
    pub fn new(
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    ) -> Self {
        Self {
            base: AsyncTaskBuilder::new(runtime, active_tasks),
            _phantom: PhantomData,
        }
    }
}

impl<T, U, E, I> AsyncTaskBuilder for TokioEmittingTaskBuilder<T, U, E, I>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    fn new() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        Self::new(runtime, active_tasks)
    }
    
    fn name(self, name: &str) -> Self {
        Self {
            base: self.base.name(name),
            ..self
        }
    }
    
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

/// Receiver builder for processing emitted events
pub struct TokioReceiverBuilder<T, U, E, I>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// The emitting task builder
    builder: TokioEmittingTaskBuilder<T, U, E, I>,
    /// The processing strategy
    strategy: ReceiverStrategy,
}

impl<T, U, E, I> SenderBuilder<T, U, E, I> for TokioEmittingTaskBuilder<T, U, E, I>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    type Receiver = TokioReceiverBuilder<T, U, E, I>;
    type StreamProcessorReceiver<C: Send + 'static, Coll: Send + 'static> = TokioReceiverBuilder<T, Coll, E, I>;
    
    fn receiver(
        self,
        strategy: ReceiverStrategy,
        _processor: fn(&T, &mut (), uuid::Uuid) -> U,
    ) -> Self::Receiver {
        TokioReceiverBuilder {
            builder: self,
            strategy,
        }
    }
    
    fn stream_processor<C: Send + 'static, F, S, Coll: Send + 'static>(
        self,
        _processor: F,
        strategy: ReceiverStrategy,
    ) -> Self::StreamProcessorReceiver<C, Coll>
    where
        F: sweet_async_api::task::builder::AsyncWork<C> + Send + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: sweet_async_api::task::emit::ReceiverEvent<T, C>,
        C: std::future::Future<Output = ()> + Send + 'static,
        Coll: Default + Send + 'static
    {
        TokioReceiverBuilder {
            builder: self,
            strategy,
        }
    }
}

impl<T, U, E, I> ReceiverBuilder<T, U, E, I> for TokioReceiverBuilder<T, U, E, I>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    type Task = TokioEmittingTask<T, U, E, I>;
    
    fn start_queue(self) -> Self::Task {
        // Create a base task
        let id = I::generate();
        let task = AsyncTask::new(
            id,
            sweet_async_api::task::TaskPriority::Normal,
            self.builder.base.runtime().clone(),
            self.builder.base.active_tasks().clone(),
        );
        
        // Apply configuration
        let task = task
            .with_timeout(self.builder.base.get_timeout())
            .with_retry(self.builder.base.get_retry_attempts())
            .with_tracing(self.builder.base.is_tracing_enabled());
            
        // Create the emitting task
        let mut emitting_task = TokioEmittingTask::new(task);
        
        // Start processing with the appropriate strategy
        let processor = |_: &T| U::default(); // Placeholder, should be replaced with actual processor
        
        match self.strategy {
            ReceiverStrategy::Serial { timeout_seconds } => {
                let strategy = SerialStrategy::new(timeout_seconds * 1000);
                tokio::spawn(async move {
                    let _ = emitting_task.start_processing(strategy, processor).await;
                });
            }
            ReceiverStrategy::Parallel { workers } => {
                let strategy = ParallelStrategy::from_minmax(workers);
                tokio::spawn(async move {
                    let _ = emitting_task.start_processing(strategy, processor).await;
                });
            }
            ReceiverStrategy::Batched { batch_size, timeout_seconds } => {
                let strategy = BatchedStrategy::new(batch_size, timeout_seconds * 1000);
                tokio::spawn(async move {
                    let _ = emitting_task.start_processing(strategy, processor).await;
                });
            }
            ReceiverStrategy::Adaptive { sample_size } => {
                let serial = SerialStrategy::new(1000);
                let parallel = ParallelStrategy::new(1, num_cpus::get());
                let batched = BatchedStrategy::new(10, 1000);
                let strategy = AdaptiveStrategy::new(serial, parallel, batched, sample_size);
                tokio::spawn(async move {
                    let _ = emitting_task.start_processing(strategy, processor).await;
                });
            }
        }
        
        emitting_task
    }
}
```

### 6. Update AsyncTask Implementation to Support Emitting

Update the AsyncTask trait implementation in AsyncTask:

```rust
impl<T: Send + 'static, I: TaskId> AsyncTask<T, I> for AsyncTask<T, I> {
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::task::spawn::builder::TokioSpawningTaskBuilder;
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        TokioSpawningTaskBuilder::<R, AsyncTaskError, I>::new(runtime, active_tasks)
    }

    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::task::emit::builder::TokioEmittingTaskBuilder;
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        TokioEmittingTaskBuilder::<R, (), AsyncTaskError, I>::new(runtime, active_tasks)
    }
}
```

### 7. Add Convenience Functions to Builder Module

Add emitting builder convenience function to `src/builder.rs`:

```rust
/// Creates a new emitting task builder with default settings
///
/// This builder is specifically for stream-based workflows that process
/// a sequence of events.
///
/// # Example
///
/// ```rust
/// use sweet_async_tokio::builder;
/// use sweet_async_api::task::TaskId;
///
/// // Using a custom task ID type
/// let builder = builder::emitting_builder::<String, impl TaskId>();
/// ```
pub fn emitting_builder<T: Send + 'static, I: TaskId>() -> task::emit::builder::TokioEmittingTaskBuilder<T, (), AsyncTaskError, I> {
    let runtime = Handle::current();
    let active_tasks = Arc::new(Mutex::new(Vec::new()));
    task::emit::builder::TokioEmittingTaskBuilder::<T, (), AsyncTaskError, I>::new(runtime, active_tasks)
}
```

### 8. Add Tests

Create a test file for event processing:

**`tests/event_processing_tests.rs`**:
```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio_test::block_on;
use sweet_async_api::task::{AsyncTask, TaskId, TaskStatus};
use sweet_async_api::task::builder::ReceiverStrategy;
use sweet_async_api::task::emit::EmittingTask;
use sweet_async_tokio::builder;

// Simple test ID implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TestTaskId(u64);

impl TaskId for TestTaskId {
    fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TestTaskId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn to_string(&self) -> String {
        format!("TestTask-{}", self.0)
    }
}

#[test]
fn test_basic_event_processing() {
    // Create an emitting task
    let builder = builder::emitting_builder::<String, TestTaskId>();
    
    // Configure with serial processing
    let emitting_task = builder
        .receiver(
            ReceiverStrategy::Serial { timeout_seconds: 10 },
            |data, _, _| data.len()
        )
        .start_queue();
    
    // Send some events
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    
    block_on(async {
        // Create an event
        let event = sweet_async_tokio::task::emit::event::TokioEvent::new(
            uuid::Uuid::new_v4(),
            "Test event".to_string()
        );
        
        // Send the event
        emitting_task.send_event(event).await.unwrap();
        
        // Send a final event
        let final_event = sweet_async_tokio::task::emit::event::TokioEvent::new_final(
            uuid::Uuid::new_v4(),
            "Final event".to_string()
        );
        
        emitting_task.send_event(final_event).await.unwrap();
        
        // Wait for processing to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check collector
        let collector = emitting_task.collector();
        let items = collector.lock().await;
        assert_eq!(items.len(), 2);
        
        counter_clone.store(items.len(), Ordering::SeqCst);
    });
    
    assert_eq!(counter.load(Ordering::SeqCst), 2);
    assert!(emitting_task.is_complete());
}

#[test]
fn test_parallel_processing() {
    // Create an emitting task with parallel processing
    let builder = builder::emitting_builder::<usize, TestTaskId>();
    
    // Configure with parallel processing
    let emitting_task = builder
        .receiver(
            ReceiverStrategy::Parallel { 
                workers: sweet_async_api::task::builder::MinMax(2, 4) 
            },
            |data, _, _| *data * 2
        )
        .start_queue();
    
    // Send events
    block_on(async {
        // Send 10 events
        for i in 0..10 {
            let event = sweet_async_tokio::task::emit::event::TokioEvent::new(
                uuid::Uuid::new_v4(),
                i
            );
            
            emitting_task.send_event(event).await.unwrap();
        }
        
        // Send final event
        let final_event = sweet_async_tokio::task::emit::event::TokioEvent::new_final(
            uuid::Uuid::new_v4(),
            100
        );
        
        emitting_task.send_event(final_event).await.unwrap();
        
        // Wait for processing to complete
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Check collector
        let collector = emitting_task.collector();
        let items = collector.lock().await;
        assert_eq!(items.len(), 11);
    });
}

#[test]
fn test_cancellation() {
    // Create an emitting task
    let builder = builder::emitting_builder::<String, TestTaskId>();
    
    // Configure with batched processing
    let emitting_task = builder
        .receiver(
            ReceiverStrategy::Batched { 
                batch_size: 5,
                timeout_seconds: 10
            },
            |data, _, _| data.len()
        )
        .start_queue();
    
    block_on(async {
        // Send some events
        for i in 0..5 {
            let event = sweet_async_tokio::task::emit::event::TokioEvent::new(
                uuid::Uuid::new_v4(),
                format!("Event {}", i)
            );
            
            emitting_task.send_event(event).await.unwrap();
        }
        
        // Cancel the task
        emitting_task.cancel().unwrap();
        
        // Verify cancellation
        assert_eq!(emitting_task.task.status(), TaskStatus::Cancelled);
    });
}

#[test]
fn test_adaptive_strategy() {
    // Create an emitting task with adaptive processing
    let builder = builder::emitting_builder::<usize, TestTaskId>();
    
    // Configure with adaptive processing
    let emitting_task = builder
        .receiver(
            ReceiverStrategy::Adaptive { sample_size: 20 },
            |data, _, _| *data
        )
        .start_queue();
    
    // Send a mix of fast and slow events
    block_on(async {
        // Send 100 events
        for i in 0..100 {
            let event = sweet_async_tokio::task::emit::event::TokioEvent::new(
                uuid::Uuid::new_v4(),
                i
            );
            
            emitting_task.send_event(event).await.unwrap();
            
            // Alternate between fast and slow operations
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
        
        // Send final event
        let final_event = sweet_async_tokio::task::emit::event::TokioEvent::new_final(
            uuid::Uuid::new_v4(),
            999
        );
        
        emitting_task.send_event(final_event).await.unwrap();
        
        // Wait for processing to complete
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Check collector
        let collector = emitting_task.collector();
        let items = collector.lock().await;
        assert_eq!(items.len(), 101);
    });
}
```

## Implementation Notes

1. The event processing implementation is complex and has many moving parts
2. The different processing strategies provide flexibility for different workloads
3. The adaptive strategy automatically chooses the best approach based on runtime characteristics
4. Events flow through a channel-based system with backpressure support
5. Final events signal the end of processing

## Expected Outcome

After this PR:

1. Users will be able to create event-based tasks using the builder pattern
2. The static method `AsyncTask::emits()` will work for creating event builders
3. Different processing strategies will be available for different workloads
4. The adaptive strategy will automatically optimize processing
5. Event collection will work properly for aggregating results

![Book](/assets/book.png)
