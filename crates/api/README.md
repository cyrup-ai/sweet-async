# Sweet Async

Syntactic sugars for asynchronous work in Rust.

## Quick Start

## Process data with a future-based task

```rust
let task_result = AsyncTask::resolves_to<Model>()
    .with_name("Process User Data")
    .with_timeout_seconds(30)
    .with_priority(TaskPriority::Medium)
    .fallback_value({
        static_model.clone()
    })
    .spawn({ 
        all_models().filter_by_name("mistral:mistral-small-latest")
    })
    .await_result();
```

## Process Stream Events

```rust
let task = AsyncTask::emits<Vec<UserData>>()
    .with_name("Fetch User Data")
    .with_sender({
        strategy: SenderStrategy::Parallel,
        workers: MinMax(1, 4),
        rate_limit: 5.0, // Events per second
    }, { |collector| ->
        userdata_dao.find_all()
    }
    .with_receiver(|event, collector| {
        // Process each event
        let user = event.data();
        stats.increment_count();
        collector.collect(user.id, user);
    }, ReceiverStrategy::Serial)
    .execute();

// Wait for completion and get the final results
let (stats, final_event) = task.await_final_event().await;
println!("{} events processed", stats.processed);

```

## Processing Single Future

To execute a single piece of asynchronous work and get a result:

```rust
let task = AsyncTask::<SMSMessage>()
    .with_name("My Task")
    .spawn(my_async_function);

// Later, await the result
let result = task.await;
```

## Processing Events

```rust

let result = AsyncTask::emits<SMSMessage>()
    .with_name("Process Events")
    .with_sender(sender, SenderStrategy::Serial)
    .with_receiver(ReceiverStrategy::Serial { (e, collector) => {
        // Process each event individually
        process_data(event.data());
        stats.processed += 1;
    }, )
    .execute();

let (stats, final_event) = task.await_final_event().await;
println!("{} events processed", stats.processed);
```
