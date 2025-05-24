//! Spawning task implementation for Tokio
//!
//! This module provides the implementation for tasks that execute once and return a result.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::spawn::{AsyncResult, SpawningTask, TaskResult};

use crate::task::async_task::AsyncTask;
use crate::task::spawn::result::AsyncTaskResult;

/// Implementation of SpawningTask for AsyncTask
impl<T: Clone + Send + 'static, I: TaskId> SpawningTask<T, I> for AsyncTask<T, I> {
    type TaskResult = Result<T, AsyncTaskError>;
    type OutputFuture = Pin<Box<dyn Future<Output = Self::TaskResult> + Send>>;
    type JoinChildrenFuture = Pin<Box<dyn Future<Output = Self::JoinChildrenResult> + Send + 'static>>;
    type JoinChildrenResult = Result<Vec<I>, AsyncTaskError>;
    type AsyncWork = Box<dyn Fn() -> T + Send + 'static>;
    
    fn task_id(&self) -> I {
        // Return the task's unique identifier
        self.id
    }

    fn id(&self) -> I {
        // Alias for task_id
        self.task_id()
    }

    fn run(self, work: Self::AsyncWork) -> Self {
        // Create a future from the work function
        let future = async move {
            match tokio::task::spawn_blocking(move || work()).await {
                Ok(result) => Ok(result),
                Err(err) => Err(AsyncTaskError::Failure(format!("Task execution failed: {}", err))),
            }
        };
        
        // Attach the future to this task
        let boxed_future: Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>> = Box::pin(future);
        self.with_future(boxed_future)
    }

    fn run_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Send + 'static,
        Self: SpawningTask<R, I>
    {
        // In a complete implementation, this would create and track a child task
        // in the parent's child_tasks registry, then execute it.
        
        // For now, we'll return a future that immediately fails
        Box::pin(async move {
            Err(AsyncTaskError::Failure("Child task execution not fully implemented".to_string()))
        })
    }

    fn join_children(&self) -> Self::JoinChildrenFuture {
        // In a complete implementation, this would wait for all registered
        // child tasks to complete and collect their results
        
        // For now, we'll return a future that returns an empty vec
        Box::pin(async move {
            Ok(Vec::new())
        })
    }

    fn value(&self) -> Option<&T> {
        // Check atomic flags first to see if result is available
        if !self.atomic_result_available.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        
        if !self.atomic_result_success.load(std::sync::atomic::Ordering::Relaxed) {
            return None; // Result available but failed
        }
        
        // Try to get the result without blocking
        match self.result.try_lock() {
            Ok(result) => {
                if let Some(Ok(ref value)) = *result {
                    Some(value)
                } else {
                    None
                }
            }
            Err(_) => None, // If locked, return None
        }
    }

    fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: AsyncWork<U> + Send + 'static,
        U: Send + 'static,
        Self: SpawningTask<U, I>
    {
        // Create a future that awaits this task and then runs the next function
        Box::pin(async move {
            // First, await the current task
            let result = self.await.await;
            
            // If successful, run the next function
            match result {
                Ok(value) => {
                    // First create a context with the previous value
                    let context = Arc::new(value);
                    
                    // Then run the next function with this context
                    let next_future = f.run();
                    
                    // Return the result of the next function
                    match next_future.await {
                        Ok(next_value) => Ok(next_value),
                        Err(err) => Err(AsyncTaskError::Failure(format!("Chain execution failed: {}", err))),
                    }
                },
                Err(err) => Err(err),
            }
        })
    }
}

/// Extension trait for AsyncTask to implement into_future
impl<T: Clone + Send + 'static, I: TaskId> AsyncTask<T, I> {
    /// Convert this task to a future that can be awaited
    pub fn into_future(self) -> Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>> {
        // Simply return self as a future
        Box::pin(self)
    }
    
    /// Create a simple AsyncTask from a closure 
    ///
    /// This is a simpler alternative to using the builder directly
    ///
    /// Note: Currently disabled until the builder pattern is fully implemented
    #[allow(dead_code)]
    pub fn spawn<F, E>(f: F) -> AsyncTask<T, I>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: std::fmt::Debug + Send + 'static,
    {
        unimplemented!("spawn is not yet implemented - will be added in a future PR after the builder pattern is completed")
    }
    
    /// Spawn a task with a timeout
    ///
    /// This is a simpler alternative to using the builder directly
    ///
    /// Note: Currently disabled until the builder pattern is fully implemented
    #[allow(dead_code)]
    pub fn spawn_with_timeout<F, E>(f: F, timeout: Duration) -> AsyncTask<T, I>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: std::fmt::Debug + Send + 'static,
    {
        unimplemented!("spawn_with_timeout is not yet implemented - will be added in a future PR after the builder pattern is completed")
    }
}
