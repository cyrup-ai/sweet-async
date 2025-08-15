//! Tests for TokioPrioritizedTask and RankableByPriority implementations
//!
//! This module contains all tests for priority-based task management,
//! ensuring proper priority ranking, comparison operations, and task creation.

use sweet_async_tokio::task::{TokioPrioritizedTask, TokioTaskPriority, TokioRankableByPriority, TokioTaskId};
use sweet_async_api::task::{TaskPriority, task_priority::RankableByPriority};

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
    let task = TokioPrioritizedTask::<String>::new(
        TaskPriority::High,
        "test_task".to_string(),
    );

    assert_eq!(task.name(), "test_task");
    assert_eq!(task.priority().as_u8(), 1); // High priority
}

#[test]
fn test_tokio_task_priority_conversions() {
    let original = TaskPriority::High;
    let wrapped = TokioTaskPriority::from(original);
    let converted_back = TaskPriority::from(wrapped);
    
    assert_eq!(original, converted_back);
    assert_eq!(wrapped.as_u8(), 1);
}

#[test]
fn test_priority_comparison_operations() {
    let critical = TokioTaskPriority::from(TaskPriority::Critical);
    let high = TokioTaskPriority::from(TaskPriority::High);
    let normal = TokioTaskPriority::from(TaskPriority::Normal);
    let low = TokioTaskPriority::from(TaskPriority::Low);
    let background = TokioTaskPriority::from(TaskPriority::Background);

    // Test is_higher_than
    assert!(critical.is_higher_than(&high));
    assert!(high.is_higher_than(&normal));
    assert!(normal.is_higher_than(&low));
    assert!(low.is_higher_than(&background));

    // Test is_lower_than
    assert!(background.is_lower_than(&low));
    assert!(low.is_lower_than(&normal));
    assert!(normal.is_lower_than(&high));
    assert!(high.is_lower_than(&critical));

    // Test difference calculation
    assert_eq!(critical.difference(&background), 4);
    assert_eq!(high.difference(&low), 2);
    assert_eq!(normal.difference(&normal), 0);
}

#[test]
fn test_priority_extremes() {
    let highest = TokioTaskPriority::highest();
    let lowest = TokioTaskPriority::lowest();
    
    assert_eq!(highest.as_u8(), 0); // Critical
    assert_eq!(lowest.as_u8(), 4);  // Background
    
    assert!(highest.is_higher_than(&lowest));
    assert!(lowest.is_lower_than(&highest));
}

#[test]
fn test_default_priority() {
    let default = TokioTaskPriority::default_priority();
    assert_eq!(default.as_u8(), 2); // Normal priority
    assert_eq!(TaskPriority::from(default), TaskPriority::Normal);
}

#[test]
fn test_priority_from_u8() {
    assert_eq!(TaskPriority::from_u8(0), TaskPriority::Critical);
    assert_eq!(TaskPriority::from_u8(1), TaskPriority::High);
    assert_eq!(TaskPriority::from_u8(2), TaskPriority::Normal);
    assert_eq!(TaskPriority::from_u8(3), TaskPriority::Low);
    assert_eq!(TaskPriority::from_u8(4), TaskPriority::Background);
    assert_eq!(TaskPriority::from_u8(99), TaskPriority::Background); // Out of range defaults to Background
}

#[test]
fn test_tokio_rankable_by_priority() {
    let priority_5 = TokioRankableByPriority::new(5);
    let priority_8 = TokioRankableByPriority::new(8);
    
    assert_eq!(priority_5.as_u8(), 5);
    assert!(priority_8.is_higher_than(&priority_5));
    assert!(priority_5.is_lower_than(&priority_8));
    
    assert_eq!(priority_5.priority_name(), "Medium");
    assert_eq!(priority_8.priority_name(), "High");
}

#[test]
fn test_rankable_priority_names() {
    assert_eq!(TokioRankableByPriority::new(0).priority_name(), "Idle");
    assert_eq!(TokioRankableByPriority::new(1).priority_name(), "Low");
    assert_eq!(TokioRankableByPriority::new(3).priority_name(), "BelowMedium");
    assert_eq!(TokioRankableByPriority::new(5).priority_name(), "Medium");
    assert_eq!(TokioRankableByPriority::new(6).priority_name(), "AboveMedium");
    assert_eq!(TokioRankableByPriority::new(9).priority_name(), "High");
    assert_eq!(TokioRankableByPriority::new(10).priority_name(), "Critical");
    assert_eq!(TokioRankableByPriority::new(15).priority_name(), "Unknown");
}

#[test]
fn test_prioritized_task_metrics_integration() {
    let task = TokioPrioritizedTask::<Vec<u8>>::new(
        TaskPriority::Critical,
        "metrics_test_task".to_string(),
    );
    
    // Verify metrics are initialized
    assert_eq!(task.cpu_usage().total_time(), std::time::Duration::ZERO);
    assert_eq!(task.memory_usage().current_usage(), 0);
    assert_eq!(task.io_usage().bytes_read(), 0);
    assert_eq!(task.io_usage().bytes_written(), 0);
}

#[test]
fn test_prioritized_task_creation_timestamps() {
    let before = std::time::Instant::now();
    let task = TokioPrioritizedTask::<String>::new(
        TaskPriority::Normal,
        "timestamp_test".to_string(),
    );
    let after = std::time::Instant::now();
    
    let created_at = task.created_at();
    assert!(created_at >= before);
    assert!(created_at <= after);
}