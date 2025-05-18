# Parent-Child Relationship Implementation Plan

![Sweet Async Logo](/assets/sweet_async.png)

This document outlines the implementation plan for parent-child task relationships in the Tokio implementation of Sweet Async.

## Overview

Parent-child relationships are a core aspect of structured concurrency in the Sweet Async API. They enable hierarchical task management, resource lifecycle tracking, and coordinated cancellation. The current Tokio implementation stores parent-child relationships but does not properly implement their behavior.

## Reference Files

**API Definition Files (Read-Only):**
- `/crates/api/src/task/task_context.rs` - ContextualizedTask trait defining parent-child relationships
- `/crates/api/src/task/spawn/task.rs` - SpawningTask trait with child task operations

**Existing Tokio Implementation:**
- `/crates/tokio/src/task/tokio_task.rs` - Current AsyncTask implementation with stubs

## Implementation Tasks

### 1. Enhance AsyncTask Storage and Access

Update the AsyncTask structure in `/crates/tokio/src/task/tokio_task.rs`:

```markdown
Enhance storage:
- Update `child_tasks` field to store typed child tasks rather than `Box<dyn Any>`
- Use a more appropriate container like `HashMap<I, Arc<AsyncTask<T, I>>>` for faster lookups
- Ensure proper synchronization with Arc<Mutex<...>> 
- Store a strong reference to the runtime in each task
```

### 2. Implement ContextualizedTask Methods

Update the ContextualizedTask implementation for AsyncTask:

```markdown
Methods to implement:
- `fn child_tasks(&self) -> Vec<T>` - Return actual values from children
  - Extract and convert values from child task references
  - Handle possible failures gracefully

- `fn parent(&self) -> Option<T>` - Return the parent task value
  - Extract and convert value from parent reference
  - Handle type conversion safely

- `fn runtime(&self) -> &Self::RuntimeType`
  - Return the stored runtime reference
  - Ensure runtime outlives all tasks
```

### 3. Implement Child Task Creation

Update the SpawningTask implementation for AsyncTask:

```markdown
Methods to implement:
- `fn run_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture`
  - Create a new task with this task as parent
  - Register in child_tasks collection
  - Propagate context like working directory, timeout settings

Create helper methods:
- Typed access to child tasks
- Safe conversion between task types
- Child task lifecycle management
```

### 4. Implement Cancellation Propagation

Update the CancellableTask implementation:

```markdown
Enhance cancellation:
- When cancelling a task, recursively cancel all child tasks
- Implement proper cleanup when a parent is cancelled
- Maintain cancellation status consistency between parent and children
```

### 5. Implement Join Operations

Complete the join_children method:

```markdown
Implement:
- `fn join_children(&self) -> Self::JoinChildrenFuture`
  - Create a future that completes when all children complete
  - Propagate errors appropriately
  - Handle partial success cases
```

### 6. Add Parent-Child Resource Management

```markdown
Implement:
- Shared resource tracking between parent and child tasks
- Proper cleanup of shared resources
- Context inheritance (like working directory)
```

## Implementation Notes

1. **Type Safety**: Ensure proper typing of parent and child references.

2. **Memory Safety**: Prevent reference cycles between parents and children.

3. **Error Handling**: Propagate errors appropriately through the hierarchy.

4. **Cancellation**: Ensure cancellation propagates properly through the hierarchy.

5. **Testing**: Test complex parent-child scenarios including multiple nesting levels.

## Expected Outcome

A complete parent-child relationship implementation that:
- Provides accurate tracking of task hierarchies
- Supports structured concurrency patterns
- Enables resource sharing and lifecycle management
- Properly propagates cancellation through the hierarchy
- Supports joining multiple child tasks

![Book](/assets/book.png)
