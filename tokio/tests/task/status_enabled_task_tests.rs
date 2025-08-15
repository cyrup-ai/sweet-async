//! Tests for TokioStatusEnabledTask implementation
//!
//! This module contains all tests for status-enabled task functionality,
//! ensuring proper status transitions, atomic operations integrity, and thread safety.

use std::sync::Arc;
use std::thread;
use sweet_async_tokio::task::status_enabled_task::TokioStatusEnabledTask;
use sweet_async_api::task::{TaskStatus, task_status::StatusEnabledTask};

#[test]
fn test_status_transitions() {
    let task = TokioStatusEnabledTask::<String>::new();

    // Initial status should be Pending
    assert!(matches!(task.status(), TaskStatus::Pending));
    assert!(task.is_active());
    assert!(!task.is_terminal());

    // Mark as running
    task.mark_running();
    assert!(matches!(task.status(), TaskStatus::Running));
    assert!(task.is_active());
    assert!(!task.is_terminal());

    // Mark as completed
    task.mark_completed();
    assert!(matches!(task.status(), TaskStatus::Completed));
    assert!(!task.is_active());
    assert!(task.is_terminal());
}

#[test]
fn test_cancellation_flow() {
    let task = TokioStatusEnabledTask::<String>::new();

    task.mark_running();
    task.mark_pending_cancellation();
    assert!(matches!(task.status(), TaskStatus::PendingCancellation));
    assert!(task.is_active());

    task.mark_cancelled();
    assert!(matches!(task.status(), TaskStatus::Cancelled));
    assert!(task.is_terminal());
}

#[test]
fn test_thread_safety() {
    let task = Arc::new(TokioStatusEnabledTask::<String>::new());

    let task_clone = Arc::clone(&task);
    let handle = thread::spawn(move || {
        task_clone.mark_running();
        task_clone.mark_completed();
    });

    handle.join().expect("Thread should complete successfully");
    assert!(matches!(task.status(), TaskStatus::Completed));
}

#[test]
fn test_with_initial_status() {
    let task = TokioStatusEnabledTask::<String>::with_status(TaskStatus::Running);
    assert!(matches!(task.status(), TaskStatus::Running));
}

#[test]
fn test_status_helpers() {
    let task = TokioStatusEnabledTask::<String>::new();
    
    // Test pending state
    assert!(task.is_active());
    assert!(!task.is_terminal());
    
    // Test running state
    task.mark_running();
    assert!(task.is_active());
    assert!(!task.is_terminal());
    
    // Test failed state
    task.mark_failed();
    assert!(!task.is_active());
    assert!(task.is_terminal());
}

#[test]
fn test_atomic_operations_integrity() {
    let task = TokioStatusEnabledTask::<Vec<u8>>::new();
    
    // Test that atomic operations preserve ordering
    task.set_status(TaskStatus::Running);
    assert_eq!(task.status(), TaskStatus::Running);
    
    task.set_status(TaskStatus::Completed);
    assert_eq!(task.status(), TaskStatus::Completed);
    
    // Test that status reads are consistent
    for _ in 0..1000 {
        assert_eq!(task.status(), TaskStatus::Completed);
    }
}

#[test]
fn test_concurrent_status_updates() {
    let task = Arc::new(TokioStatusEnabledTask::<String>::new());
    let mut handles = Vec::new();
    
    // Spawn multiple threads that try to update status
    for i in 0..10 {
        let task_clone = Arc::clone(&task);
        let handle = thread::spawn(move || {
            if i % 2 == 0 {
                task_clone.mark_running();
            } else {
                task_clone.mark_pending_cancellation();
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    // Status should be one of the valid states (not corrupted)
    let final_status = task.status();
    assert!(matches!(
        final_status,
        TaskStatus::Running | TaskStatus::PendingCancellation
    ));
}

#[test]
fn test_all_status_transitions() {
    let task = TokioStatusEnabledTask::<i32>::new();
    
    // Test all possible status transitions
    let statuses = [
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskStatus::PendingCancellation,
        TaskStatus::Cancelled,
        TaskStatus::Failed,
        TaskStatus::Completed,
    ];
    
    for &status in &statuses {
        task.set_status(status);
        assert_eq!(task.status(), status);
    }
}

#[test]
fn test_terminal_state_detection() {
    let task = TokioStatusEnabledTask::<String>::new();
    
    // Non-terminal states
    task.set_status(TaskStatus::Pending);
    assert!(!task.is_terminal());
    
    task.set_status(TaskStatus::Running);
    assert!(!task.is_terminal());
    
    task.set_status(TaskStatus::PendingCancellation);
    assert!(!task.is_terminal());
    
    // Terminal states
    task.set_status(TaskStatus::Completed);
    assert!(task.is_terminal());
    
    task.set_status(TaskStatus::Failed);
    assert!(task.is_terminal());
    
    task.set_status(TaskStatus::Cancelled);
    assert!(task.is_terminal());
}

#[test]
fn test_active_state_detection() {
    let task = TokioStatusEnabledTask::<String>::new();
    
    // Active states
    task.set_status(TaskStatus::Pending);
    assert!(task.is_active());
    
    task.set_status(TaskStatus::Running);
    assert!(task.is_active());
    
    task.set_status(TaskStatus::PendingCancellation);
    assert!(task.is_active());
    
    // Inactive states (terminal)
    task.set_status(TaskStatus::Completed);
    assert!(!task.is_active());
    
    task.set_status(TaskStatus::Failed);
    assert!(!task.is_active());
    
    task.set_status(TaskStatus::Cancelled);
    assert!(!task.is_active());
}

#[test]
fn test_clone_preserves_status() {
    let original = TokioStatusEnabledTask::<String>::with_status(TaskStatus::Running);
    let cloned = original.clone();
    
    assert_eq!(original.status(), cloned.status());
    
    // Verify they operate independently after cloning
    original.mark_completed();
    cloned.mark_failed();
    
    assert_eq!(original.status(), TaskStatus::Completed);
    assert_eq!(cloned.status(), TaskStatus::Failed);
}

#[test]
fn test_debug_formatting() {
    let task = TokioStatusEnabledTask::<String>::with_status(TaskStatus::Running);
    let debug_str = format!("{:?}", task);
    
    assert!(debug_str.contains("TokioStatusEnabledTask"));
    assert!(debug_str.contains("Running"));
}

#[test]
fn test_status_trait_implementation() {
    let task = TokioStatusEnabledTask::<Vec<i32>>::new();
    
    // Test that StatusEnabledTask trait is properly implemented
    let status: &dyn StatusEnabledTask<Vec<i32>> = &task;
    assert_eq!(status.status(), TaskStatus::Pending);
    
    task.mark_running();
    assert_eq!(status.status(), TaskStatus::Running);
}