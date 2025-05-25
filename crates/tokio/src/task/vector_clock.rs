use std::collections::HashMap;
use std::fmt::Debug;
use sweet_async_api::task::TaskId;

/// Vector clock for distributed causality tracking
#[derive(Clone, Debug, Default)]
pub struct VectorClock<I: TaskId> {
    /// Map of (task_id, hostname) to logical time
    pub clocks: HashMap<(I, String), u64>,
}

impl<I: TaskId> VectorClock<I> {
    /// Create a new empty vector clock
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment our own clock
    pub fn tick(&mut self, task_id: &I, hostname: &str) {
        let key = (task_id.clone(), hostname.to_string());
        let counter = self.clocks.entry(key).or_insert(0);
        *counter += 1;
    }
    
    /// Merge another vector clock into this one (take max of each entry)
    pub fn merge(&mut self, other: &VectorClock<I>) {
        for (key, &other_time) in &other.clocks {
            let our_time = self.clocks.entry(key.clone()).or_insert(0);
            *our_time = (*our_time).max(other_time);
        }
    }
    
    /// Check if this clock happened before another
    pub fn happened_before(&self, other: &VectorClock<I>) -> bool {
        // All our times <= other's times, and at least one is <
        let mut all_leq = true;
        let mut some_less = false;
        
        for (key, &our_time) in &self.clocks {
            let other_time = other.clocks.get(key).copied().unwrap_or(0);
            if our_time > other_time {
                all_leq = false;
                break;
            }
            if our_time < other_time {
                some_less = true;
            }
        }
        
        // Also check if other has entries we don't (counts as us being less)
        for key in other.clocks.keys() {
            if !self.clocks.contains_key(key) {
                some_less = true;
            }
        }
        
        all_leq && some_less
    }
    
    /// Check if two events are concurrent (neither happened before the other)
    pub fn is_concurrent(&self, other: &VectorClock<I>) -> bool {
        !self.happened_before(other) && !other.happened_before(self)
    }
    
    /// Get the logical time for a specific node
    pub fn get_time(&self, task_id: &I, hostname: &str) -> u64 {
        let key = (task_id.clone(), hostname.to_string());
        self.clocks.get(&key).copied().unwrap_or(0)
    }
    
    /// Prune entries older than a certain threshold
    pub fn prune_below_threshold(&mut self, threshold: u64) {
        self.clocks.retain(|_, &mut time| time >= threshold);
    }
    
    /// Get the minimum time across all entries (useful for garbage collection)
    pub fn min_time(&self) -> Option<u64> {
        self.clocks.values().min().copied()
    }
    
    /// Get the maximum time across all entries
    pub fn max_time(&self) -> Option<u64> {
        self.clocks.values().max().copied()
    }
    
    /// Check if this is an empty clock
    pub fn is_empty(&self) -> bool {
        self.clocks.is_empty()
    }
    
    /// Get the number of nodes tracked
    pub fn node_count(&self) -> usize {
        let mut nodes = std::collections::HashSet::new();
        for (_, hostname) in self.clocks.keys() {
            nodes.insert(hostname);
        }
        nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    struct TestTaskId(String);
    
    impl TaskId for TestTaskId {
        fn to_string(&self) -> String {
            self.0.clone()
        }
        
        fn from_string(s: &str) -> Option<Self> {
            Some(TestTaskId(s.to_string()))
        }
    }
    
    #[test]
    fn test_vector_clock_causality() {
        let task1 = TestTaskId("task1".to_string());
        let task2 = TestTaskId("task2".to_string());
        
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();
        
        // Clock1 ticks
        clock1.tick(&task1, "node1");
        assert_eq!(clock1.get_time(&task1, "node1"), 1);
        
        // Clock2 receives from clock1 and ticks
        clock2.merge(&clock1);
        clock2.tick(&task2, "node2");
        
        // Clock1 happened before clock2
        assert!(clock1.happened_before(&clock2));
        assert!(!clock2.happened_before(&clock1));
        assert!(!clock1.is_concurrent(&clock2));
        
        // Both tick independently - now they're concurrent
        clock1.tick(&task1, "node1");
        let clock1_snapshot = clock1.clone();
        
        clock2.tick(&task2, "node2");
        
        assert!(clock1_snapshot.is_concurrent(&clock2));
    }
}