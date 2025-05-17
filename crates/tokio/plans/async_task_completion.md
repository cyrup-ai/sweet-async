# AsyncTask Implementation Completion Plan

This document outlines the plan for completing the AsyncTask implementation in the Tokio crate of Sweet Async.

## Overview

The `AsyncTask` trait is the core abstraction in the Sweet Async API, composed of seven specialized traits that provide different capabilities. While most trait methods have implementations in the Tokio crate, several key methods are stubbed with `unimplemented!()` and need proper implementation.

## Reference Files

**API Definition Files (Read-Only):**
- `/crates/api/src/task/async_task.rs` - Main AsyncTask trait definition
- `/crates/api/src/task/spawn/task.rs` - SpawningTask trait
- `/crates/api/src/task/emit/task.rs` - EmittingTask trait
- `/crates/api/src/orchestra/orchestrator_builder.rs` - OrchestratorBuilder trait

**Existing Tokio Implementation:**
- `/crates/tokio/src/task/tokio_task.rs` - Current TokioTask implementation

## Implementation Tasks

### 1. Complete the AsyncTask Trait Methods

Update the TokioTask implementation in `/crates/tokio/src/task/tokio_task.rs` to implement the static methods:

```markdown
Methods to implement:
- `fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>`
  - This static method should create an OrchestratorBuilder that can build future-based tasks
  - It corresponds to the "await" workflow pattern
  
- `fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>`
  - This static method should create an OrchestratorBuilder that can build stream-based tasks
  - It corresponds to the "event processing" workflow pattern
```

### 2. Create OrchestratorBuilder Implementation

Create a new file:
- `/crates/tokio/src/orchestra/builder.rs`

```markdown
Components to implement:
- `TokioOrchestratorBuilder<T, Task, I>` struct that implements OrchestratorBuilder
  - Should be able to construct both types of task builders (spawning and emitting)
  - Should connect to the existing TokioOrchestrator
- Factory methods to simplify creation
```

### 3. Complete the SpawningTask Trait Methods

Update the TokioTask implementation to complete these methods:

```markdown
Methods to implement:
- `fn run_child<R>(&self, task: R) -> <Self as SpawningTask<R, I>>::OutputFuture`
  - Creates a child task that inherits properties from the parent
  - Establishes parent-child relationship

- `fn join_children(&self) -> Self::JoinChildrenFuture`
  - Returns a future that completes when all child tasks have completed
  - Should respect task cancellation propagation

- `fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture`
  - Creates a new task that executes after this task completes
  - Takes result from current task as input to the next
```

### 4. Implement Type Conversions and Task Identity

```markdown
Create:
- Proper type conversions between tasks and futures
- Consistent task identity propagation between parent and child tasks
- Task result wrapping that preserves both result and task identity
```

### 5. Implement Missing ContextualizedTask Methods

```markdown
Fix:
- `fn runtime(&self) -> &Self::RuntimeType`
  - Currently returns unimplemented!()
  - Should return a reference to the runtime that created this task

- `fn child_tasks(&self) -> Vec<T>`
  - Currently returns an empty Vec
  - Should return actual child task values

- `fn parent(&self) -> Option<T>`
  - Currently returns None
  - Should return the parent task if it exists
```

### 6. Add Support for Task Chaining

```markdown
Create:
- Implement appropriate Future combinators
- Ensure proper error propagation
- Maintain task context throughout the chain
```

## Implementation Notes

1. **Task Identity**: Ensure consistent task IDs throughout parent-child relationships and chains.

2. **Error Handling**: Properly propagate errors through child tasks and chains.

3. **Resource Management**: Ensure proper cleanup of resources when tasks complete or are cancelled.

4. **Immutability**: Maintain immutability consistent with the API design.

5. **Testing**: Each component should be independently testable.

## Expected Outcome

A complete AsyncTask implementation that:
- Provides both future-based and stream-based task creation
- Supports hierarchical task relationships
- Allows task chaining and composition
- Properly integrates with the OrchestratorBuilder pattern
- Maintains all the contextual information expected by the API