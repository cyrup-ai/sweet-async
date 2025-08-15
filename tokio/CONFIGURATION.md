# Sweet Async Tokio Configuration Reference

This document provides a complete reference for all configuration options available when building AsyncTask instances in the Tokio implementation.

## Core Builder Configuration (AsyncTaskBuilder)

These methods are available on all task builders and configure fundamental task behavior.

### `timeout(duration: Duration)`
Sets the maximum execution time for the task. If the task doesn't complete within this duration, it will be cancelled and return a timeout error.

```rust
AsyncTask::to::<String>()
    .timeout(Duration::from_secs(30))
    .run(|| async { /* work */ })
```

### `retry(attempts: u8)`
Configures automatic retry on failure. The task will be retried up to the specified number of attempts before returning the final error.

```rust
AsyncTask::to::<Data>()
    .retry(3)  // Retry up to 3 times on failure
    .run(|| async { /* potentially failing work */ })
```

### `tracing(enabled: bool)`
Enables or disables detailed execution tracing. When enabled, the task will emit tracing events for lifecycle events and errors.

```rust
AsyncTask::to::<Result>()
    .tracing(true)  // Enable detailed tracing
    .run(|| async { /* work to trace */ })
```

### `name(name: &str)`
Sets a descriptive name for the task, useful for debugging and monitoring.

```rust
AsyncTask::to::<Output>()
    .name("data-processor")
    .run(|| async { /* processing logic */ })
```

## Spawning Task Specific Configuration

### `parent(relationships: TaskRelationships)`
Establishes parent-child relationships between tasks, enabling hierarchical task communication.

```rust
let parent_task = AsyncTask::to::<ParentResult>()
    .run(|| async { /* parent work */ });

let child_task = AsyncTask::to::<ChildResult>()
    .parent(parent_task.relationships())
    .run(|| async { /* child work */ });
```

## Task Trait Implementations

The following traits are implemented by all tasks and provide additional capabilities:

### 1. NamedTask
Provides task naming and identification capabilities.

**Methods:**
- `name() -> Option<&str>` - Get the task's name
- `set_name(name: String)` - Set the task's name

### 2. TimedTask
Tracks task execution timing and enforces timeouts.

**Methods:**
- `created_timestamp() -> SystemTime` - When the task was created
- `executed_timestamp() -> SystemTime` - When execution began
- `completed_timestamp() -> SystemTime` - When the task completed
- `timeout() -> Duration` - Get configured timeout duration

### 3. TracingTask
Enables detailed execution tracing and error recording.

**Methods:**
- `handle_error(error: AsyncTaskError) -> Result<T, AsyncTaskError>` - Handle errors with tracing
- `record_error(error: &AsyncTaskError)` - Record error for diagnostics
- `is_tracing_enabled() -> bool` - Check if tracing is enabled

### 4. CancellableTask
Provides task cancellation with different urgency levels.

**Methods:**
- `cancel(level: CancellationLevel) -> Future<Output = Result<(), OrchestratorError>>` - Cancel with specified level
- `is_cancelled() -> bool` - Check if task has been cancelled
- `on_cancel<F>(callback: F) -> Self` - Register cancellation callback
- `cancel_gracefully()` - Request graceful shutdown
- `cancel_forcefully()` - Force immediate termination
- `cancel_immediately()` - Kill without cleanup

**Cancellation Levels:**
- `Graceful` - Allow task to complete current operation
- `Kill` - Terminate as soon as possible
- `KillHard` - Immediate termination without cleanup

### 5. RecoverableTask
Implements retry logic and fallback strategies for resilient execution.

**Methods:**
- `recover(error: AsyncTaskError) -> Future<Output = Result<T, AsyncTaskError>>` - Attempt recovery
- `can_recover_from(error: &AsyncTaskError) -> bool` - Check if error is recoverable
- `fallback_work() -> &FallbackWork` - Get fallback work to execute
- `max_retries() -> u8` - Maximum retry attempts
- `current_retry() -> u8` - Current retry attempt number
- `retry_strategy() -> RetryStrategy` - Get configured retry strategy

**Retry Strategies:**
- `Exponential(base_delay)` - Exponential backoff (2^n * base_delay)
- `Linear(increment)` - Linear backoff (n * increment)
- `Fixed(delay)` - Fixed delay between retries
- `Immediate` - No delay between retries

### 6. MetricsEnabledTask
Collects execution metrics for monitoring and optimization.

**Methods:**
- `cpu_usage() -> &CpuUsage` - CPU utilization metrics
- `memory_usage() -> &MemoryUsage` - Memory consumption metrics
- `io_usage() -> &IoUsage` - I/O operation metrics

### 7. PrioritizedTask
Manages task execution priority for scheduling.

**Methods:**
- `priority() -> &TaskPriority` - Get task priority

**Priority Levels:**
- `Critical` - Highest priority, execute immediately
- `High` - High priority execution
- `Normal` - Standard priority (default)
- `Low` - Low priority, can be deferred
- `Idle` - Only execute when no other work

### 8. StatusEnabledTask
Tracks task execution status throughout its lifecycle.

**Methods:**
- `status() -> TaskStatus` - Get current task status

**Status Values:**
- `Pending` - Task created but not started
- `Running` - Currently executing
- `Completed` - Successfully finished
- `Failed` - Terminated with error
- `Cancelled` - Cancelled before completion

## Complete Configuration Example

```rust
use sweet_async::AsyncTask;
use std::time::Duration;

// Fully configured task with all options
let result = AsyncTask::to::<ProcessedData>()
    // Core configuration
    .name("critical-processor")
    .timeout(Duration::from_secs(60))
    .retry(5)
    .tracing(true)
    
    // Priority configuration
    .priority(TaskPriority::High)
    
    // Recovery configuration
    .with_retry_strategy(RetryStrategy::Exponential(Duration::from_millis(100)))
    .with_fallback(|| async {
        // Fallback logic if all retries fail
        ProcessedData::default()
    })
    
    // Cancellation configuration
    .on_cancel(|| async {
        // Cleanup on cancellation
        cleanup_resources().await;
    })
    
    // Execute the task
    .run(|| async {
        // Main task logic
        process_critical_data().await
    })
    .await;

// Check task status and metrics
if let Ok(data) = result {
    println!("Task completed successfully");
    println!("CPU usage: {:?}", task.cpu_usage());
    println!("Memory usage: {:?}", task.memory_usage());
}
```

## Action Methods

These methods trigger task execution:

### `run<F, R>(work: F) -> Task`
Creates and returns a Future that can be awaited by the user.

### `await_result<F, R>(work: F, handler: H) -> Result<T, E>`
Executes the task internally and returns the result synchronously (but execution is still async).

### `await_result_with_handler<F, R, H, Out>(work: F, handler: H) -> Out`
Executes the task and applies a handler function to transform the result.

## Performance Considerations

- **Zero Allocation**: Configuration is stored inline in task structures where possible
- **Lock-Free Polling**: The `await_result` methods use lock-free polling with adaptive backoff
- **Atomic Operations**: Status and metrics updates use atomic operations for thread safety
- **Lazy Initialization**: Orchestra and runtime are created only when needed