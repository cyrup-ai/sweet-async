# Sweet Async

Syntactic sugars for asynchronous work in Rust.

## Quick Start

## Process data with a future-based task

```rust
let mistral_model = AsyncTask::resolves_to<LLM>()
    .with_name("Fetch Mistral")
    .with_timeout_seconds(30)
    .with_priority(TaskPriority::Low)
    .spawn({ 
        all_models().filter_by_name("mistral:mistral-small-latest").first()
    })
    .await_result();
```

## Process Stream Events

```rust
let task = AsyncTask::emits<UserData>()
    .with_name("Fetch User Data")
    .with_sender(SenderStrategy::Parallel {
        workers: MinMax(1, 4),
        rate_limit: 5.0, // Events per second
    }, { |collector| =>
        userdata_dao.find_all()
    }
    .with_receiver(ReceiverStrategy::Serial {
        timeout_seconds: 30
    },
    |e, collector| {
        // Process each event
        let user = e.data();
        stats.increment_count();
        collector.collect(user.id, user);
    })
    .execute();

// Wait for completion and get the final results
let (stats, final_event) = task.await_final_event().await;
println!("{} events processed", stats.processed);

```
