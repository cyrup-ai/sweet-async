# Task Chaining Implementation Plan

![Sweet Async Logo](/assets/sweet_async.png)

This document outlines the implementation plan for task chaining in the Tokio implementation of Sweet Async.

## Overview

Task chaining is a powerful feature of the Sweet Async API that allows for composition of asynchronous tasks. It enables a functional programming style where tasks can be sequenced, with each task taking the result of the previous task as input. The current Tokio implementation has a stubbed implementation of the `chain()` method that needs to be implemented fully.

## Reference Files

**API Definition Files (Read-Only):**
- `/crates/api/src/task/spawn/task.rs` - SpawningTask trait with chain method
- `/crates/api/src/task/spawn/into_async_result.rs` - Type conversions for task results

**Existing Tokio Implementation:**
- `/crates/tokio/src/task/tokio_task.rs` - Current TokioTask implementation with stubs

## Implementation Tasks

### 1. Implement Task Chaining Method

Update the TokioTask struct in `/crates/tokio/src/task/tokio_task.rs` to implement the chain method:

```markdown
Method to implement:
- `fn chain<U, F>(self, f: F) -> <Self as SpawningTask<U, I>>::OutputFuture`
  - Where F: AsyncWork<U> + Send + 'static, U: Send + 'static
  - Takes the result of the current task as input for the next task
  - Returns a future representing the combined operation
```

### 2. Create TokioTaskChain Structure

Create a new file `/crates/tokio/src/task/spawn/chain.rs`:

```markdown
Create:
- `TokioTaskChain<T, U, I>` struct that holds the chained tasks
  - First task of type TokioTask<T, I>
  - Second task (the continuation) of type F where F: AsyncWork<U>
  - Propagates task identity, context, and other properties
```

### 3. Implement Future Trait for Chained Tasks

```markdown
Implement:
- `Future` trait for `TokioTaskChain`
  - Poll first task until completion
  - On completion, poll second task with first task's result
  - Properly handle task cancellation
  - Propagate errors appropriately
```

### 4. Add Task Result Conversion

```markdown
Create:
- Helper methods for converting between different result types
- Functions for transforming errors between tasks
- Type-safe wrappers that preserve task identity throughout the chain
```

### 5. Implement Error Handling for Chains

```markdown
Implement:
- Chain-aware error handling strategies
- Option to short-circuit on error vs. continue with default values
- Error mapping between different task error types
```

### 6. Create Builder Methods for Chains

Update `/crates/tokio/src/task/spawn/builder.rs`:

```markdown
Add:
- Builder methods specific to task chains
- Configuration options for chain execution
- Methods to add conditional branches and error handlers to chains
```

### 7. Add Support for Multiple Chained Tasks

```markdown
Implement:
- Support for more than two tasks in a chain
- Methods for composing multiple chains together
- Optimization for reducing overhead in long chains
```

## Implementation Notes

1. **Type Safety**: Ensure type parameters are properly handled throughout the chain.

2. **Error Propagation**: Create clear paths for error propagation between chained tasks.

3. **Cancellation**: Ensure cancellation properly propagates through the chain.

4. **Performance**: Minimize overhead for chained operations.

5. **Backpressure**: Consider backpressure for chains processing streams of data.

## Task Chaining Patterns to Support

1. **Simple Chaining**:
   ```rust
   task1.chain(|result| process_result(result))
   ```

2. **Chain with Error Handling**:
   ```rust
   task1.chain(|result| match result {
       Ok(val) => process_success(val),
       Err(e) => handle_error(e),
   })
   ```

3. **Multi-Task Chains**:
   ```rust
   task1.chain(|r1| task2(r1))
        .chain(|r2| task3(r2))
   ```

4. **Conditional Chains**:
   ```rust
   task1.chain(|result| {
       if should_continue(result) {
           next_task(result)
       } else {
           alternate_task(result)
       }
   })
   ```

## Expected Outcome

A complete task chaining implementation that:
- Provides natural composition of asynchronous tasks
- Handles errors properly throughout the chain
- Maintains task identity and context
- Respects task cancellation
- Offers a clear and ergonomic API

![Book](/assets/book.png)