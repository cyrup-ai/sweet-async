//! Task context implementation for Tokio
//!
//! This module provides the implementation for the contextual information associated
//! with tasks, including parent-child relationships and runtime environment.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::orchestra::runtime::Runtime;
use sweet_async_api::task::{ContextualizedTask, TaskId};

use crate::task::async_task::AsyncTask;
use crate::runtime::TokioRuntime;

// Implementation moved to async_task.rs

/// Extension trait for AsyncTask to provide additional context utilities
/// 
/// Note: Methods that need async operations now provide both sync and async versions
pub trait AsyncTaskContext<T: Clone + Send + 'static, I: TaskId> {
    /// Set a parent task for this task
    fn with_parent(self, parent: Box<dyn std::any::Any + Send + Sync>) -> Self;
    
    /// Add a child task to this task (async version)
    fn add_child_async(&self, child: Box<dyn std::any::Any + Send + Sync>) -> Pin<Box<dyn Future<Output = ()> + Send>>;
    
    /// Get the runtime handle directly
    fn runtime_handle(&self) -> &tokio::runtime::Handle;
    
    /// Set the current working directory for the task
    fn with_cwd(self, path: PathBuf) -> Self;
    
    /// Set a name for the task
    fn with_name(self, name: String) -> Self;
    
    /// Get the task name if set (async version)
    fn name_async(&self) -> Pin<Box<dyn Future<Output = Option<String>> + Send>>;
}

impl<T: Clone + Send + 'static, I: TaskId> AsyncTaskContext<T, I> for AsyncTask<T, I> {
    /// Set a parent task for this task
    /// Uses a detached task to avoid blocking
    fn with_parent(self, parent: Box<dyn std::any::Any + Send + Sync>) -> Self {
        let parent_clone = Arc::clone(&self.parent);
        
        // Spawn a detached task to set the parent
        tokio::spawn(async move {
            let mut parent_lock = parent_clone.lock().await;
            *parent_lock = Some(parent);
        });
        
        self
    }
    
    /// Add a child task to this task (async version)
    fn add_child_async(&self, child: Box<dyn std::any::Any + Send + Sync>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let children_clone = Arc::clone(&self.child_tasks);
        
        Box::pin(async move {
            let mut children = children_clone.lock().await;
            children.push(child);
        })
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
    /// Uses a detached task to avoid blocking
    fn with_name(self, name: String) -> Self {
        let name_clone = Arc::clone(&self.name);
        
        // Spawn a detached task to set the name
        tokio::spawn(async move {
            let mut task_name = name_clone.lock().await;
            *task_name = Some(name);
        });
        
        self
    }
    
    /// Get the task name if set (async version)
    fn name_async(&self) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> {
        let name_clone = Arc::clone(&self.name);
        
        Box::pin(async move {
            let name = name_clone.lock().await;
            name.clone()
        })
    }
}
