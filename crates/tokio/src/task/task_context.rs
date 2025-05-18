//! Task context implementation for Tokio
//!
//! This module provides the implementation for the contextual information associated
//! with tasks, including parent-child relationships and runtime environment.

use std::path::PathBuf;

use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::task::{ContextualizedTask, TaskId};

use crate::task::async_task::AsyncTask;
use crate::runtime::TokioRuntime;

// Implementation moved to async_task.rs

/// Extension trait for AsyncTask to provide additional context utilities
pub trait AsyncTaskContext<T: Clone + Send + 'static, I: TaskId> {
    /// Set a parent task for this task
    fn with_parent(self, parent: Box<dyn std::any::Any + Send + Sync>) -> Self;
    
    /// Add a child task to this task
    fn add_child(&self, child: Box<dyn std::any::Any + Send + Sync>);
    
    /// Get the runtime handle directly
    fn runtime_handle(&self) -> &tokio::runtime::Handle;
    
    /// Set the current working directory for the task
    fn with_cwd(self, path: PathBuf) -> Self;
    
    /// Set a name for the task
    fn with_name(self, name: String) -> Self;
    
    /// Get the task name if set
    fn name(&self) -> Option<String>;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskContext<T, I> for AsyncTask<T, I> {
    /// Set a parent task for this task
    fn with_parent(self, parent: Box<dyn std::any::Any + Send + Sync>) -> Self {
        futures::executor::block_on(async {
            let mut parent_lock = self.parent.lock().await;
            *parent_lock = Some(parent);
        });
        self
    }
    
    /// Add a child task to this task
    fn add_child(&self, child: Box<dyn std::any::Any + Send + Sync>) {
        futures::executor::block_on(async {
            let mut children = self.child_tasks.lock().await;
            children.push(child);
        });
    }
    
    /// Get the runtime handle directly
    fn runtime_handle(&self) -> &tokio::runtime::Handle {
        &self.runtime
    }
    
    /// Set the current working directory for the task
    fn with_cwd(mut self, path: PathBuf) -> Self {
        self.cwd = path;
        self
    }
    
    /// Set a name for the task
    fn with_name(self, name: String) -> Self {
        futures::executor::block_on(async {
            let mut task_name = self.name.lock().await;
            *task_name = Some(name);
        });
        self
    }
    
    /// Get the task name if set
    fn name(&self) -> Option<String> {
        futures::executor::block_on(async {
            let name = self.name.lock().await;
            name.clone()
        })
    }
}
