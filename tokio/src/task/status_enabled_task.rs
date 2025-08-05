//! Tokio implementation of StatusEnabledTask trait
//!
//! This module provides status tracking capabilities for tasks with
//! thread-safe atomic status updates.

use std::sync::atomic::{AtomicU8, Ordering};
use sweet_async_api::task::task_status::{StatusEnabledTask, TaskStatus};
use sweet_async_api::task::TaskId;

/// Tokio implementation of StatusEnabledTask
/// 
/// Provides thread-safe status tracking using atomic operations.
#[derive(Debug)]
pub struct TokioStatusEnabledTask<T: Send + 'static, I: TaskId> {
    task_id: I,
    status: AtomicU8,
    name: String,
    created_at: std::time::Instant,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static, I: TaskId> TokioStatusEnabledTask<T, I> {
    /// Create a new status-enabled task
    pub fn new(task_id: I, name: String) -> Self {
        Self {
            task_id,
            status: AtomicU8::new(0), // Pending
            name,
            created_at: std::time::Instant::now(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create a new status-enabled task with initial status
    pub fn with_status(task_id: I, name: String, initial_status: TaskStatus) -> Self {
        Self {
            task_id,
            status: AtomicU8::new(status_to_u8(initial_status)),
            name,
            created_at: std::time::Instant::now(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Update the task status atomically
    pub fn set_status(&self, new_status: TaskStatus) {
        self.status.store(status_to_u8(new_status), Ordering::Relaxed);
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
    
    /// Mark the task as running
    pub fn mark_running(&self) {
        self.set_status(TaskStatus::Running);
    }
    
    /// Mark the task as completed
    pub fn mark_completed(&self) {
        self.set_status(TaskStatus::Completed);
    }
    
    /// Mark the task as failed
    pub fn mark_failed(&self) {
        self.set_status(TaskStatus::Failed);
    }
    
    /// Mark the task as cancelled
    pub fn mark_cancelled(&self) {
        self.set_status(TaskStatus::Cancelled);
    }
    
    /// Mark the task as pending cancellation
    pub fn mark_pending_cancellation(&self) {
        self.set_status(TaskStatus::PendingCancellation);
    }
    
    /// Check if the task is in a terminal state (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(self.status(), 
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled)
    }
    
    /// Check if the task is active (running or pending)
    pub fn is_active(&self) -> bool {
        matches!(self.status(), 
            TaskStatus::Pending | TaskStatus::Running | TaskStatus::PendingCancellation)
    }
}

impl<T: Send + 'static, I: TaskId> Clone for TokioStatusEnabledTask<T, I> {
    fn clone(&self) -> Self {
        Self {
            task_id: self.task_id,
            status: AtomicU8::new(self.status.load(Ordering::Relaxed)),
            name: self.name.clone(),
            created_at: self.created_at,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Send + 'static, I: TaskId> StatusEnabledTask<T> for TokioStatusEnabledTask<T, I> {
    fn status(&self) -> TaskStatus {
        let status_u8 = self.status.load(Ordering::Relaxed);
        TaskStatus::from_u8(status_u8)
    }
}

/// Convert TaskStatus to u8 for atomic storage
fn status_to_u8(status: TaskStatus) -> u8 {
    match status {
        TaskStatus::Pending => 0,
        TaskStatus::Running => 1,
        TaskStatus::Completed => 2,
        TaskStatus::Failed => 3,
        TaskStatus::PendingCancellation => 4,
        TaskStatus::Cancelled => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TokioTaskId;
    
    #[test]
    fn test_status_transitions() {
        let task_id = TokioTaskId::new();
        let task = TokioStatusEnabledTask::<String, _>::new(task_id, "test_task".to_string());
        
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
        let task_id = TokioTaskId::new();
        let task = TokioStatusEnabledTask::<String, _>::new(task_id, "test_task".to_string());
        
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
        use std::sync::Arc;
        use std::thread;
        
        let task_id = TokioTaskId::new();
        let task = Arc::new(TokioStatusEnabledTask::<String, _>::new(task_id, "test_task".to_string()));
        
        let task_clone = Arc::clone(&task);
        let handle = thread::spawn(move || {
            task_clone.mark_running();
            task_clone.mark_completed();
        });
        
        handle.join().unwrap();
        assert!(matches!(task.status(), TaskStatus::Completed));
    }
}