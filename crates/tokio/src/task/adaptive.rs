use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::Stream;
use tokio::sync::{Mutex, mpsc, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, warn};

/// Types of workloads that can be detected and optimized for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkloadType {
    /// CPU-bound workloads benefit from parallel processing with dedicated thread pools
    CpuBound,
    /// IO-bound workloads benefit from high concurrency with async operations
    IoBound,
    /// Mixed workloads have characteristics of both CPU and IO bound operations
    Mixed,
}

/// Configuration for adaptive concurrency 
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Minimum number of concurrent workers
    pub min_workers: usize,
    /// Maximum number of concurrent workers
    pub max_workers: usize,
    /// Sample size for duration tracking
    pub sample_size: usize,
    /// Threshold in milliseconds to classify as IO-bound
    pub io_threshold_ms: u64,
    /// Interval in milliseconds between adaptation checks
    pub adapt_interval_ms: u64,
    /// Chunk size for CPU-bound operations
    pub cpu_chunk_size: usize,
    /// Chunk size for IO-bound operations
    pub io_chunk_size: usize,
    /// Chunk size for mixed workloads
    pub mixed_chunk_size: usize,
    /// Whether to cancel partial chunks when switching modes
    pub partial_cancel: bool,
}

impl Default for AdaptiveConfig {
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
            partial_cancel: false,
        }
    }
}

/// Engine types for processing chunks of work
enum ConcurrencyEngine {
    /// Uses blocking workers for CPU-intensive work
    Rayon,
    /// Uses asynchronous tasks for IO-bound work
    Tokio,
}

impl ConcurrencyEngine {
    /// Spawns a chunk of work on the appropriate engine
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
            ConcurrencyEngine::Rayon if cpu_bound => {
                let (tx, rx) = mpsc::channel(1);
                
                // Use a blocking task to simulate rayon-like behavior
                let map_fn_cl = map_fn.clone();
                tokio::task::spawn_blocking(move || {
                    let start = std::time::Instant::now();
                    let result: Vec<Out> = chunk.iter().map(|it| map_fn_cl(it)).collect();
                    debug!("Rayon chunk job took {:?}", start.elapsed());
                    let _ = tx.blocking_send(result);
                });
                
                EngineTask::Rayon(rx)
            }
            ConcurrencyEngine::Rayon => {
                let (tx, rx) = mpsc::channel(1);
                
                // Use a blocking task for non-CPU-bound work too when using Rayon engine
                let map_fn_cl = map_fn.clone();
                tokio::task::spawn_blocking(move || {
                    let start = std::time::Instant::now();
                    let result: Vec<Out> = chunk.iter().map(|it| map_fn_cl(it)).collect();
                    debug!("Rayon chunk job took {:?}", start.elapsed());
                    let _ = tx.blocking_send(result);
                });
                
                EngineTask::Rayon(rx)
            }
            ConcurrencyEngine::Tokio => {
                let (tx, rx) = mpsc::channel(1);
                
                // Use a regular async task for Tokio engine
                let map_fn_cl = map_fn.clone(); 
                tokio::spawn(async move {
                    let start = std::time::Instant::now();
                    let result: Vec<Out> = chunk.into_iter().map(|it| map_fn_cl(&it)).collect();
                    debug!("Tokio chunk job took {:?}", start.elapsed());
                    let _ = tx.send(result).await;
                });
                
                EngineTask::Tokio(rx)
            }
        }
    }
}

/// A task spawned by either the Rayon or Tokio engine
enum EngineTask<T> {
    /// A task spawned with blocking capabilities
    Rayon(mpsc::Receiver<T>),
    /// A task spawned with async capabilities
    Tokio(mpsc::Receiver<T>),
}

impl<T: Send + 'static> EngineTask<T> {
    /// Wait for the task to complete and return its result
    async fn wait(self) -> Result<T, String> {
        match self {
            EngineTask::Rayon(mut rx) => {
                rx.recv().await.ok_or("Rayon channel closed".to_string())
            }
            EngineTask::Tokio(mut rx) => {
                rx.recv().await.ok_or("Tokio channel closed".to_string())
            }
        }
    }
}

/// Statistics for tracking execution durations
#[derive(Default)]
struct ModeStats {
    durations: Vec<Duration>,
}

impl ModeStats {
    /// Calculate the median duration
    fn median(&self) -> Option<Duration> {
        if self.durations.is_empty() {
            return None;
        }
        
        let mut ds = self.durations.clone();
        ds.sort();
        
        Some(ds[ds.len() / 2])
    }
}

/// State for adaptation decision making
struct AdaptStats {
    /// Ring buffer of recent durations
    ring: ModeStats,
    /// Time of last adaptation
    last_adapt: Instant,
    /// Current worker count
    current_workers: usize,
    /// Current workload type
    workload_type: WorkloadType,
    /// Current chunk size
    chunk_size: usize,
}

impl AdaptStats {
    fn new() -> Self {
        let cpus = num_cpus::get().max(1);
        
        Self {
            ring: ModeStats::default(),
            last_adapt: Instant::now(),
            current_workers: cpus,
            workload_type: WorkloadType::Mixed, // Start in mixed mode
            chunk_size: 16, // Start with a moderate chunk size
        }
    }
}

/// Adapt concurrency based on execution statistics
fn adapt_concurrency(
    st: &mut AdaptStats,
    cfg: &AdaptiveConfig,
    in_flight: &mut Vec<tokio::task::JoinHandle<(Vec<()>, Duration)>>,
) {
    let median = match st.ring.median() {
        Some(m) => m,
        None => return,
    };
    
    // Interpret median > threshold => IO, < threshold => CPU, else Mixed
    let ms = median.as_millis() as u64;
    let new_type = if ms > cfg.io_threshold_ms {
        WorkloadType::IoBound
    } else {
        WorkloadType::CpuBound
    };
    
    // More nuanced classification for mixed workloads
    if matches!(new_type, WorkloadType::CpuBound) && ms > (cfg.io_threshold_ms / 2) {
        // Near the threshold, call it Mixed
        if ms > (cfg.io_threshold_ms * 2 / 3) {
            st.workload_type = WorkloadType::Mixed;
        } else {
            st.workload_type = new_type;
        }
    } else {
        st.workload_type = new_type;
    }
    
    // Get CPU count for concurrency decisions
    let cpus = num_cpus::get().max(1);
    
    // Incrementally ramp chunk_size to target
    if st.workload_type == WorkloadType::CpuBound && st.chunk_size < cfg.cpu_chunk_size {
        st.chunk_size = (st.chunk_size + cfg.cpu_chunk_size) / 2; // ramp up
    } else if st.workload_type == WorkloadType::IoBound && st.chunk_size > cfg.io_chunk_size {
        st.chunk_size = (st.chunk_size + cfg.io_chunk_size) / 2; // ramp down
    } else if st.workload_type == WorkloadType::Mixed 
        && st.chunk_size != cfg.mixed_chunk_size
    {
        st.chunk_size = (st.chunk_size + cfg.mixed_chunk_size) / 2;
    }
    
    // Determine worker count based on workload type
    let new_workers = match st.workload_type {
        WorkloadType::CpuBound => cpus.clamp(cfg.min_workers, cfg.max_workers),
        WorkloadType::IoBound => (cpus * 2).clamp(cfg.min_workers, cfg.max_workers),
        WorkloadType::Mixed => ((cpus * 3) / 2).clamp(cfg.min_workers, cfg.max_workers),
    };
    
    debug!(
        "Adapting concurrency: workload={:?}, chunk_size={}, workers={}, median_ms={}",
        st.workload_type, st.chunk_size, new_workers, ms
    );
    
    // Update state
    st.current_workers = new_workers;
    st.ring.durations.clear();
    st.last_adapt = Instant::now();
    
    // If partial_cancel is enabled, cancel in-flight tasks when switching modes
    if cfg.partial_cancel {
        if (matches!(st.workload_type, WorkloadType::CpuBound) && ms > cfg.io_threshold_ms * 2) ||
           (matches!(st.workload_type, WorkloadType::IoBound) && ms < cfg.io_threshold_ms / 2) 
        {
            debug!("Cancelling {} in-flight tasks due to major mode switch", in_flight.len());
            for jh in in_flight.drain(..) {
                jh.abort();
            }
        }
    }
}

/// Create an adaptive stream that processes items with optimized concurrency
pub fn adaptive_stream<It, Item, MapFn, Out>(
    items: It,
    map_fn: MapFn,
    cfg: AdaptiveConfig,
) -> impl Stream<Item = Out>
where
    It: IntoIterator<Item = Item> + Send + 'static,
    <It as IntoIterator>::IntoIter: Send,
    Item: Send + Sync + 'static,
    MapFn: Fn(&Item) -> Out + Send + Sync + 'static,
    Out: Send + 'static,
{
    let (tx_out, rx_out) = mpsc::channel::<Out>(cfg.max_workers.saturating_mul(2));
    let map_fn = Arc::new(map_fn);
    
    tokio::spawn(async move {
        let mut st = AdaptStats::new();
        let concurrency_sema = Arc::new(Semaphore::new(st.current_workers));
        let mut chunk_buf = Vec::with_capacity(st.chunk_size);
        
        let mut input_iter = items.into_iter();
        let mut in_flight: Vec<tokio::task::JoinHandle<(Vec<Out>, Duration)>> = Vec::new();
        
        // Helper function to spawn a chunk job
        fn spawn_chunk_job<Item, Out>(
            chunk: Vec<Item>,
            st: &mut AdaptStats,
            map_fn: &Arc<dyn Fn(&Item) -> Out + Send + Sync + 'static>,
            sema_cl: &Arc<Semaphore>,
            cfg: &AdaptiveConfig,
        ) -> Option<tokio::task::JoinHandle<(Vec<Out>, Duration)>>
        where
            Item: Send + Sync + 'static,
            Out: Send + 'static,
        {
            if chunk.is_empty() {
                return None;
            }
            
            let start = Instant::now();
            let cpu_bound = matches!(st.workload_type, WorkloadType::CpuBound);
            let engine = if cpu_bound {
                ConcurrencyEngine::Rayon
            } else {
                ConcurrencyEngine::Tokio
            };
            
            let map_cl = Arc::clone(map_fn);
            let sema_c = Arc::clone(sema_cl);
            
            let task = engine.spawn_chunk(chunk, map_cl, cpu_bound);
            
            Some(tokio::spawn(async move {
                let _permit = sema_c.acquire_owned().await.unwrap();
                let outputs = match task.wait().await {
                    Ok(o) => o,
                    Err(e) => {
                        warn!("Chunk job failed: {e}");
                        Vec::new()
                    }
                };
                
                (outputs, start.elapsed())
            }))
        }
        
        // Process the input iterator
        loop {
            let next_item = input_iter.next();
            
            match next_item {
                Some(it) => {
                    chunk_buf.push(it);
                    
                    if chunk_buf.len() >= st.chunk_size {
                        let chunk = std::mem::take(&mut chunk_buf);
                        
                        if let Some(fut) = spawn_chunk_job(
                            chunk,
                            &mut st,
                            &map_fn,
                            &concurrency_sema,
                            &cfg,
                        ) {
                            in_flight.push(fut);
                        }
                    }
                }
                None => {
                    if !chunk_buf.is_empty() {
                        let chunk = std::mem::take(&mut chunk_buf);
                        
                        if let Some(fut) = spawn_chunk_job(
                            chunk,
                            &mut st,
                            &map_fn,
                            &concurrency_sema,
                            &cfg,
                        ) {
                            in_flight.push(fut);
                        }
                    }
                    
                    break;
                }
            }
        }
        
        // Process in-flight tasks
        for handle in &mut in_flight {
            match handle.await {
                Ok((outputs, dur)) => {
                    st.ring.durations.push(dur);
                    
                    for out_val in outputs {
                        if tx_out.send(out_val).await.is_err() {
                            break;
                        }
                    }
                    
                    // Check if we should adapt
                    let now = Instant::now();
                    if st.ring.durations.len() >= cfg.sample_size
                        && now.duration_since(st.last_adapt).as_millis()
                            > cfg.adapt_interval_ms as u128
                    {
                        adapt_concurrency(&mut st, &cfg, &mut in_flight);
                        
                        // Update concurrency
                        concurrency_sema.close();
                        concurrency_sema.add_permits(
                            st.current_workers
                                .saturating_sub(concurrency_sema.available_permits()),
                        );
                    }
                }
                Err(e) => {
                    if e.is_cancelled() {
                        debug!("Chunk job was cancelled");
                    } else {
                        warn!("Chunk job failed: {e}");
                    }
                }
            }
        }
        
        debug!("Adaptive stream processing complete");
        in_flight.clear();
        drop(tx_out);
    });
    
    ReceiverStream::new(rx_out)
}
