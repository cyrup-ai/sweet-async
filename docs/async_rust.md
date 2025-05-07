# Async Rust Improvments

## The Article

[https://tmandry.gitlab.io/blog/posts/making-async-reliable/](https://tmandry.gitlab.io/blog/posts/making-async-reliable/)

### Objective: Handling Cancellation in Async Rust
**Solution:** Use better primitives such as `merge` instead of `select!` to handle cancellation contracts explicitly. This reduces the risk of unexpected cancellation.

```rust
let mut file = ...;
let mut channel = ...;
merge! {
    repeated(|| read_send(&mut file, &mut channel)) => (),
    some_data = socket.packet_stream() => {
        // Handle incoming data
    }
}.await;
```

### Objective: Mitigating Starvation in Async Iterators
**Solution:** Implement `poll_progress` or spawn tasks to ensure progress without starving the iterator.

```rust
// Using poll_progress
async fn process_stream<I>(mut stream: I)
where
    I: AsyncIterator,
{
    while let Some(item) = stream.poll_progress().await {
        // Process item
    }
}

// Spawning tasks
async fn process_with_spawning<I>(stream: I)
where
    I: AsyncIterator,
{
    task::spawn(async move {
        while let Some(item) = stream.next().await {
            // Process item
        }
    });
}
```

### Objective: Avoiding Detached Execution
**Solution:** Use structured concurrency to manage task lifetimes and avoid tasks running out of scope.

```rust
async fn structured_task() {
    let scope = Scope::new();
    scope.spawn(async {
        // Task logic
    });
    // All tasks complete before scope ends
}

// Implementing structured concurrency manually
struct Scope<'a> {
    // Task management fields
}

impl<'a> Scope<'a> {
    fn new() -> Self {
        // Initialize scope
    }

    fn spawn<F>(&self, f: F)
    where
        F: Future + 'a,
    {
        // Spawn task
    }
}
```

### Objective: Simplifying Async Control Flow with Generators
**Solution:** Use generators to write concise async iteration logic.

```rust
async fn generator_example() -> impl AsyncIterator<Item = u32> {
    async move {
        yield 1;
        yield 2;
        yield 3;
    }
}

// Using the generator
let mut gen = generator_example();
while let Some(val) = gen.next().await {
    println!("{}", val);
}
```

These solutions aim to improve reliability and manageability in Async Rust programming by addressing specific concerns related to cancellation, starvation, detached execution, and control flow..
