# Event Processing Implementation Plan

![Sweet Async Logo](/assets/sweet_async.png)

This document outlines the implementation plan for event processing in the Tokio implementation of Sweet Async.

## Overview

Event processing is a core feature of the Sweet Async API, providing a stream-based alternative to future-based task execution. It allows for continuous processing of data streams with different strategies (serial, parallel, batched) and rich event collection capabilities. The current Tokio implementation does not include any event processing functionality.

## Reference Files

**API Definition Files (Read-Only):**
- `/crates/api/src/task/emit/task.rs` - EmittingTask, SenderTask, and ReceiverTask traits
- `/crates/api/src/task/emit/event.rs` - Event interfaces and collector abstraction
- `/crates/api/src/task/emit/builder.rs` - Builder pattern for event-based tasks
- `/crates/api/src/task/builder.rs` - ReceiverStrategy and SenderStrategy definitions

**Existing Tokio Implementation:**
- `/crates/tokio/src/task/tokio_task.rs` - Current TokioTask implementation with no event processing

## Implementation Tasks

### 1. Create Base Event Types

Create new files:
- `/crates/tokio/src/task/emit/mod.rs` - Module definition
- `/crates/tokio/src/task/emit/event.rs` - Event implementations

```markdown
Implement:
- `TokioEvent<T>` - Base event implementation
  - Implements StreamingEvent<T>
  - Stores timestamps, IDs, data, and event type
  
- `TokioSenderEvent<T>` - Event for sending
  - Implements SenderEvent<T>
  - Includes builder implementation
  
- `TokioReceiverEvent<T, C>` - Event for receiving
  - Implements ReceiverEvent<T, C>
  - Includes data extraction and processing methods
  
- `TokioFinalEvent<T, C, F>` - Final event in a sequence
  - Implements FinalEvent<T, C, F>
  - Includes completion semantics
```

### 2. Implement Event Collector

Create a new file:
- `/crates/tokio/src/task/emit/collector.rs`

```markdown
Implement:
- `TokioEventCollector<T, C>` - Collection mechanism
  - Implements Collector<T, C>
  - Thread-safe collection with Arc<Mutex<...>>
  - Support for different collection strategies
  - Methods for collecting and retrieving items
```

### 3. Create Event Processing Task

Create a new file:
- `/crates/tokio/src/task/emit/task.rs`

```markdown
Implement:
- `TokioEmittingTask<T, C, E, I>` - Task for event processing
  - Implements EmittingTask<T, C, E, I>
  - Stores event channel and processing state
  - Handles cancellation and completion
  
- `TokioSenderTask<T, C, E, I>` - Task for sending events
  - Implements SenderTask<T, C, E, I>
  - Manages event production
  
- `TokioReceiverTask<T, C, E, I>` - Task for receiving events
  - Implements ReceiverTask<T, C, E, I>
  - Manages event consumption
```

### 4. Implement Processing Strategies

Create new files:
- `/crates/tokio/src/task/emit/strategy/mod.rs`
- `/crates/tokio/src/task/emit/strategy/serial.rs`
- `/crates/tokio/src/task/emit/strategy/parallel.rs`
- `/crates/tokio/src/task/emit/strategy/batched.rs`
- `/crates/tokio/src/task/emit/strategy/adaptive.rs`

```markdown
Implement:
- Processing strategy interface
- Serial processing (one event at a time)
- Parallel processing (concurrent event handling)
- Batched processing (grouped event handling)
- Adaptive processing (dynamically adjusted concurrency)
```

### 5. Create Event Channels

```markdown
Implement:
- Channel for connecting sender and receiver
- Backpressure mechanism
- Cancellation propagation
- Event buffering with configurable capacity
```

### 6. Implement Event Builder

Create a new file:
- `/crates/tokio/src/task/emit/builder.rs`

```markdown
Implement:
- `TokioEmittingTaskBuilder<T, C, E, I>` - Builder for event tasks
  - Implements EmittingTaskBuilder
  - Configure sender and receiver properties
  - Set processing strategies
```

### 7. Connect to AsyncTask Implementation

Update:
- `/crates/tokio/src/task/tokio_task.rs`

```markdown
Implement:
- `fn emits<R: Send + 'static, Task: AsyncTask<R, I>>()` method
  - Create an EmittingTaskBuilder
  - Connect to TokioOrchestrator
```

### 8. Add Convenience Methods

Create:
- `/crates/tokio/src/emit.rs`

```markdown
Add:
- Factory functions for event processing
- Shortcuts for common event patterns
- Helpers for event stream conversion
```

## Implementation Notes

1. **Thread Safety**: All event processing must be thread-safe.

2. **Backpressure**: Implement proper backpressure to prevent overwhelming consumers.

3. **Cancellation**: Events must respect task cancellation.

4. **Memory Management**: Events should be cleaned up properly when tasks complete.

5. **Error Handling**: Errors in event processing should be properly propagated.

## Processing Strategy Details

### Serial Processing
- Events are processed one at a time in order
- Simple implementation with predictable behavior
- Good for maintaining strict ordering

### Parallel Processing
- Multiple events processed concurrently
- Uses worker pool with configurable size
- Good for independent event processing

### Batched Processing
- Groups events into batches
- Processes each batch as a unit
- Good for operations with setup/teardown costs

### Adaptive Processing
- Dynamically adjust concurrency based on workload
- Monitored performance metrics
- Balances throughput and resource usage

## Event Flow

1. **Sender** creates events
2. Events flow through **channel**
3. **Receiver** processes events using selected strategy
4. Results collected in **collector**
5. **Final event** signifies completion

## Expected Outcome

A complete event processing implementation that:
- Provides rich event streaming capabilities
- Supports different processing strategies
- Offers thread-safe collection of results
- Integrates with the AsyncTask framework
- Maintains the same ergonomic API as the spec

![Book](/assets/book.png)