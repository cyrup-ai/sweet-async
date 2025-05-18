# AsyncTask Implementation Completion PR

![Sweet Async Logo](/assets/sweet_async.png)

This document outlines the implementation plan for completing the AsyncTask implementation in the Tokio crate, which will build on the foundation established by the Builder Pattern PR.

## Overview

The AsyncTask trait is the core trait in the Sweet Async API, composed of several specialized traits that provide different capabilities. This PR will focus on properly implementing all the missing methods and traits required by AsyncTask, particularly the parent-child relationship methods and context handling.

## Implementation Plan

### 1. Update AsyncTask to Properly Store Context

First, we'll update the AsyncTask struct to properly store and manage runtime context:

```rust
// Add missing field for task name
name: Arc<Mutex<Option<String>>>,

// Update runtime to store a strong reference
runtime: Arc<TokioRuntime>, // Change from Handle to owned TokioRuntime
```

### 2. Complete ContextualizedTask Implementation

Update the ContextualizedTask implementation to properly handle parent-child relationships:

```rust
impl<T: Send + 'static, I: TaskId> ContextualizedTask<T, I> for AsyncTask<T, I> {
    type RuntimeType = crate::runtime::TokioRuntime;

    fn child_tasks(&self) -> Vec<T> {
        // Retrieve actual child task values
        futures::executor::block_on(async {
            let children = self.child_tasks.lock().await;
            let mut results = Vec::new();
            
            for child in children.iter() {
                // Try to downcast to the correct type
                if let Some(task) = child.downcast_ref::<Arc<AsyncTask<T, I>>>() {
                    // If the task has a value, add it to results
                    if let Some(value) = task.value() {
                        results.push(value.clone());
                    }
                }
            }
            
            results
        })
    }
    
    fn parent(&self) -> Option<T> {
        // Return the parent task's value if available
        futures::executor::block_on(async {
            let parent = self.parent.lock().await;
            if let Some(p) = parent.as_ref() {
                if let Some(task) = p.downcast_ref::<Arc<AsyncTask<T, I>>>() {
                    return task.value().cloned();
                }
            }
            None
        })
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        // Return reference to the runtime
        &self.runtime
    }
    
    fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }
}
```

### 3. Implement run_child Method

Update the SpawningTask implementation to add the `run_child` method:

```rust
impl<T: Send + 'static, I: TaskId> SpawningTask<T, I> for AsyncTask<T, I> {
    // Existing methods...
    
    fn run_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture
    where
        R: Send + 'static,
        Self: SpawningTask<R, I>
    {
        // Generate a task ID for the child
        let id = I::generate();
        
        // Create a builder with the same runtime and settings
        let builder = crate::task::spawn::builder::TokioSpawningTaskBuilder::<R, AsyncTaskError, I>::new(
            self.runtime.handle().clone(),
            self.active_tasks.clone()
        )
        .timeout(self.timeout)
        .retry(self.retry_count)
        .tracing(self.tracing_enabled);
        
        // Create the child task
        let child_task = builder.run(move || async move { task });
        
        // Set this task as the parent
        let self_arc = Arc::new(self.clone());
        child_task.with_parent(Box::new(self_arc));
        
        // Register as a child of this task
        let child_arc = Arc::new(child_task.clone());
        futures::executor::block_on(async {
            let mut children = self.child_tasks.lock().await;
            children.push(Box::new(child_arc));
        });
        
        // Return the child task's future
        Box::pin(child_task)
    }
    
    fn join_children(&self) -> Self::JoinChildrenFuture {
        // Create a future that waits for all children to complete
        let children = futures::executor::block_on(async {
            let children = self.child_tasks.lock().await;
            children.iter()
                .filter_map(|child| {
                    child.downcast_ref::<Arc<AsyncTask<T, I>>>()
                        .map(|task| task.clone())
                })
                .collect::<Vec<_>>()
        });
        
        // Return a future that completes when all children complete
        Box::pin(async move {
            let mut ids = Vec::new();
            let mut errors = Vec::new();
            
            for child in children {
                match child.await {
                    Ok(_) => ids.push(child.task_id()),
                    Err(e) => errors.push(e)
                }
            }
            
            if errors.is_empty() {
                Ok(ids)
            } else {
                Err(AsyncTaskError::Failure(format!("Child tasks failed: {:?}", errors)))
            }
        })
    }
    
    fn value(&self) -> Option<&T> {
        futures::executor::block_on(async {
            let result = self.result.lock().await;
            if let Some(ref res) = *result {
                if let Ok(ref value) = res {
                    return Some(value);
                }
            }
            None
        })
    }
}
```

### 4. Implement chain Method

Add the `chain` method to the SpawningTask implementation:

```rust
impl<T: Send + 'static, I: TaskId> SpawningTask<T, I> for AsyncTask<T, I> {
    // Other methods...
    
    fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture
    where
        F: AsyncWork<U> + Send + 'static,
        U: Send + 'static,
        Self: SpawningTask<U, I>
    {
        // Create a future that awaits this task and then runs the chained function
        Box::pin(async move {
            // Await this task
            match self.await {
                Ok(result) => {
                    // Run the chained function
                    let chained_result = f.run().await;
                    Ok(chained_result)
                }
                Err(e) => Err(e),
            }
        })
    }
}
```

### 5. Add Support for Task Result Access

Create a new helper method to support accessing task results:

```rust
impl<T: Send + 'static, I: TaskId> AsyncTask<T, I> {
    // Other methods...
    
    /// Get the current task result if available
    pub fn result(&self) -> Option<Result<T, AsyncTaskError>> {
        futures::executor::block_on(async {
            let result = self.result.lock().await;
            result.clone()
        })
    }
    
    /// Get the task ID
    pub fn id(&self) -> I {
        self.id
    }
    
    /// Get the task name if set
    pub fn name(&self) -> Option<String> {
        futures::executor::block_on(async {
            let name = self.name.lock().await;
            name.clone()
        })
    }
}
```

### 6. Implement Proper Cancellation Propagation

Update the cancellation implementation to properly propagate to child tasks:

```rust
impl<T: Send + 'static, I: TaskId> CancellableTask<T> for AsyncTask<T, I> {
    // Existing cancellation code...
    
    async fn cancel(&self, level: CancellationLevel) -> Result<(), OrchestratorError> {
        // Update status
        {
            let mut status = self.status.lock().await;
            *status = TaskStatus::PendingCancellation;
        }
        
        // Send cancellation signal
        let cancel_tx = {
            let mut cancel_tx = self.cancel_tx.lock().await;
            cancel_tx.take()
        };
        
        if let Some(tx) = cancel_tx {
            let _ = tx.send(level);
        }
        
        // If KillHard, abort the task
        if matches!(level, CancellationLevel::KillHard) {
            let handle = {
                let mut handle = self.handle.lock().await;
                handle.take()
            };
            
            if let Some(handle) = handle {
                handle.abort();
            }
        }
        
        // Cancel all child tasks
        let children = {
            let children = self.child_tasks.lock().await;
            children.iter()
                .filter_map(|child| {
                    child.downcast_ref::<Arc<AsyncTask<T, I>>>()
                        .map(|task| task.clone())
                })
                .collect::<Vec<_>>()
        };
        
        for child in children {
            let _ = child.cancel(level).await;
        }
        
        // Update status
        {
            let mut status = self.status.lock().await;
            *status = TaskStatus::Cancelled;
        }
        
        // Execute cancellation callbacks
        let callbacks = {
            let callbacks = self.cancel_callbacks.lock().await;
            callbacks.iter().map(|f| f()).collect::<Vec<_>>()
        };
        
        for callback in callbacks {
            let _ = callback.await;
        }
        
        Ok(())
    }
    
    // Other cancellation methods...
}
```

### 7. Add Tests

Create a new test file:

**`tests/task_hierarchy_tests.rs`**:
```rust
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sweet_async_api::task::{AsyncTask, TaskId, TaskStatus, CancellableTask};
use sweet_async_api::task::spawn::SpawningTask;
use sweet_async_tokio::builder;
use tokio_test::block_on;

// Simple test ID implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TestTaskId(u64);

impl TaskId for TestTaskId {
    fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TestTaskId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn to_string(&self) -> String {
        format!("TestTask-{}", self.0)
    }
}

#[test]
fn test_parent_child_relationship() {
    // Create a parent task
    let parent = builder::spawning_builder::<String, TestTaskId>()
        .run(|| async { "Parent task".to_string() });
    
    // Create a child task
    let child_ran = Arc::new(AtomicBool::new(false));
    let child_ran_clone = child_ran.clone();
    
    let child_future = parent.run_child(async move {
        child_ran_clone.store(true, Ordering::SeqCst);
        "Child task".to_string()
    });
    
    // Complete both tasks
    let _ = block_on(parent.clone());
    let child_result = block_on(child_future);
    
    // Verify child ran and returned correct result
    assert!(child_ran.load(Ordering::SeqCst));
    assert_eq!(child_result.unwrap(), "Child task");
    
    // Verify parent-child relationship
    let children = parent.child_tasks();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0], "Child task");
}

#[test]
fn test_join_children() {
    // Create a parent task
    let parent = builder::spawning_builder::<String, TestTaskId>()
        .run(|| async { "Parent task".to_string() });
    
    // Create three child tasks
    for i in 1..=3 {
        let _ = parent.run_child(async move {
            tokio::time::sleep(Duration::from_millis(50 * i as u64)).await;
            format!("Child task {}", i)
        });
    }
    
    // Join all children
    let join_result = block_on(parent.join_children());
    
    // Verify all children completed
    assert!(join_result.is_ok());
    assert_eq!(join_result.unwrap().len(), 3);
}

#[test]
fn test_cancel_propagation() {
    // Create a parent task with a long-running operation
    let parent = builder::spawning_builder::<String, TestTaskId>()
        .run(|| async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            "Parent completed".to_string()
        });
    
    // Create child tasks
    let child_cancelled = Arc::new(AtomicBool::new(false));
    let child_cancelled_clone = child_cancelled.clone();
    
    let _child = parent.run_child(async move {
        // Set up cancellation detection
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                "Child completed".to_string()
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                child_cancelled_clone.store(true, Ordering::SeqCst);
                "Child cancelled".to_string()
            }
        }
    });
    
    // Cancel the parent
    block_on(parent.cancel(sweet_async_api::task::CancellationLevel::Graceful)).unwrap();
    
    // Verify parent is cancelled
    assert_eq!(parent.status(), TaskStatus::Cancelled);
    
    // Verify child was also cancelled
    assert!(child_cancelled.load(Ordering::SeqCst));
}

#[test]
fn test_task_chain() {
    // Create a task that will be chained
    let task1 = builder::spawning_builder::<u32, TestTaskId>()
        .run(|| async { 42 });
    
    // Chain another task that uses the result
    let chain_future = task1.chain(|| async { 
        "Chained task completed".to_string()
    });
    
    // Execute the chain
    let result = block_on(chain_future);
    
    // Verify chain completed
    assert_eq!(result.unwrap(), "Chained task completed");
}

#[test]
fn test_multiple_child_tasks() {
    // Create a parent task
    let parent = builder::spawning_builder::<String, TestTaskId>()
        .run(|| async { "Parent task".to_string() });
    
    // Create 10 child tasks
    let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
    
    for i in 0..10 {
        let counter_clone = counter.clone();
        let _child = parent.run_child(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            format!("Child {}", i)
        });
    }
    
    // Join all children
    let _ = block_on(parent.join_children());
    
    // Verify all children ran
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}
```

## Implementation Notes

1. This PR builds on the Builder Pattern PR foundation
2. It focuses on implementing the missing methods in the AsyncTask trait
3. Parent-child relationships are key to structured concurrency
4. The chain method enables functional composition of tasks
5. Proper cancellation propagation is essential for resource cleanup

## Expected Outcome

After this PR:

1. Users will be able to create hierarchical task structures
2. Parent tasks can spawn child tasks that inherit their context
3. Cancellation will propagate properly through the task hierarchy
4. Task chaining will allow composition of async operations
5. All essential AsyncTask trait methods will be fully implemented

![Book](/assets/book.png)
