# Sweet Async Trait Alignment Plan - FINAL

## Summary
- **API Traits**: 43 total
- **Tokio Traits**: 6 total  
- **Missing from Tokio**: 43 traits (tokio implements NONE of the API traits!)
- **Duplicate Traits in Tokio**: 5 that should be deleted

## Tokio's Duplicate Traits (DELETE THESE)
1. **AsyncTaskContext** → Delete, implement API's **ContextualizedTask** instead
2. **AsyncTaskRecovery** → Delete, implement API's **RecoverableTask** instead  
3. **AsyncTaskTiming** → Delete, implement API's **TimedTask** instead
4. **AsyncTaskTracing** → Delete, implement API's **TracingTask** instead
5. **DurationExt** → Delete, implement API's **TimeExt** instead

## Tokio's Unique Trait (KEEP)
1. **AutoScalable** → This is tokio-specific functionality, keep it

## API Traits Missing from Tokio (43 total)
ALL of them! Tokio has not implemented ANY of the API traits:

```
AsyncResult
AsyncTask                    ← Core trait requiring 8 others
AsyncTaskBuilder
AsyncWork
CancellableTask             ← Required by AsyncTask
CancellationResult
Collector
ContextualizedTask          ← Required by AsyncTask (tokio has duplicate AsyncTaskContext)
CpuUsage
EmittingTask
EmittingTaskBuilder
ExecutionStats
FinalEvent
IntoAsyncResult
IoUsage
MemoryUsage
MetricsEnabledTask          ← Required by AsyncTask
NamedTask                   ← Required by AsyncTask
Orchestra
OrchestratorBuilder
PrioritizedTask             ← Required by AsyncTask
RankableByPriority
ReceiverBuilder
ReceiverEvent
ReceiverTask
RecoverableTask             ← Required by AsyncTask (tokio has duplicate AsyncTaskRecovery)
Runtime
RuntimeBuilder
SenderBuilder
SenderEvent
SenderEventBuilder
SenderTask
SpawningTask
SpawningTaskBuilder
StatusEnabledTask           ← Required by AsyncTask
StreamingEvent
TaskId                      ← Required by AsyncTask
TaskOrchestrator
TaskRelationships
TaskResult
TimedTask                   ← Required by AsyncTask (tokio has duplicate AsyncTaskTiming)
TimeExt                     ← (tokio has duplicate DurationExt)
TracingTask                 ← Required by AsyncTask (tokio has duplicate AsyncTaskTracing)
```

## Critical Finding
Tokio has been creating its own parallel trait hierarchy instead of implementing the API traits. This is exactly what you suspected when you said tokio is "redefining API traits".

## Action Plan

### Phase 1: Delete Duplicate Traits
```bash
# Remove tokio's duplicate traits
rm crates/tokio/src/task/task_context.rs  # Has AsyncTaskContext
rm crates/tokio/src/task/recoverable_task.rs  # Has AsyncTaskRecovery  
rm crates/tokio/src/task/timed_task.rs  # Has AsyncTaskTiming
rm crates/tokio/src/task/tracing_task.rs  # Has AsyncTaskTracing
rm crates/tokio/src/duration_ext.rs  # Has DurationExt

# Update imports throughout tokio to use API traits
```

### Phase 2: Implement ALL API Traits
Since tokio implements ZERO API traits, we need to implement all 43 of them. Priority order:

1. **Core AsyncTask Dependencies** (implement these first):
   - TaskId
   - NamedTask  
   - MetricsEnabledTask
   - PrioritizedTask
   - CancellableTask
   - TracingTask (replace AsyncTaskTracing)
   - TimedTask (replace AsyncTaskTiming)
   - ContextualizedTask (replace AsyncTaskContext)
   - RecoverableTask (replace AsyncTaskRecovery)
   - StatusEnabledTask

2. **Then implement AsyncTask trait itself**

3. **Then all the others**

## Success Criteria
- [ ] All 5 duplicate tokio traits removed
- [ ] All 43 API traits implemented on tokio structs
- [ ] Tokio's AsyncTask struct implements API's AsyncTask trait
- [ ] No compilation errors
- [ ] All existing tokio functionality preserved