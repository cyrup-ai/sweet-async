# TODO.md - Crossbeam Implementation for Sweet Async

This TODO list details the implementation tasks for the crossbeam crate, which provides a synchronous, threaded alternative to the tokio implementation using crossbeam primitives (channels, threads, scopes, deques). The structure mirrors the api crate's trait definitions and the tokio implementation's layout. All structs must truly implement the corresponding traits from the api crate, without duplication or stubs. Focus on zero allocations where possible, blazing-fast performance, no unsafe code, no unchecked operations, no locking (leverage crossbeam's lock-free structures), and elegant, ergonomic code. Handle all errors fully and semantically using Result types. Use the latest crossbeam API signatures and best idioms.

Implementation must adapt the async-oriented API to synchronous threaded execution: e.g., task spawning uses crossbeam::thread::scope for scoped threads, communication via crossbeam::channel::bounded for efficient, allocation-minimal channels. Builders return types implementing api traits. No async/await in this impl; use threads for concurrency.

**General Guidelines for All Files:**
- Import traits and types from sweet_async_api (add dependency in Cargo.toml).
- Define structs that implement api traits exactly.
- Optimize: Use bounded channels to avoid allocations, inline hot paths, minimize cloning.
- Error handling: Propagate errors via Result, no unwrap/expect in src/.
- No tests in src/; they go in tests/.
- Certify code is complete: full implementations, no "future enhancements", todos, or stubs.

## Cargo.toml Setup
1. Add dependency: sweet_async_api = { path = "../api" }
2. Add crossbeam = "0.8" (latest version as of knowledge cutoff; verify and use latest).
3. Add other necessary deps (e.g., crossbeam-channel, crossbeam-deque, crossbeam-utils) with features for lock-free ops.
4. Set edition = "2021".

## src/lib.rs
1. Review api/src/lib.rs for re-exports.
2. Define mod task; mod orchestra; mod runtime;
3. Re-export key implementations: pub use task::*; pub use orchestra::*; etc., mirroring api.
4. Implement any top-level traits if needed, using crossbeam for concurrency.
5. Optimize: Ensure no unnecessary allocations in re-exports.
6. Handle errors: Any global setup errors returned semantically.

## src/runtime.rs
1. Review api/src/orchestra/runtime/runtime_trait.rs for RuntimeTrait.
2. Define struct CrossbeamRuntime implementing api::orchestra::runtime::RuntimeTrait.
3. Implement methods to initialize a crossbeam-based runtime (e.g., using crossbeam::thread::scope as base).
4. Add support for spawning tasks via threads, managing thread pools with crossbeam-deque for work-stealing.
5. Optimize: Use thread-local storage for zero-allocation task queuing where possible.
6. Handle errors: Thread panics caught and converted to TaskError.

## src/task/mod.rs
1. Review api/src/task/mod.rs for task-related traits.
2. Define pub mod builder; pub mod emit; pub mod spawn; etc., mirroring api/task subdirs.
3. Re-export all task implementations.
4. Implement any module-level traits or helpers using crossbeam.

## src/task/async_task.rs
1. Review api/src/task/async_task.rs for AsyncTask trait (adapt to sync).
2. Define struct CrossbeamAsyncTask<T> implementing api::task::AsyncTask<T>.
3. Implement builder methods to configure task (timeout, etc.) using crossbeam timers or channels.
4. For run(), spawn a thread via crossbeam::scope, execute closure synchronously, send result via bounded channel.
5. Optimize: Use bounded(1) channel for single-result tasks to minimize allocations.
6. Handle errors: Wrap thread results in Result, propagate join errors.

## src/task/builder.rs
1. Review api/src/task/builder.rs for task builder traits.
2. Define struct CrossbeamTaskBuilder<T> implementing api::task::TaskBuilder<T>.
3. Implement fluent methods (with_timeout, etc.) storing config in struct fields.
4. Build method spawns thread and returns handle implementing api traits.
5. Optimize: Pre-allocate channels in builder for zero runtime alloc.
6. Handle errors: Validate config in build, return Err on invalid states.

## src/task/cancellable_task.rs
1. Review api/src/task/cancellable_task.rs for CancellableTask.
2. Define struct CrossbeamCancellableTask<T> implementing the trait.
3. Implement cancel() by sending signal via channel to thread, which checks periodically.
4. Use crossbeam-channel::Select for non-blocking checks.
5. Optimize: Minimal overhead checks in hot loops.
6. Handle errors: Cancellation failure if thread already completed.

## src/task/cpu_usage.rs (and similar for io_usage, memory_usage)
1. Review api/src/task/cpu_usage.rs for metrics traits.
2. Define struct CrossbeamCpuUsage implementing api::task::CpuUsage.
3. Implement tracking using crossbeam-utils or system calls in threads.
4. Aggregate metrics via channels without locking.
5. Optimize: Sample metrics efficiently, avoid allocations in sampling.
6. Handle errors: Metrics unavailability returns default or Err.

## src/task/default_context.rs
1. Review api/src/task/default_context.rs.
2. Define CrossbeamDefaultContext implementing the trait.
3. Provide default thread-local context using crossbeam-utils::thread.
4. Optimize: Zero-cost abstraction.
5. Handle errors: N/A, or context init failures.

## src/task/fallback.rs
1. Review api/src/task/fallback.rs.
2. Define CrossbeamFallbackTask<T> implementing fallback traits.
3. Implement fallback logic by trying primary thread, on error spawn fallback.
4. Use crossbeam::scope for nested scopes.
5. Optimize: Inline fallback if simple.
6. Handle errors: Chain errors from primary and fallback.

## src/task/message_builder.rs
1. Review api/src/task/message_builder.rs.
2. Define CrossbeamMessageBuilder implementing the trait.
3. Build messages for channel communication.
4. Optimize: Use smallvec or fixed-size if possible to avoid alloc.
5. Handle errors: Invalid message formats.

## src/task/named_task.rs
1. Review api/src/task/named_task.rs.
2. Define CrossbeamNamedTask<T> implementing the trait.
3. Add name to task struct, use in logging/metrics.
4. Optimize: Const string if possible.
5. Handle errors: N/A.

## src/task/recoverable_task.rs
1. Review api/src/task/recoverable_task.rs.
2. Define CrossbeamRecoverableTask<T> implementing the trait.
3. Implement retry logic with backoff using threads and timers.
4. Use crossbeam-channel for control.
5. Optimize: Exponential backoff without alloc.
6. Handle errors: Max retries exceeded.

## src/task/task_communication.rs
1. Review api/src/task/task_communication.rs.
2. Define structs for channels implementing communication traits.
3. Use crossbeam::channel::bounded.
4. Optimize: Choose capacity to avoid resizing.
5. Handle errors: Send/recv failures.

## src/task/task_context.rs
1. Review api/src/task/task_context.rs.
2. Define CrossbeamTaskContext implementing the trait.
3. Store thread-specific data.
4. Optimize: Thread-local storage.
5. Handle errors: Context access errors.

## src/task/task_error.rs
1. Review api/src/task/task_error.rs.
2. Define CrossbeamTaskError variants extending api's Error.
3. Implement From for common errors.
4. Optimize: No alloc if using enum without Box.
5. Handle errors: All cases covered.

## src/task/task_id.rs
1. Review api/src/task/task_id.rs.
2. Define CrossbeamTaskId implementing the trait, using atomic or uuid.
3. Use crossbeam-utils::atomic for id generation.
4. Optimize: Lock-free id increment.
5. Handle errors: Id exhaustion (unlikely).

## src/task/task_metrics.rs
1. Review api/src/task/task_metrics.rs.
2. Define CrossbeamTaskMetrics implementing aggregation.
3. Use channels to collect from threads.
4. Optimize: Batch metrics to reduce overhead.
5. Handle errors: Metrics overflow.

## src/task/task_priority.rs
1. Review api/src/task/task_priority.rs.
2. Define CrossbeamTaskPriority implementing the trait.
3. Use priority queues via crossbeam-deque.
4. Optimize: Work-stealing with priorities.
5. Handle errors: N/A.

## src/task/task_relationships.rs
1. Review api/src/task/task_relationships.rs.
2. Define structs for parent-child tasks using channels for synchronization.
3. Implement dependencies with barriers or channels.
4. Optimize: Minimal sync points.
5. Handle errors: Dependency cycles.

## src/task/task_status.rs
1. Review api/src/task/task_status.rs.
2. Define CrossbeamTaskStatus implementing monitoring.
3. Poll status via channels without blocking.
4. Optimize: Atomic status flags.
5. Handle errors: Status unavailable.

## src/task/timed_task.rs
1. Review api/src/task/timed_task.rs.
2. Define CrossbeamTimedTask<T> implementing timeout logic.
3. Use std::time or crossbeam for timing, abort thread on timeout.
4. Optimize: Precise timing without polling.
5. Handle errors: Timeout as specific Err.

## src/task/tracing_task.rs
1. Review api/src/task/tracing_task.rs.
2. Define CrossbeamTracingTask<T> implementing tracing.
3. Integrate with tracing crate, emit events from threads.
4. Optimize: Low-overhead tracing.
5. Handle errors: Tracing failures ignored or logged.

## src/task/emit/* (mirror files from api/task/emit)
1. For each file in api/task/emit, create equivalent implementing emit traits.
2. Use crossbeam channels for event emission.
3. Steps similar to above: define struct, impl trait, optimize channels, handle send errors.

## src/task/spawn/* (mirror files from api/task/spawn)
1. Similar to emit, implement spawn traits with crossbeam::thread::spawn_scoped.
2. Optimize: Scoped threads to avoid leaks.
3. Handle join errors.

## src/orchestra/mod.rs
1. Review api/src/orchestra/mod.rs.
2. Re-export orchestra implementations.
3. Implement any orchestra-level traits.

## src/orchestra/builder.rs
1. Review api/src/orchestra/builder.rs.
2. Define CrossbeamOrchestraBuilder implementing the trait.
3. Build orchestrators with config for threading.
4. Optimize: Pre-configure channel capacities.
5. Handle errors: Invalid config.

## src/orchestra/orchestrator.rs
1. Review api/src/orchestra/orchestrator.rs.
2. Define CrossbeamOrchestrator implementing the trait.
3. Orchestrate multiple tasks via thread pools and channels.
4. Support patterns like sender/receiver with crossbeam channels.
5. Optimize: Work-stealing deque for tasks.
6. Handle errors: Orchestration failures, propagate task errors.

## src/orchestra/channel_orchestrator.rs (if present in tokio)
1. Define implementing channel-based orchestration.
2. Use crossbeam::channel for multi-producer multi-consumer.
3. Optimize: Bounded channels.
4. Handle errors: Channel closed.

## src/orchestra/vector_clock.rs (if present)
1. Implement vector clock for ordering using crossbeam atomics.
2. Optimize: Lock-free updates.
3. Handle errors: Clock skew.

## src/orchestra/deployment/* (mirror subdirs)
1. For auto_scale etc., implement deployment traits with threaded scaling.
2. Use crossbeam to dynamically adjust thread counts.
3. Optimize: Dynamic worker pools without alloc.
4. Handle errors: Scaling limits.

**Completion Criteria:**
- All api traits implemented in crossbeam structs.
- Code certified complete, performant, error-handled.
- Verify by building the crate and checking trait impls align.

Once all tasks are complete, mark this TODO as done.