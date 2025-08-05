//! Tokio implementation of PrioritizedTask and RankableByPriority traits
//!
//! This module provides priority-based task management capabilities with
//! full async support and integration with the Tokio runtime.

use sweet_async_api::task::task_priority::{PrioritizedTask, RankableByPriority};
use sweet_async_api::task::MetricsEnabledTask;
use sweet_async_api::task::{TaskId, TaskPriority};

/// Tokio implementation of PrioritizedTask
/// 
/// Combines metrics tracking with priority-based scheduling.
#[derive(Debug, Clone)]
pub struct TokioPrioritizedTask<T: Send + 'static, I: TaskId> {
    task_id: I,
    priority: TaskPriority,
    name: String,
    created_at: std::time::Instant,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static, I: TaskId> TokioPrioritizedTask<T, I> {
    /// Create a new prioritized task with the given priority
    pub fn new(task_id: I, priority: TaskPriority, name: String) -> Self {
        Self {
            task_id,
            priority,
            name,
            created_at: std::time::Instant::now(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create a new prioritized task with normal priority
    pub fn new_normal(task_id: I, name: String) -> Self {
        Self::new(task_id, TaskPriority::Normal, name)
    }
    
    /// Get the task ID
    pub fn task_id(&self) -> &I {
        &self.task_id
    }
    
    /// Get the task name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get when this task was created
    pub fn created_at(&self) -> std::time::Instant {
        self.created_at
    }
}

impl<T: Send + 'static, I: TaskId> MetricsEnabledTask<T> for TokioPrioritizedTask<T, I> {
    fn started_at(&self) -> std::time::Instant {
        self.created_at
    }
    
    fn task_name(&self) -> &str {
        &self.name
    }
    
    fn record_metric(&self, _name: &str, _value: f64) {
        // In a real implementation, this would record metrics
        // For now, we'll just track the basic timing info
    }
    
    fn get_metric(&self, _name: &str) -> Option<f64> {
        // In a real implementation, this would retrieve stored metrics
        None
    }
    
    fn all_metrics(&self) -> std::collections::HashMap<String, f64> {
        // Return basic timing metric
        let mut metrics = std::collections::HashMap::new();
        metrics.insert(
            "age_ms".to_string(),
            self.created_at.elapsed().as_millis() as f64,
        );
        metrics
    }
}

impl<T: Send + 'static, I: TaskId> PrioritizedTask<T> for TokioPrioritizedTask<T, I> {
    fn priority(&self) -> &impl RankableByPriority {
        &self.priority
    }
}

/// Tokio wrapper for TaskPriority to implement RankableByPriority
/// 
/// This enables the standard TaskPriority enum to work with the priority system.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokioTaskPriority(TaskPriority);

impl From<TaskPriority> for TokioTaskPriority {
    fn from(priority: TaskPriority) -> Self {
        Self(priority)
    }
}

impl From<TokioTaskPriority> for TaskPriority {
    fn from(priority: TokioTaskPriority) -> Self {
        priority.0
    }
}

impl RankableByPriority for TokioTaskPriority {
    fn as_u8(&self) -> u8 {
        match self.0 {
            TaskPriority::Critical => 0,
            TaskPriority::High => 1,
            TaskPriority::Normal => 2,
            TaskPriority::Low => 3,
            TaskPriority::Background => 4,
        }
    }
    
    fn from_u8(value: u8) -> Self {
        let priority = match value {
            0 => TaskPriority::Critical,
            1 => TaskPriority::High,
            2 => TaskPriority::Normal,
            3 => TaskPriority::Low,
            _ => TaskPriority::Background,
        };
        TokioTaskPriority(priority)
    }
    
    fn default_priority() -> Self {
        TokioTaskPriority(TaskPriority::Normal)
    }
    
    fn is_higher_than(&self, other: &Self) -> bool {
        self.as_u8() < other.as_u8() // Lower number = higher priority
    }
    
    fn is_lower_than(&self, other: &Self) -> bool {
        self.as_u8() > other.as_u8() // Higher number = lower priority
    }
    
    fn difference(&self, other: &Self) -> u8 {
        let self_val = self.as_u8();
        let other_val = other.as_u8();
        if self_val > other_val {
            self_val - other_val
        } else {
            other_val - self_val
        }
    }
    
    fn highest() -> Self {
        TokioTaskPriority(TaskPriority::Critical)
    }
    
    fn lowest() -> Self {
        TokioTaskPriority(TaskPriority::Background)
    }
}

// Also implement RankableByPriority directly for TaskPriority for convenience
impl RankableByPriority for TaskPriority {
    fn as_u8(&self) -> u8 {
        TokioTaskPriority(*self).as_u8()
    }
    
    fn from_u8(value: u8) -> Self {
        TokioTaskPriority::from_u8(value).0
    }
    
    fn default_priority() -> Self {
        TaskPriority::Normal
    }
    
    fn is_higher_than(&self, other: &Self) -> bool {
        TokioTaskPriority(*self).is_higher_than(&TokioTaskPriority(*other))
    }
    
    fn is_lower_than(&self, other: &Self) -> bool {
        TokioTaskPriority(*self).is_lower_than(&TokioTaskPriority(*other))
    }
    
    fn difference(&self, other: &Self) -> u8 {
        TokioTaskPriority(*self).difference(&TokioTaskPriority(*other))
    }
    
    fn highest() -> Self {
        TaskPriority::Critical
    }
    
    fn lowest() -> Self {
        TaskPriority::Background
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TokioTaskId;
    
    #[test]
    fn test_priority_ranking() {
        let critical = TaskPriority::Critical;
        let normal = TaskPriority::Normal;
        let background = TaskPriority::Background;
        
        assert!(critical.is_higher_than(&normal));
        assert!(normal.is_higher_than(&background));
        assert!(background.is_lower_than(&critical));
        
        assert_eq!(critical.as_u8(), 0);
        assert_eq!(normal.as_u8(), 2);
        assert_eq!(background.as_u8(), 4);
    }
    
    #[test]
    fn test_prioritized_task_creation() {
        let task_id = TokioTaskId::new();
        let task = TokioPrioritizedTask::<String, _>::new(
            task_id,
            TaskPriority::High,
            "test_task".to_string(),
        );
        
        assert_eq!(task.name(), "test_task");
        assert_eq!(task.priority().as_u8(), 1); // High priority
    }
}