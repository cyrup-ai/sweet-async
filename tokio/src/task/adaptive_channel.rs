//! Channel-based adaptive task execution without Arc or Semaphores
//!
//! This module provides adaptive concurrency using only channels for coordination.

use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Message types for the adaptive executor
#[derive(Debug)]
enum AdaptiveMessage<Item, Out> {
    /// Submit work to be processed
    ProcessItem(Item, oneshot::Sender<Out>),
    /// Update concurrency level
    UpdateConcurrency(usize),
    /// Get current statistics
    GetStats(oneshot::Sender<AdaptiveStats>),
    /// Shutdown the executor
    Shutdown,
}

/// Statistics for adaptive execution
#[derive(Debug, Clone)]
pub struct AdaptiveStats {
    pub total_items: usize,
    pub processed_items: usize,
    pub current_workers: usize,
    pub avg_latency_ms: f64,
    pub throughput_per_sec: f64,
}

/// Channel-based adaptive executor
pub struct ChannelAdaptiveExecutor<Item, Out> {
    sender: mpsc::UnboundedSender<AdaptiveMessage<Item, Out>>,
}

impl<Item, Out> ChannelAdaptiveExecutor<Item, Out>
where
    Item: Send + 'static,
    Out: Send + 'static,
{
    /// Create a new adaptive executor with initial worker count
    pub fn new<F>(initial_workers: usize, process_fn: F) -> Self
    where
        F: Fn(Item) -> Out + Send + Sync + 'static + Clone,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        // Spawn the coordinator task
        tokio::spawn(async move {
            let mut stats = AdaptiveStats {
                total_items: 0,
                processed_items: 0,
                current_workers: initial_workers,
                avg_latency_ms: 0.0,
                throughput_per_sec: 0.0,
            };
            
            // Worker control channels
            let mut worker_txs = Vec::new();
            let mut worker_handles = Vec::new();
            
            // Spawn initial workers
            for _ in 0..initial_workers {
                let (worker_tx, worker_rx) = mpsc::unbounded_channel();
                let process_fn = process_fn.clone();
                
                let handle = tokio::spawn(worker_task(worker_rx, process_fn));
                worker_txs.push(worker_tx);
                worker_handles.push(handle);
            }
            
            let start_time = Instant::now();
            let mut total_latency_ms = 0.0;
            
            while let Some(msg) = rx.recv().await {
                match msg {
                    AdaptiveMessage::ProcessItem(item, respond_tx) => {
                        stats.total_items += 1;
                        
                        // Round-robin to workers
                        let worker_idx = stats.total_items % worker_txs.len();
                        let item_start = Instant::now();
                        
                        // Send to worker
                        if let Some(worker_tx) = worker_txs.get(worker_idx) {
                            let _ = worker_tx.send((item, respond_tx, item_start));
                        }
                    }
                    
                    AdaptiveMessage::UpdateConcurrency(new_workers) => {
                        if new_workers > stats.current_workers {
                            // Add workers
                            for _ in stats.current_workers..new_workers {
                                let (worker_tx, worker_rx) = mpsc::unbounded_channel();
                                let process_fn = process_fn.clone();
                                
                                let handle = tokio::spawn(worker_task(worker_rx, process_fn));
                                worker_txs.push(worker_tx);
                                worker_handles.push(handle);
                            }
                        } else if new_workers < stats.current_workers {
                            // Remove workers
                            while worker_txs.len() > new_workers {
                                if let Some(tx) = worker_txs.pop() {
                                    drop(tx); // This will cause the worker to exit
                                }
                                if let Some(handle) = worker_handles.pop() {
                                    let _ = handle.await;
                                }
                            }
                        }
                        stats.current_workers = new_workers;
                        info!("Updated concurrency to {} workers", new_workers);
                    }
                    
                    AdaptiveMessage::GetStats(respond_tx) => {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        stats.throughput_per_sec = if elapsed > 0.0 {
                            stats.processed_items as f64 / elapsed
                        } else {
                            0.0
                        };
                        
                        stats.avg_latency_ms = if stats.processed_items > 0 {
                            total_latency_ms / stats.processed_items as f64
                        } else {
                            0.0
                        };
                        
                        let _ = respond_tx.send(stats.clone());
                    }
                    
                    AdaptiveMessage::Shutdown => {
                        // Close all workers
                        worker_txs.clear();
                        for handle in worker_handles.drain(..) {
                            let _ = handle.await;
                        }
                        break;
                    }
                }
            }
        });
        
        Self { sender: tx }
    }
    
    /// Process a single item
    pub async fn process(&self, item: Item) -> Result<Out, String> {
        let (tx, rx) = oneshot::channel();
        
        self.sender
            .send(AdaptiveMessage::ProcessItem(item, tx))
            .map_err(|_| "Executor shut down".to_string())?;
            
        rx.await.map_err(|_| "Worker failed".to_string())
    }
    
    /// Process a batch of items
    pub async fn process_batch(&self, items: Vec<Item>) -> Vec<Result<Out, String>> {
        let mut results = Vec::with_capacity(items.len());
        let mut handles = Vec::new();
        
        for item in items {
            let sender = self.sender.clone();
            let handle = tokio::spawn(async move {
                let (tx, rx) = oneshot::channel();
                if let Err(e) = sender.send(AdaptiveMessage::ProcessItem(item, tx)) {
                    return Err(format!("Failed to send: {}", e));
                }
                rx.await.map_err(|_| "Worker failed".to_string())
            });
            handles.push(handle);
        }
        
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(format!("Task panicked: {}", e))),
            }
        }
        
        results
    }
    
    /// Update the concurrency level
    pub async fn set_concurrency(&self, workers: usize) -> Result<(), String> {
        self.sender
            .send(AdaptiveMessage::UpdateConcurrency(workers))
            .map_err(|_| "Executor shut down".to_string())
    }
    
    /// Get current statistics
    pub async fn stats(&self) -> Result<AdaptiveStats, String> {
        let (tx, rx) = oneshot::channel();
        
        self.sender
            .send(AdaptiveMessage::GetStats(tx))
            .map_err(|_| "Executor shut down".to_string())?;
            
        rx.await.map_err(|_| "Failed to get stats".to_string())
    }
    
    /// Shutdown the executor
    pub async fn shutdown(self) -> Result<(), String> {
        self.sender
            .send(AdaptiveMessage::Shutdown)
            .map_err(|_| "Already shut down".to_string())
    }
}

/// Worker task that processes items
async fn worker_task<Item, Out, F>(
    mut rx: mpsc::UnboundedReceiver<(Item, oneshot::Sender<Out>, Instant)>,
    process_fn: F,
) where
    F: Fn(Item) -> Out + Send + 'static,
    Item: Send + 'static,
    Out: Send + 'static,
{
    while let Some((item, respond_tx, start_time)) = rx.recv().await {
        let result = process_fn(item);
        let _ = respond_tx.send(result);
        
        let latency = start_time.elapsed();
        if latency > Duration::from_secs(1) {
            warn!("Slow processing: {:?}", latency);
        }
    }
}

/// Channel-based adaptive iterator
pub struct ChannelAdaptiveIterator<Item, Out> {
    executor: ChannelAdaptiveExecutor<Item, Out>,
    adaptation_interval: Duration,
    target_latency_ms: f64,
}

impl<Item, Out> ChannelAdaptiveIterator<Item, Out>
where
    Item: Send + 'static,
    Out: Send + 'static,
{
    /// Create a new adaptive iterator
    pub fn new<F>(
        initial_workers: usize,
        process_fn: F,
        adaptation_interval: Duration,
        target_latency_ms: f64,
    ) -> Self
    where
        F: Fn(Item) -> Out + Send + Sync + 'static + Clone,
    {
        Self {
            executor: ChannelAdaptiveExecutor::new(initial_workers, process_fn),
            adaptation_interval,
            target_latency_ms,
        }
    }
    
    /// Process items adaptively
    pub async fn process_adaptive(
        &self,
        items: Vec<Item>,
    ) -> Result<Vec<Out>, String> {
        let mut results = Vec::new();
        let mut last_adaptation = Instant::now();
        
        // Process in chunks to allow adaptation
        let chunk_size = 100;
        for chunk in items.chunks(chunk_size) {
            // Process chunk
            let chunk_results = self.executor.process_batch(chunk.to_vec()).await;
            
            // Collect successful results
            for result in chunk_results {
                match result {
                    Ok(out) => results.push(out),
                    Err(e) => return Err(e),
                }
            }
            
            // Check if we should adapt
            if last_adaptation.elapsed() > self.adaptation_interval {
                if let Ok(stats) = self.executor.stats().await {
                    let new_workers = self.calculate_optimal_workers(&stats);
                    if new_workers != stats.current_workers {
                        let _ = self.executor.set_concurrency(new_workers).await;
                    }
                }
                last_adaptation = Instant::now();
            }
        }
        
        Ok(results)
    }
    
    /// Calculate optimal worker count based on stats
    fn calculate_optimal_workers(&self, stats: &AdaptiveStats) -> usize {
        if stats.avg_latency_ms > self.target_latency_ms * 1.5 {
            // Too slow, add workers
            (stats.current_workers + 1).min(num_cpus::get() * 2)
        } else if stats.avg_latency_ms < self.target_latency_ms * 0.5 {
            // Too fast (over-provisioned), remove workers
            (stats.current_workers - 1).max(1)
        } else {
            // Just right
            stats.current_workers
        }
    }
}

// Re-export for compatibility
pub use self::{
    ChannelAdaptiveExecutor as AdaptiveExecutor,
    ChannelAdaptiveIterator as AdaptiveIterator,
    AdaptiveStats as AdaptixStats,
};

use tokio::sync::oneshot;