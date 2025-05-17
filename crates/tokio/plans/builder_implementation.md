# Builder Pattern Implementation Plan

This document outlines the implementation plan for the Builder pattern in the Tokio implementation of Sweet Async.

## Overview

The builder pattern is a core component of the Sweet Async API, providing a fluent interface for configuring and creating tasks. The current Tokio implementation does not include this pattern, which needs to be implemented to match the API's design.

## Reference Files

**API Definition Files (Read-Only):**
- `/crates/api/src/task/builder.rs` - Main AsyncTaskBuilder trait
- `/crates/api/src/task/spawn/builder.rs` - SpawningTaskBuilder for future-based tasks
- `/crates/api/src/task/emit/builder.rs` - EmittingTaskBuilder for stream-based tasks
- `/crates/api/src/macros/builder.rs` - Macros for builder creation

**Existing Tokio Implementation:**
- `/crates/tokio/src/task/tokio_task.rs` - Current TokioTask implementation

## Implementation Tasks

### 1. Create Core Builder Types

Create the following new files:
- `/crates/tokio/src/task/builder/mod.rs` - Module definition
- `/crates/tokio/src/task/builder/base_builder.rs` - Base builder implementation

The base builder should:
- Implement the AsyncTaskBuilder trait
- Store configuration like timeout, retry count, tracing enabled
- Use the immutable builder pattern (each method returns a new instance)

```markdown
Key methods to implement:
- `new()` - Create a new builder with default values
- `timeout(self, duration: Duration) -> Self` - Set task timeout
- `retry(self, attempts: u8) -> Self` - Set retry attempts
- `tracing(self, enabled: bool) -> Self` - Enable/disable tracing
```

### 2. Implement SpawningTaskBuilder

Create a new file:
- `/crates/tokio/src/task/spawn/builder.rs`

This builder should:
- Implement the SpawningTaskBuilder trait from the API
- Handle task creation for future-based workflows
- Create TokioTask instances when finalized

```markdown
Key methods to implement:
- `run<F, R>(self, work: F) -> Self::Task` - Create a task with the given work
  - Where F: AsyncWork<R> + Send + 'static, R: IntoAsyncResult<T, E> + Send + 'static
- `await_result<F, R>(self, work: F) -> impl Future<Output = Result<T, E>> + Send`
  - Create and immediately await a task
- `await_result_with_handler<F, R, H, Out>(self, work: F, handler: H) -> impl Future<Output = Out> + Send`
  - Create, await, and process with a handler
```

### 3. Implement EmittingTaskBuilder 

Create new files:
- `/crates/tokio/src/task/emit/builder.rs`
- `/crates/tokio/src/task/emit/event.rs`
- `/crates/tokio/src/task/emit/collector.rs`

This builder should:
- Implement the SenderBuilder and ReceiverBuilder traits
- Handle stream-based task creation 
- Support different processing strategies

```markdown
Key components:
- `TokioEventSender` - Manages event production
- `TokioEventReceiver` - Manages event consumption
- `TokioEventCollector` - Collects and aggregates events
```

### 4. Connect Builders to Runtime/Orchestrator

Update:
- `/crates/tokio/src/runtime.rs` to use builders
- `/crates/tokio/src/lib.rs` to expose builders

```markdown
Key changes:
- Create factory methods that return builders
- Ensure orchestrator works with builder-created tasks
```

### 5. Implement Static Builder Methods in TokioTask

Update:
- `/crates/tokio/src/task/tokio_task.rs`

```markdown
Implement in `AsyncTask` trait:
- `fn to<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>`
  - Returns a SpawningTaskBuilder for creating future-based tasks
- `fn emits<R: Send + 'static, Task: AsyncTask<R, I>>() -> impl OrchestratorBuilder<R, Task, I>`
  - Returns an EmittingTaskBuilder for creating stream-based tasks
```

### 6. Add Convenience Wrapper Methods

Create a new file:
- `/crates/tokio/src/builder.rs`

```markdown
Add convenience functions:
- `pub fn builder<T: Send + 'static, I: TaskId>() -> TokioTaskBuilder<T, I>`
- `pub fn spawning_builder<T: Send + 'static, I: TaskId>() -> TokioSpawningTaskBuilder<T, I>`
- `pub fn emitting_builder<T: Send + 'static, I: TaskId>() -> TokioEmittingTaskBuilder<T, I>`
```

## Implementation Notes

1. **Immutability**: All builder methods should return a new builder instance rather than modifying self.

2. **Type Parameters**: Preserve the type parameters from the API for maximum flexibility.

3. **Composition**: The implementation should compose the Tokio primitives in a way that matches the API behavior, without modifying the API itself.

4. **Error Handling**: Ensure proper error propagation through the builder methods.

5. **Testing**: Each builder component should be independently testable.

## Expected Outcome

A complete builder implementation that:
- Fully implements the API traits
- Maintains the same fluent interface
- Creates properly configured TokioTask instances
- Supports both future-based and stream-based workflows