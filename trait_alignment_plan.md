# Sweet Async Trait Alignment Plan

## Overview
This plan systematically aligns tokio implementation with sweet_async API traits. The goal is to make tokio structs implement API traits while preserving existing tokio functionality.

## API Traits (40 total)
```
AsyncResult<T>
AsyncTask<T, I>                    ← Core trait requiring 8 others
AsyncTaskBuilder
AsyncWork<R>
CancellableTask<T>                 ← Required by AsyncTask
CancellationResult
Collector<T, C, Collection>
ContextualizedTask<T, I>           ← Required by AsyncTask
CpuUsage
EmittingTask<T, C, E, I>
EmittingTaskBuilder<T, C, E, I>
ExecutionStats
FinalEvent<T, C, Item, Collection>
IntoAsyncResult<T, E>
IoUsage
MemoryUsage
MetricsEnabledTask<T>              ← Required by AsyncTask
NamedTask                          ← Required by AsyncTask
Orchestra<T, Task, I>
OrchestratorBuilder<T, Task, I>
PrioritizedTask<T>                 ← Required by AsyncTask
RankableByPriority
ReceiverBuilder<T, C, E, I>
ReceiverEvent<T, C>
ReceiverTask<T, C, E, I>
RecoverableTask<T>                 ← Required by AsyncTask
Runtime<T, I>
RuntimeBuilder<T, I>
SenderBuilder<T, C, E, I>
SenderEvent<T>
SenderEventBuilder<T>
SenderTask<T, C, E, I>
SpawningTask<T, I>
SpawningTaskBuilder<T, E, I>
StatusEnabledTask<T>               ← Required by AsyncTask
StreamingEvent<T>
TaskId                             ← Required by AsyncTask
TaskOrchestrator<T, Task, I>
TaskRelationships<T, I>
TaskResult<T>
TimedTask<T>                       ← Required by AsyncTask
TimeExt
TracingTask<T>                     ← Required by AsyncTask
```

## Tokio Traits (6 total)
```
AsyncTaskContext<T, I>      ← DUPLICATE of ContextualizedTask<T, I>
AsyncTaskRecovery<T, I>     ← DUPLICATE of RecoverableTask<T>
AsyncTaskTiming<T, I>       ← DUPLICATE of TimedTask<T>
AsyncTaskTracing<T, I>      ← DUPLICATE of TracingTask<T>
AutoScalable                ← Tokio-specific trait
DurationExt                 ← DUPLICATE of TimeExt
```

## Critical Issue: Tokio Redefining API Traits
Tokio has created 5 duplicate traits that should be deleted:

1. **AsyncTaskContext** → Delete, use API's **ContextualizedTask**
2. **AsyncTaskRecovery** → Delete, use API's **RecoverableTask**  
3. **AsyncTaskTiming** → Delete, use API's **TimedTask**
4. **AsyncTaskTracing** → Delete, use API's **TracingTask**
5. **DurationExt** → Delete, use API's **TimeExt**

## Implementation Plan

### Phase 1: Remove Duplicate Traits (Priority: CRITICAL)
```bash
# Delete tokio's duplicate trait files
rm crates/tokio/src/task/async_task_context.rs
rm crates/tokio/src/task/async_task_recovery.rs  
rm crates/tokio/src/task/async_task_timing.rs
rm crates/tokio/src/task/async_task_tracing.rs
rm crates/tokio/src/duration_ext.rs

# Update imports throughout tokio to use API traits instead
```

### Phase 2: Implement Core API Traits on Tokio Structs
Priority order based on AsyncTask dependencies:

1. **TaskId** - Add `impl TaskId for Uuid`
2. **NamedTask** - Add `impl NamedTask for AsyncTask`
3. **MetricsEnabledTask** - Add `impl MetricsEnabledTask for AsyncTask`
4. **TimedTask** - Add `impl TimedTask for AsyncTask` 
5. **StatusEnabledTask** - Add `impl StatusEnabledTask for AsyncTask`
6. **CancellableTask** - Add `impl CancellableTask for AsyncTask`
7. **TracingTask** - Add `impl TracingTask for AsyncTask`
8. **ContextualizedTask** - Add `impl ContextualizedTask for AsyncTask`
9. **RecoverableTask** - Add `impl RecoverableTask for AsyncTask`
10. **PrioritizedTask** - Add `impl PrioritizedTask for AsyncTask`
11. **AsyncTask** - Add `impl AsyncTask for AsyncTask` (core trait)

### Phase 3: Implement Builder Traits
1. **AsyncTaskBuilder** - Add `impl AsyncTaskBuilder for AsyncTaskBuilder`
2. **AsyncWork** - Add `impl AsyncWork for closures/functions`
3. **SpawningTaskBuilder** - Add implementations
4. **EmittingTaskBuilder** - Add implementations

### Phase 4: Implement Orchestra Traits  
1. **Runtime** - Add `impl Runtime for TokioRuntime`
2. **RuntimeBuilder** - Add `impl RuntimeBuilder for RuntimeBuilder`
3. **Orchestra** - Add `impl Orchestra for Orchestra`
4. **TaskOrchestrator** - Add `impl TaskOrchestrator for orchestrators`
5. **OrchestratorBuilder** - Add implementations

### Phase 5: Implement Result/Future Traits
1. **TaskResult** - Add `impl TaskResult for task results`
2. **AsyncResult** - Add `impl AsyncResult for async results`
3. **IntoAsyncResult** - Add implementations for conversions

### Phase 6: Implement Streaming/Event Traits
1. **EmittingTask** - Add `impl EmittingTask for EmittingTask`
2. **SpawningTask** - Add `impl SpawningTask for SpawningTask`
3. **StreamingEvent** - Add implementations
4. **SenderTask/ReceiverTask** - Add implementations

## File Changes Required

### Files to Delete:
- `crates/tokio/src/task/async_task_context.rs`
- `crates/tokio/src/task/async_task_recovery.rs`
- `crates/tokio/src/task/async_task_timing.rs`
- `crates/tokio/src/task/async_task_tracing.rs`
- `crates/tokio/src/duration_ext.rs`

### Files to Modify:
- `crates/tokio/src/lib.rs` - Remove duplicate trait exports, add API trait impls
- `crates/tokio/src/task/mod.rs` - Update imports and exports
- `crates/tokio/src/task/async_task.rs` - Add API trait implementations
- `crates/tokio/src/runtime.rs` - Add Runtime trait implementations
- All files importing the duplicate traits - Update to use API traits

### New Files to Create:
- `crates/tokio/src/task/api_trait_impls.rs` - Centralized API trait implementations
- `crates/tokio/src/orchestra/api_trait_impls.rs` - Orchestra trait implementations

## Success Criteria
- [ ] All duplicate tokio traits removed
- [ ] All API traits implemented on appropriate tokio structs
- [ ] Tokio's AsyncTask struct implements API's AsyncTask trait
- [ ] No compilation errors
- [ ] All existing tokio functionality preserved
- [ ] Clean separation between API contracts and tokio implementation

## Next Steps
1. Start with Phase 1: Delete duplicate traits and update imports
2. Implement core traits in dependency order
3. Test incrementally to ensure no functionality is lost
4. Verify full API compliance when complete