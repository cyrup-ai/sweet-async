# TODOLIST for `sweet_async` Tokio Implementation

This is a comprehensive list of all remaining items required for a robust, production-quality implementation of the Tokio backend for `sweet_async`. This list is based on the API contract, README usage patterns, and best practices for async Rust. All items must be completed before the crate is considered done. 

## 1. Trait Compliance & Core Features
- [ ] Ensure **every trait** in `api/task` and `api/orchestra` has a corresponding, robust Tokio implementation (no stubs, no partials)
- [ ] All public interfaces match the ergonomic, fluent builder API in the README
- [ ] All async methods return awaitable handles/results, never block the main thread
- [ ] The emit/event system is fully implemented and first-class

## 2. Correctness, Safety, and Style
- [ ] No use of `block_on`, `unwrap`, `expect`, `panic!`, `eprintln!`, `dbg!`, or `unsafe` in production code
- [ ] No `#[allow(...)]` or warning suppression; all warnings must be fixed at the source
- [ ] No commented-out code, stubs, or partial implementations
- [ ] All `Result`/`Option` handled explicitly; no silent error swallowing
- [ ] Use `tracing` for logs at appropriate levels

## 3. Metrics, Cancellation, and Group Operations
- [ ] All task metrics (CPU, memory, I/O, etc.) are tracked and surfaced as per API contract
- [ ] Task cancellation is robust and propagates correctly (including group and dependency cancellation)
- [ ] Group operations (register, cancel, await group) are fully implemented and tested

## 4. Builder and Macro Ergonomics
- [ ] Builder pattern is fully ergonomic and supports all API/README usage patterns
- [ ] All macros from `api/macros` are supported as needed for public API
- [ ] Block reduction and sync-to-async patterns are fully supported

## 5. Testing and Documentation
- [ ] All code is covered by nextest-based async tests (no blocking in tests)
- [ ] All public APIs are documented clearly (doc comments, usage examples)
- [ ] Internal docs explain complex areas (e.g., orchestrator, emit system)

## 6. Performance and Concurrency
- [ ] Mutexes/RwLocks are used judiciously; consider `DashMap` or lock-free where appropriate
- [ ] No unnecessary contention or deadlocks
- [ ] All operations are as performant as possible for high-throughput async workloads

## 7. Final Checks
- [ ] `cargo fmt && cargo check --message-format short --quiet` passes with zero warnings or errors
- [ ] `cargo test` (with nextest) passes all tests
- [ ] All TODOs, FIXMEs, and commented-out code are removed

---

## Concrete File-Specific Tasks (from audit)

### `task/recoverable_task.rs`
- [ ] Refactor all usages of `futures::executor::block_on` to use `safe_blocking` or a fully async approach. No blocking in async context allowed.
- [ ] Ensure all trait methods are fully async-compatible and do not risk deadlocks.

### `task/builder/builder.rs`
- [ ] Review and address the type erasure/placeholder logic in `await_result_with_handler` for emitting tasks. Confirm this is unreachable or provide a robust implementation.
- [ ] Confirm all builder methods match the API trait contract and README usage patterns (especially for emitting and spawning paths).

---

### `task/task_context.rs`
- [ ] Refactor all usages of `futures::executor::block_on` to use `safe_blocking` or a fully async approach. No blocking in async context allowed.
- [ ] Ensure all trait methods are fully async-compatible and do not risk deadlocks.

---

### `task/timed_task.rs`
- [ ] Refactor all usages of `futures::executor::block_on` to use `safe_blocking` or a fully async approach. No blocking in async context allowed.
- [ ] Ensure all trait methods are fully async-compatible and do not risk deadlocks.

---

**If you add or change a feature, update this list!**

---

> Last updated: 2025-05-20
