# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

```bash
# Build the crate
cargo build

# Run tests with nextest (preferred test runner)
cargo nextest run

# Format code and check for warnings
cargo fmt && cargo check --message-format short --quiet

# Run a single test
cargo nextest run <test_name>

# Check documentation
cargo doc --no-deps --open
```

## Architecture Overview

This crate is the Tokio implementation of the Sweet Async library, providing **fully asynchronous** task management and orchestration capabilities. The implementation follows the trait contracts defined in `sweet_async_api`.

## 🚨 CRITICAL: This is FULLY ASYNC - NOT SYNCHRONOUS! 🚨

**Sweet Async is 100% asynchronous**. The API design uses **synchronous method signatures that return awaitable futures/streams**, which is different from `async fn` but still **completely async**:

- **NO blocking** - Everything runs asynchronously via Tokio
- **NO `block_on`** - All blocking calls have been removed
- **Synchronous signatures** - Methods like `run()` have sync signatures but return `Future<Output=T>`
- **Awaitable results** - Callers use `.await` to execute the async work
- **Non-blocking execution** - All work happens in Tokio's async runtime

This pattern provides better composability than `async fn` while maintaining full async behavior.

### Core API Patterns

The Sweet Async API provides a fluent builder pattern with these key **async** usage patterns:

1. **Basic Task Execution** (FULLY ASYNC):
   ```rust
   AsyncTask::to::<ReturnType>()
       .timeout(Duration::from_secs(30))
       .run(|| async { /* async work */ })  // Returns a Future
       .await?  // Async execution happens here!
   ```

2. **Block Reduction Syntax** (sync-looking code that STILL runs async):
   ```rust
   AsyncTask::to::<ReturnType>()
       .run({ /* sync-looking code */ })  // Returns a Future
       .await?  // NOT blocking - this is async!
   ```

3. **await_result() Pattern** ("Leap Frog" - sync signature but ASYNC execution):
   ```rust
   let result = AsyncTask::to::<ReturnType>()
       .await_result(
           || async { /* async work */ },  // Work runs async
           |result| match result {  // Handler for result
               Ok(r) => r,
               Err(e) => Err(e)
           }
       )
       .await;  // Still async! Just wrapped differently
   ```
   
   **NOTE**: `await_result()` is NOT synchronous! It returns a Future that must be awaited. The "sync signature" means it doesn't use `async fn` syntax, but it's still 100% async.

4. **Event Emission Pattern** (ASYNC streaming):
   ```rust
   AsyncTask::emits::<EventType>()
       .sender(SenderStrategy::Parallel { workers }, |event, collector| async {
           // Async event generation
       })
       .receiver(ReceiverStrategy::Serial { timeout_seconds }, |event, collector| async {
           // Async event processing  
       })
       .await_final_event(|event, collector| async {
           // Async final handling
       })
       .await  // Entire pipeline is async!
   ```

### Key Components

1. **Task System** (`src/task/`)
   - `AsyncTask`: Core task implementation wrapping Tokio's JoinHandle
   - Builder pattern for fluent task creation
   - Emit/Spawn strategies for different execution models
   - Support for cancellation, recovery, timing, and tracing capabilities

2. **Runtime** (`src/runtime.rs`)
   - `TokioRuntime`: Wrapper around Tokio runtime
   - `safe_blocking`: Utility to safely run blocking operations without deadlocks

3. **Orchestra** (`src/orchestra/`)
   - Task orchestration and dependency management
   - Group operations for managing related tasks

### Implementation Status

The codebase implements a **fully async** API using Tokio. All blocking calls have been removed:
- ✅ All `futures::executor::block_on` replaced with async-safe alternatives
- ✅ Atomic values for synchronous access without blocking
- ✅ `try_lock` patterns for non-blocking mutex access
- ✅ Spawned tasks for deferred updates

Remaining work focuses on API completeness, not async safety.

### Important Patterns

1. **ASYNC EVERYWHERE**: This is a fully async implementation. Methods with sync signatures return Futures/Streams.
2. **No Blocking**: Zero `block_on` usage in async contexts. Use atomics or try_lock for sync access.
3. **Builder Pattern**: Fluent API that returns awaitable tasks
4. **Sync Signatures, Async Execution**: Methods like `run()` have sync signatures but return `impl Future`
5. **Error Handling**: No `unwrap`, `expect`, or `panic!` in production code
6. **Testing**: Use `cargo nextest` for all async tests

### Understanding the Async Model

```rust
// This method has a sync signature...
fn run(self, work: impl AsyncWork) -> impl Future<Output=Result<T, E>> {
    // ...but returns a Future that runs async when awaited!
    Box::pin(async move {
        // All work happens asynchronously here
        work.run().await
    })
}

// Caller uses it like this:
let result = builder.run(my_work).await;  // Async execution via .await
```

This pattern allows better trait composition than `async fn` while being 100% async.

## 🚨 CRITICAL: Execution Flow Understanding 🚨

### NOTHING EXECUTES UNTIL POLLED!

The execution flow is fundamentally lazy:

1. **Builder methods** (`.timeout()`, `.retry()`, etc.) - Just configure, NO execution
2. **`run()` method** - Creates task, returns Future, **NO EXECUTION YET**
3. **`.await` or polling** - **THIS IS WHEN EXECUTION HAPPENS**

### Common Mistakes to Avoid:

❌ **WRONG**: Spawning work during task construction
```rust
// WRONG - spawns immediately during construction
fn from_work() -> Self {
    let handle = tokio::spawn(async { work.run().await });  // NO! Executes immediately!
    Self { handle, ... }
}
```

✅ **CORRECT**: Store work, spawn only when polled
```rust
// CORRECT - lazy execution
fn from_work() -> Self {
    Self { work: Some(work), handle: None, ... }  // Just store it
}

impl Future for Task {
    fn poll(...) {
        if let Some(work) = self.work.take() {
            // NOW spawn using self.runtime()
            self.handle = Some(self.runtime().spawn(work));
        }
        // Poll the handle
    }
}
```

### Runtime Access Rules:

1. **ONLY access runtime through `runtime()` method** - This is the API
   - ✅ `self.runtime().spawn(...)` - Uses the trait method
   - ❌ `self.runtime.spawn(...)` - Direct field access
   - ❌ `tokio::spawn(...)` - Bypasses abstraction

2. **NO Handle parameters** - Tasks are self-contained
   - ❌ `fn new(handle: tokio::runtime::Handle)` - NO!
   - ✅ `fn new()` - Task creates/gets its own runtime internally

3. **Runtime is available via trait** - Every AsyncTask has `runtime()`
   - Provided by `ContextualizedTask` trait
   - Returns `&Self::RuntimeType`
   - Task stores runtime internally, provides via method

### The `run()` Method is Universal:

`run()` is THE execution point where:
- All configuration is applied
- Task is created with all settings
- Orchestra/Runtime is obtained (via thread-local or default)
- Future is returned (but NOT yet executed)

`await_result()` is just sugar that:
1. Calls `self.run(work)` to get the Future
2. Polls that Future to completion
3. Returns the result

Everything flows through `run()` - it's the universal method that bridges configuration to execution.

### Dependencies

The crate depends on:
- `sweet_async_api`: Trait definitions and contracts
- `tokio`: Async runtime with multi-threaded executor
- `futures`: Async utilities
- `uuid`: Task ID generation
- `tracing`: Structured logging