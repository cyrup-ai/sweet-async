use std::collections::HashMap;
use sweet_async_api::task::{TaskId, TaskMessage};
use crate::task::vector_clock::VectorClock;
use crate::task::task_envelope::TaskEnvelopeExt;

/// Metrics snapshot with vector clock for distributed aggregation
#[derive(Clone, Debug)]
pub struct MetricsSnapshot<I: TaskId> {
    /// Task that generated these metrics
    pub task_id: I,
    /// Hostname where metrics were collected
    pub hostname: String,
    /// Vector clock at time of snapshot
    pub vector_clock: VectorClock<I>,
    /// CPU time in nanoseconds
    pub cpu_time_nanos: u64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// I/O bytes read
    pub io_read_bytes: u64,
    /// I/O bytes written
    pub io_write_bytes: u64,
    /// Number of child tasks included in this aggregate
    pub child_count: usize,
}

/// Distributed metrics aggregator
pub struct MetricsAggregator<I: TaskId> {
    /// Accumulated metrics by task/host
    metrics: HashMap<(I, String), MetricsSnapshot<I>>,
    /// Our vector clock
    vector_clock: VectorClock<I>,
}

impl<I: TaskId> MetricsAggregator<I> {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            vector_clock: VectorClock::new(),
        }
    }
    
    /// Add a metrics snapshot
    pub fn add_snapshot(&mut self, snapshot: MetricsSnapshot<I>) {
        // Update our vector clock
        self.vector_clock.merge(&snapshot.vector_clock);
        
        let key = (snapshot.task_id.clone(), snapshot.hostname.clone());
        
        // Check if we have an existing snapshot
        if let Some(existing) = self.metrics.get(&key) {
            // Only update if the new snapshot is causally later
            if existing.vector_clock.happened_before(&snapshot.vector_clock) {
                self.metrics.insert(key, snapshot);
            } else if snapshot.vector_clock.is_concurrent(&existing.vector_clock) {
                // Concurrent updates - merge by taking max
                let merged = MetricsSnapshot {
                    task_id: snapshot.task_id,
                    hostname: snapshot.hostname,
                    vector_clock: {
                        let mut merged_clock = existing.vector_clock.clone();
                        merged_clock.merge(&snapshot.vector_clock);
                        merged_clock
                    },
                    cpu_time_nanos: existing.cpu_time_nanos.max(snapshot.cpu_time_nanos),
                    memory_bytes: existing.memory_bytes.max(snapshot.memory_bytes),
                    io_read_bytes: existing.io_read_bytes.max(snapshot.io_read_bytes),
                    io_write_bytes: existing.io_write_bytes.max(snapshot.io_write_bytes),
                    child_count: existing.child_count.max(snapshot.child_count),
                };
                self.metrics.insert(key, merged);
            }
            // If existing happened after snapshot, keep existing
        } else {
            self.metrics.insert(key, snapshot);
        }
    }
    
    /// Get aggregated metrics across all tasks
    pub fn aggregate_all(&self) -> MetricsSnapshot<I> {
        let mut total_cpu = 0u64;
        let mut max_memory = 0u64;
        let mut total_io_read = 0u64;
        let mut total_io_write = 0u64;
        let mut total_children = 0usize;
        
        for snapshot in self.metrics.values() {
            total_cpu = total_cpu.saturating_add(snapshot.cpu_time_nanos);
            max_memory = max_memory.max(snapshot.memory_bytes);
            total_io_read = total_io_read.saturating_add(snapshot.io_read_bytes);
            total_io_write = total_io_write.saturating_add(snapshot.io_write_bytes);
            total_children = total_children.saturating_add(snapshot.child_count);
        }
        
        // Return a snapshot with aggregated metrics
        // This would need a "system" task ID in practice
        let task_id = match self.metrics.keys().next().map(|(id, _)| id.clone()) {
            Some(id) => id,
            None => {
                // Create a default/system task ID when no metrics exist
                tracing::debug!("No metrics to aggregate, creating empty metrics snapshot");
                I::default()
            }
        };
        
        MetricsSnapshot {
            task_id,
            hostname: "aggregated".to_string(),
            vector_clock: self.vector_clock.clone(),
            cpu_time_nanos: total_cpu,
            memory_bytes: max_memory,
            io_read_bytes: total_io_read,
            io_write_bytes: total_io_write,
            child_count: total_children,
        }
    }
    
    /// Get metrics that are concurrent with a given snapshot
    pub fn get_concurrent_metrics(&self, snapshot: &MetricsSnapshot<I>) -> Vec<&MetricsSnapshot<I>> {
        self.metrics.values()
            .filter(|s| s.vector_clock.is_concurrent(&snapshot.vector_clock))
            .collect()
    }
    
    /// Prune old metrics based on vector clock threshold
    pub fn prune_old_metrics(&mut self, min_logical_time: u64) {
        self.metrics.retain(|_, snapshot| {
            snapshot.vector_clock.max_time().unwrap_or(0) >= min_logical_time
        });
    }
}

/// Convert metrics snapshot to task message
impl<I: TaskId> From<MetricsSnapshot<I>> for TaskMessage<MetricsSnapshot<I>> {
    fn from(snapshot: MetricsSnapshot<I>) -> Self {
        TaskMessage::Data(snapshot)
    }
}