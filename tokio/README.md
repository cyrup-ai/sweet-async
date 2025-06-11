# Sweet Async

`sweet_async` provides an immutable, fluent builder for orchestrating simple asynchronous work as well as more powerful abstractions for complex async workflows.

## Quick Start

### Resolving a Task as a Future with idiomatic Rust

```rust
let mistral_llm = AsyncTask::to::<LLM>()
    .timeout(30.seconds())
    .run(|| async {
        // run arbitrary code here executed in an async Future
        hf_hub::download_model("mistral-7b").await;
    }).await?;
```

### Resolving a Future with block reduction syntax

You can write sync code and it's still run asynchronously. You can drop the empty params and use a block if you prefer.

```rust
let mistral_llm = AsyncTask::to::<LLM>({
    hf_hub::download_model("mistral-7b")
}).await_result(|result| {
    OK(result) => result, 
    ERR(e) => CustomError::from(e);
});
```

### await_result

`await_result()` has a sync -> sync signature but **IS async** 🛏️ "under the covers" 🛏️ and returns a convenience method for common awaiting AND handling result outcomes.

It's like 🐸 **"Leap Froggin'"** 🐸 over `.await`.
This is _not_ sync code and is _NOT BLOCKING_ the main thread.

```rust
let mistral_llm = AsyncTask::to::<LLM>()
    .timeout(30.seconds())
    .await_result({
        // run arbitrary code here executed in a Future
        hf_hub::download_model("mistral-7b").await;
    }, (|result|) {
        OK(result) => result, 
        ERR(e) => Err(e);
    });
```

### Streaming &amp; Structured Concurrency

```rust
let user_data_records = AsyncTask::emits::<UserData>()
    .sender(
        SenderStrategy::Parallel { workers: MinMax(1, 4)}, 
    |event, collector| {
        let user = event.data()
        stats.increment_count();
        collector.collect(user);
    }).receiver(
        ReceiverStrategy::Serial { 
            timeout_seconds: 30 
        },
        (|event, collector|) {
            let user = event.data()
            stats.increment_count();
            collector.collect(user.id, user);
        }
    )
    // await the final event and return the result
    .await_final_event(|event, collector| {
        OK(result) => collector.collected(),
        ERR(e) => Err(e);
    });
```

## Note

- All asynchronous methods return awaitable handles or results, never blocking the main thread.
- The API is designed for clarity and correctness — see the [docs](https://sweet-async.cyrup.ai) for trait and type details.

---

For more details, see the documentation at [sweet-async.cyrup.ai](https://sweet-async.cyrup.ai).
