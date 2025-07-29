//! Task priority implementation for the Tokio runtime

use std::cmp::Ordering;
use sweet_async_api::task::{
    PrioritizedTask, TaskPriority, RankableByPriority, MetricsEnabledTask,
    CpuUsage, MemoryUsage, IoUsage
};

/// Tokio-specific task priority implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokioTaskPriority {
    value: i32,
}

impl TokioTaskPriority {
    /// Highest priority (most urgent)
    pub const CRITICAL: i32 = 1000;
    /// High priority
    pub const HIGH: i32 = 750;
    /// Normal priority (default)
    pub const NORMAL: i32 = 500;
    /// Low priority
    pub const LOW: i32 = 250;
    /// Lowest priority (background tasks)
    pub const BACKGROUND: i32 = 0;

    /// Create a new priority with the given value
    pub fn new(value: i32) -> Self {
        Self { value }
    }

    /// Create a critical priority task
    pub fn critical() -> Self {
        Self::new(Self::CRITICAL)
    }

    /// Create a high priority task
    pub fn high() -> Self {
        Self::new(Self::HIGH)
    }

    /// Create a normal priority task
    pub fn normal() -> Self {
        Self::new(Self::NORMAL)
    }

    /// Create a low priority task
    pub fn low() -> Self {
        Self::new(Self::LOW)
    }

    /// Create a background priority task
    pub fn background() -> Self {
        Self::new(Self::BACKGROUND)
    }

    /// Get the raw priority value
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Check if this priority is higher than another
    pub fn is_higher_than(&self, other: &Self) -> bool {
        self.value > other.value
    }

    /// Check if this priority is lower than another
    pub fn is_lower_than(&self, other: &Self) -> bool {
        self.value < other.value
    }

    /// Increase priority by the given amount
    pub fn increase(&mut self, amount: i32) {
        self.value = self.value.saturating_add(amount);
    }

    /// Decrease priority by the given amount
    pub fn decrease(&mut self, amount: i32) {
        self.value = self.value.saturating_sub(amount);
    }
}

impl Default for TokioTaskPriority {
    fn default() -> Self {
        Self::normal()
    }
}

impl PartialOrd for TokioTaskPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TokioTaskPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl TaskPriority for TokioTaskPriority {
    fn priority_level(&self) -> i32 {
        self.value
    }
}

impl RankableByPriority for TokioTaskPriority {
    fn as_u8(&self) -> u8 {
        // Map i32 values to u8 range (0-4) where lower u8 = higher priority
        match self.value {
            val if val >= Self::CRITICAL => 0,  // Critical priority
            val if val >= Self::HIGH => 1,      // High priority
            val if val >= Self::NORMAL => 2,    // Normal priority  
            val if val >= Self::LOW => 3,       // Low priority
            _ => 4,                              // Background priority
        }
    }

    fn from_u8(value: u8) -> Self {
        // Map u8 range (0-4) back to i32 priority values
        match value {
            0 => Self::critical(),   // Critical
            1 => Self::high(),       // High
            2 => Self::normal(),     // Normal (default)
            3 => Self::low(),        // Low
            _ => Self::background(), // Background
        }
    }

    fn default_priority() -> Self {
        Self::normal()
    }

    fn is_higher_than(&self, other: &Self) -> bool {
        self.value > other.value
    }

    fn is_lower_than(&self, other: &Self) -> bool {
        self.value < other.value
    }

    fn difference(&self, other: &Self) -> u8 {
        let self_u8 = self.as_u8();
        let other_u8 = other.as_u8();
        if self_u8 > other_u8 {
            self_u8 - other_u8
        } else {
            other_u8 - self_u8
        }
    }

    fn highest() -> Self {
        Self::critical()
    }

    fn lowest() -> Self {
        Self::background()
    }
}

impl From<i32> for TokioTaskPriority {
    fn from(value: i32) -> Self {
        Self::new(value)
    }
}

impl From<TokioTaskPriority> for i32 {
    fn from(priority: TokioTaskPriority) -> Self {
        priority.value
    }
}

/// Task priority tracker for dynamic priority adjustment
#[derive(Debug)]
pub struct TokioTaskPriorityTracker {
    initial_priority: TokioTaskPriority,
    current_priority: TokioTaskPriority,
    adjustments: Vec<PriorityAdjustment>,
    /// CPU usage metrics for this task
    cpu_metrics: CpuUsage,
    /// Memory usage metrics for this task
    memory_metrics: MemoryUsage,
    /// I/O usage metrics for this task
    io_metrics: IoUsage,
}

#[derive(Debug, Clone)]
struct PriorityAdjustment {
    reason: String,
    amount: i32,
    timestamp: std::time::Instant,
}

impl TokioTaskPriorityTracker {
    /// Create a new priority tracker
    pub fn new(initial_priority: TokioTaskPriority) -> Self {
        Self {
            initial_priority,
            current_priority: initial_priority,
            adjustments: Vec::new(),
            cpu_metrics: CpuUsage {
                percent: 0.0,
                total_time: std::time::Duration::default(),
            },
            memory_metrics: MemoryUsage {
                current_bytes: 0,
                peak_bytes: 0,
            },
            io_metrics: IoUsage {
                operations_count: 0,
                bytes_read: 0,
                bytes_written: 0,
            },
        }
    }

    /// Get the current priority
    pub fn priority(&self) -> &TokioTaskPriority {
        &self.current_priority
    }

    /// Adjust the priority with a reason
    pub fn adjust_priority(&mut self, amount: i32, reason: impl Into<String>) {
        let adjustment = PriorityAdjustment {
            reason: reason.into(),
            amount,
            timestamp: std::time::Instant::now(),
        };
        
        self.current_priority.value = self.current_priority.value.saturating_add(amount);
        self.adjustments.push(adjustment);
    }

    /// Reset to initial priority
    pub fn reset_priority(&mut self) {
        self.current_priority = self.initial_priority;
        self.adjustments.clear();
    }

    /// Get priority adjustment history
    pub fn adjustment_history(&self) -> &[PriorityAdjustment] {
        &self.adjustments
    }
}

/// Implement MetricsEnabledTask for TokioTaskPriorityTracker
impl<T: Send + 'static> MetricsEnabledTask<T> for TokioTaskPriorityTracker {
    fn cpu_usage(&self) -> CpuUsage {
        self.cpu_metrics
    }

    fn memory_usage(&self) -> MemoryUsage {
        self.memory_metrics
    }

    fn io_usage(&self) -> IoUsage {
        self.io_metrics
    }
}

impl<T: Send + 'static> PrioritizedTask<T> for TokioTaskPriorityTracker {
    fn priority(&self) -> &impl RankableByPriority {
        &self.current_priority
    }

    fn set_priority(&mut self, priority: impl TaskPriority) {
        let new_priority = TokioTaskPriority::new(priority.priority_level());
        self.adjust_priority(
            new_priority.value - self.current_priority.value,
            "Manual priority change"
        );
    }

    fn increase_priority(&mut self, amount: i32) {
        self.adjust_priority(amount, "Priority increase");
    }

    fn decrease_priority(&mut self, amount: i32) {
        self.adjust_priority(-amount, "Priority decrease");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_creation() {
        let high = TokioTaskPriority::high();
        let normal = TokioTaskPriority::normal();
        let low = TokioTaskPriority::low();

        assert!(high.is_higher_than(&normal));
        assert!(normal.is_higher_than(&low));
        assert!(low.is_lower_than(&normal));
    }

    #[test]
    fn test_priority_comparison() {
        let p1 = TokioTaskPriority::new(100);
        let p2 = TokioTaskPriority::new(200);

        assert!(p2 > p1);
        assert!(p1 < p2);
        assert_eq!(p1.cmp(&p2), Ordering::Less);
    }

    #[test]
    fn test_priority_tracker() {
        let mut tracker = TokioTaskPriorityTracker::new(TokioTaskPriority::normal());
        
        assert_eq!(tracker.priority().value(), TokioTaskPriority::NORMAL);
        
        tracker.adjust_priority(100, "Test adjustment");
        assert_eq!(tracker.priority().value(), TokioTaskPriority::NORMAL + 100);
        
        tracker.reset_priority();
        assert_eq!(tracker.priority().value(), TokioTaskPriority::NORMAL);
    }
}