use sweet_async_api::task::builder::MinMax;

/// Strategies for sending events in parallel or serial execution
#[derive(Debug, Clone)]
pub enum SenderStrategy {
    /// Execute in parallel with specified worker configuration
    Parallel { workers: MinMax<usize> },
    /// Execute serially (single-threaded)
    Serial,
}

/// Strategies for receiving events 
#[derive(Debug, Clone)]
pub enum ReceiverStrategy {
    /// Receive events serially with a timeout
    Serial { timeout_seconds: u64 },
    /// Receive events in parallel - uses adaptive strategy internally
    Parallel { 
        workers: MinMax<usize>,
        rate_limit: Option<u64>,
    },
    /// Receive events in batches
    Batched {
        batch_size: usize,
        max_delay: std::time::Duration,
    },
    /// Adaptive strategy that adjusts based on workload
    Adaptive {
        initial_capacity: usize,
        max_concurrency: usize,
        adaptation_window: std::time::Duration,
        use_rayon_for_cpu: bool,
    },
}

impl Default for SenderStrategy {
    fn default() -> Self {
        Self::Serial
    }
}

impl Default for ReceiverStrategy {
    fn default() -> Self {
        Self::Serial { timeout_seconds: 30 }
    }
}