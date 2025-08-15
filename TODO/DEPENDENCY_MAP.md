# Dependency Map & Execution Order

## Dependency Graph

```
Foundation (M0) ────────────┬──────────────┬──────────────┐
     │                     │              │              │
     │                     │              │              │
     ▼                     ▼              ▼              ▼
Lifecycle (M1)        Streaming (M2)  Spawning (M3)   Direct Dependents
     │                     │              │         (M6,M7,M8)
     │                     ├──────────────┤              │
     │                     │              │              │
     │                     ▼              │              │
     │            Sender/Receiver (M4)    │              │
     │                     │              │              │  
     │            Collection (M5) ────────┘              │
     │                     │                             │
     └─────────────────────┼─────────────────────────────┘
                           │
                           ▼
                   Orchestration (M9)
```

## Parallel Execution Groups

### Group 1: Foundation (Sequential)
**MUST BE COMPLETED FIRST**
- M0: Foundation (6 tasks) - Sequential execution required

### Group 2: Core Extensions (Parallel)
**Can run in parallel after Foundation**
- M1: Lifecycle (6 traits)
- M2: Streaming (6 traits)  
- M3: Spawning (6 traits)

### Group 3: Feature Implementation (Parallel)
**Can run in parallel after their dependencies**
- M4: Sender/Receiver (depends on M2 Streaming)
- M5: Collection (depends on M2 Streaming)
- M6: Categorization (depends on M0 Foundation only)
- M7: Monitoring (depends on M0 Foundation only)
- M8: Messaging (depends on M0 Foundation only)

### Group 4: Advanced (Sequential)
**Requires multiple dependencies**
- M9: Orchestration (depends on M0+M1+M3)

## Task-Level Dependencies

### Foundation Level (Critical Path)
1. `AsyncResult<T>` ← No dependencies (start here)
2. `AsyncTask<T,I>` ← No dependencies (parallel with 1)
3. `AsyncTaskBuilder` ← Depends on AsyncTask<T,I>
4. `AsyncWork<R>` ← No dependencies (parallel with 1,2)
5. `Runtime<T,I>` ← No dependencies (parallel with 1,2,4)
6. `TaskId` ← No dependencies (parallel with all)

### Optimization Opportunities

**Maximum Parallel Work:**
- 3 developers can work on M1, M2, M3 simultaneously
- 5 developers can work on M4, M5, M6, M7, M8 simultaneously (after deps)
- Total: Up to 8 parallel development streams

**Bottlenecks:**
- Foundation (M0) is sequential bottleneck - prioritize this
- Orchestration (M9) requires most dependencies - schedule last

**Risk Mitigation:**
- Foundation tasks can be partially parallelized (1,2,4,5,6 parallel, then 3)
- Streaming violations are critical - assign senior developer
- Test each milestone independently before moving to dependents

## Recommended Execution Order

### Phase 1: Foundation Completion
**First**: AsyncResult<T>, AsyncTask<T,I>, AsyncWork<R>, Runtime<T,I>, TaskId (5 parallel)
**Then**: AsyncTaskBuilder (depends on AsyncTask)
**Finally**: Integration testing and quality gate

### Phase 2: Core Extensions (3 Parallel Streams)
**Stream A**: Lifecycle traits (CancellableTask, StatusEnabled, etc.)
**Stream B**: Streaming traits (EmittingTask, StreamingEvent, etc.)  
**Stream C**: Spawning traits (SpawningTask, ContextualizedTask, etc.)

### Phase 3: Feature Implementation (5 Parallel Streams)
**Stream A**: Sender/Receiver (after Streaming complete)
**Stream B**: Collection (after Streaming complete)
**Stream C**: Categorization (after Foundation)
**Stream D**: Monitoring (after Foundation)
**Stream E**: Messaging (after Foundation)

### Phase 4: Advanced Features
**Sequential**: Orchestration implementation and integration testing

This dependency-aware approach maximizes parallel development while ensuring all prerequisites are met.
## Implementation Constraints

Write code with these goals in mind: 

  - zero allocation
  - blazing-fast
  - no unsafe
  - no unchecked 
  - *no locking*
  - elegant ergonomic code

DO NOT WRITE TESTS IN THE SAME FILE
ANOTHER AGENT will write those in ./tests/ (sister to src)


Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this iteratively and recursively until there is zero need for a "future enhancements" section.

think sequentially. step by step. ULTRATHINK.

Check all your work twice to ensure no symbol, method, trait bounds or other detail is missed, misaligned or omitted.

Review the architecture and requirements ... Focus keenly on the USER OBJECTIVE. it is your "guiding light" and ultimate "source of truth". Ensure all delivered items incrementally lead to this end state and ALL "the pieces fit.

Check all of your work a third time. Think sequentially, step by step. ULTRATHINK. Focus on performance. Are you using channels properly. are you optimizing allocations and inlining all the happy paths where it wi matter. Are all errors handled fully and semantically? think sequentially. step by step. ULTRATHINK.

Check all of your work a fourth time. think sequentially. step by step. ULTRATHINK. "Have I provided ALL the code, full and complete with all details handled and no "future enhancements", todos, "in a real situation", "for now", "in production". All such work will be rejected. Revise it recursively until it is perfected. 

Check all your work a fifth time. Are all the third party libraries using the very latest api signatures and "best in class idioms"? Revise your work recursively until all such issues are handled. Be a software artisan. Complex, feature rich, elegant, ergonomic source code is your requirement.

## All Issues Handle. NOTHING simplified. NOTHING stubbed. NOTHING "miminal"

Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this interactively until there is zero need for a "future enhancements" section.

=========================================

- express all source code fully
- certify that the code is complete and every potential optimization is included.


==== MANIFEST WITH THESE CONSTRAINTS =====

## No Potential for Improvement

Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this iteratively and recursively until there is zero need for a "future enhancements" section.

ADDITIONAL CONSTRAINTS:

- never use unwrap() (period!)
- never use expect() (in src/* or in examples)
- DO USE expect() in ./tests/*
- DO NOT use unwrap in ./tests/*

## MAKE ONLY NECESSARY CHANGES

- Focus on the User's objective
- Be useful, not thorough
- Make surgical, targeted changes vs sweeping changes

## DO NOT STUB CODE TO COME BACK LATER

- You will forget! 
- Write the full and correct code right now!
- if you don't know how and need to research, pause and research

## CLARIFICATIONS 

I DO NOT WANT YOU TO REWRITE WORKING CODE UNLESS REQUESTED (Bad)
I DO WANT YOU TO WRITE ALL NEW AND MODIFIED CODE WITH THESE CONSTRAINTS 