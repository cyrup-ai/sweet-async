use crate::task::TaskId;

/// Trait for tasks in distributed systems with vector clocks
pub trait DistributedTask<I: TaskId> {
    /// Vector clock type for tracking causality
    type VectorClock: Clone + Send + Sync;
    
    /// Get the task's vector clock
    fn vector_clock(&self) -> &Self::VectorClock;
    
    /// Update the vector clock
    fn tick_clock(&mut self);
    
    /// Merge vector clock from another task
    fn update_clock_from(&mut self, other: &Self::VectorClock);
}