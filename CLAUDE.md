# CLAUDE.md - Sweet Async Project

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 GROUND RULES - READ THIS FIRST 🚨

### **CRITICAL RULES FOR ALL CLAUDE SESSIONS:**

1. **NEVER CHANGE THE API CRATE** (`api/`) WITHOUT EXPLICIT WRITTEN INSTRUCTION
2. **THE SYNTAX IS REAL AND IMPLEMENTED** - DO NOT QUESTION OR DOUBT IT
3. **FOCUS ON IMPLEMENTATION WORK** - tokio first, then crossbeam
4. **DO NOT SUGGEST API CHANGES** - The fluent syntax is final and working

## 🔥 THE SYNTAX IS REAL! DO NOT DOUBT IT! 🔥

**IMPORTANT**: The Sweet Async API syntax shown in the README.md and examples **IS REAL AND IMPLEMENTED**. 

❌ **DO NOT:**
- Question whether the syntax "will work"
- Suggest changes to the API design
- Doubt that methods like `of_browser_history()` exist
- Try to "simplify" or "improve" the fluent API
- Modify trait signatures in the `api/` crate

✅ **DO:**
- Implement the traits in tokio/crossbeam crates
- Follow the existing API contracts exactly
- Trust that the syntax sugar works as designed
- Focus on runtime implementation, not API design

## API Architecture Overview

Sweet Async provides a **fluent, immutable builder API** for orchestrating asynchronous work. The key design principle is:

**Synchronous method signatures that return Future/Stream types = 100% async execution with intuitive syntax**

### Core Execution Patterns

#### 1. **Basic Task Execution** (`AsyncTask::to::<T>()`)
```rust
// THIS SYNTAX IS REAL AND WORKS:
let mistral_llm = AsyncTask::to::<LLM>()
    .timeout(30.seconds())
    .run(|| async {
        hf_hub::download_model("mistral-7b").await
    }).await?;
```

#### 2. **Block Reduction Syntax** 
```rust
// THIS SYNTAX IS REAL AND WORKS:
let mistral_llm = AsyncTask::to::<LLM>({
    hf_hub::download_model("mistral-7b")  // No .await needed in block
}).await_result(|result| {
    OK(result) => result, 
    ERR(e) => CustomError::from(e)
});
```

#### 3. **"Leap Frog" Pattern** (`await_result()`)
```rust
// THIS HAS SYNC SIGNATURE BUT IS 100% ASYNC:
let result = AsyncTask::to::<LLM>()
    .await_result({
        hf_hub::download_model("mistral-7b").await
    }, |result| {
        OK(result) => result,
        ERR(e) => Err(e)
    }); // Still returns Future - must .await!
```

#### 4. **Event Emission Pattern** (`AsyncTask::emits::<T>()`)
```rust
// THIS SYNTAX IS REAL AND WORKS:
let results = AsyncTask::emits::<CsvRecord>()
    .sender(|collector| {
        collector.of_file("data.csv")
            .with_delimiter(Delimiter::NewLine)
            .into_chunks(100.rows());
    })
    .receiver(|event, collector| {
        let record = event.data();
        collector.collect(record.id, record);
    })
    .await_final_event(|event, collector| {
        collector.collected()
    });
```

## 🍭 Syntax Sugar Categories

**ALL OF THESE METHODS ARE REAL AND IMPLEMENTED** via the macro system in `api/src/syntax_sugar.rs`:

### Task Builder Level Sugar
```rust
// THESE METHODS EXIST AND WORK:
.with_auto_scaling(bool)
.lambda(fn)
.fallback_to_vercel(fn) 
.fallback_to_local(fn)
.with_vector_clock(Vec<()>)
.with_circuit_breaker(())
.hurl(fn)
.await_result<B, H>()
```

### Social & Communication Sources
```rust
// THESE COLLECTOR METHODS ARE REAL:
collector.of_browser_history(Browser::Chrome)
    .with_profile(BrowserProfile::Primary)
    .with_date_range(last_month)
    .into_chunks(1000.visits());

collector.of_google_tasks(credentials)
    .acting_as("user@domain.com")
    .with_due_date(DueDate::Today)
    .with_status(TaskStatus::uncompleted())
    .order_by(TaskOrder::Priority.desc());

collector.of_slack_messages(credentials)
    .with_channel("general")
    .containing(["sick", "pto", "late"].any());

collector.of_linkedin_posts(credentials)
    .with_llm("jobs for CTOs in the Bay Area".generate())
    .with_posted_date(DateRange::LastMonth);

collector.of_calendar_events(CalendarProvider::Google)
    .with_event_type(EventType::Meeting)
    .ordered_by(EventOrder::StartTime.asc());
```

### Database Sources
```rust
// THESE DATABASE METHODS ARE REAL:
collector.from_surrealdb(conn, "SELECT * FROM records")
    .with_live_query(true)
    .into_chunks(500.records());

collector.from_seaorm(sea_query)
    .with_batch_size(2000)
    .into_chunks(2000.entities());

collector.from_sqlite(conn, "SELECT * FROM logs")
    .with_streaming(true)
    .into_chunks(5000.rows());

collector.from_mysql(conn, "SELECT * FROM events")
    .into_chunks(1500.rows());
```

### Cloud Storage Sources  
```rust
// THESE CLOUD STORAGE METHODS ARE REAL:
collector.from_s3(bucket, "data/*.csv")
    .with_region(Region::UsEast1)
    .with_credentials(aws_creds)
    .into_chunks(50.objects());

collector.from_gcs(bucket, "logs/**/*.json")
    .with_service_account(gcp_creds)
    .into_chunks(25.objects());

collector.from_azure_blob(container, "*.parquet")
    .with_connection_string(azure_conn)
    .into_chunks(100.blobs());
```

### API & Network Sources
```rust
// THESE API METHODS ARE REAL:
collector.from_api_batch(url_list)
    .with_concurrency(10.concurrent_requests())
    .with_retry(3.attempts())
    .with_timeout(30.seconds())
    .into_chunks(25.responses());

collector.from_websocket("wss://api.example.com/stream")
    .with_auth(bearer_token)
    .into_chunks(100.messages());

collector.from_sse("https://api.example.com/events")
    .with_headers(custom_headers)
    .into_chunks(50.events());

collector.from_graphql_subscription(client, query)
    .into_chunks(25.updates());
```

### Version Control Sources
```rust
// THESE VCS METHODS ARE REAL:
collector.from_github("owner/repo", "data/*.json")
    .with_auth(github_token)
    .with_branch("main")
    .into_chunks(10.files());

collector.from_gitlab("group/project", "configs/*.yaml")
    .with_token(gitlab_token)
    .into_chunks(20.files());

collector.from_git_repo("https://github.com/owner/repo.git")
    .with_path("data/**/*.csv")
    .with_ref("main")
    .into_chunks(5.files());
```

### Message Queue Sources
```rust
// THESE MESSAGE QUEUE METHODS ARE REAL:
collector.from_kafka_topic("user-events")
    .with_consumer_group("processors")
    .with_offset(Offset::Latest)
    .with_partition_strategy(PartitionStrategy::RoundRobin)
    .into_chunks(100.messages());

collector.from_redis_stream(conn, "events:*")
    .with_consumer_group("workers")
    .into_chunks(50.entries());

collector.from_rabbitmq(conn, "task_queue")
    .with_prefetch(100)
    .into_chunks(25.messages());

collector.from_nats(conn, "events.>")
    .with_queue_group("processors")
    .into_chunks(200.messages());
```

### Collector Configuration Sugar
```rust
// ALL THESE CONFIGURATION METHODS ARE REAL:
.with_profile()          .with_date_range()       .order_by()
.acting_as()             .with_due_date()         .with_status()
.containing()            .with_llm()              .with_posted_date()
.feed_type()             .filtered_by()           .in_time_range()
.with_generation()       .with_cursor_size()      .with_live_query()
.with_batch_size()       .with_streaming()        .with_service_account()
.with_connection_string() .with_ref()             .with_consumer_group()
.with_offset()           .with_partition_strategy() .with_prefetch()
.with_queue_group()      .with_retry()            .with_timeout()
.into_chunks()           .of()                    .sequence()
```

## Advanced Syntax Patterns

### Sequential Processing
```rust
// THIS .sequence() SYNTAX IS REAL:
AsyncTask::emits::<MigrationStep>()
    .sequence()  // Guarantees sequential execution
    .sender(|collector| {
        collector.of(migration_steps)
            .into_chunks(1.item());  // Process one at a time
    })
```

### Cloud Function Execution
```rust
// THESE CLOUD METHODS ARE REAL:
AsyncTask::emits::<ProcessedItem>()
    .lambda(|event, collector| { /* AWS Lambda */ })
    .fallback_to_vercel(|event, collector| { /* Vercel Edge */ })
    .fallback_to_local(|event, collector| { /* Local dev */ })
    .with_auto_scaling(true)
```

### Vector Clock Sequencing
```rust
// THIS VECTOR CLOCK SYNTAX IS REAL:
AsyncTask::emits::<TimestampedEvent>()
    .with_vector_clock(cluster_nodes)
    .sender(|collector| {
        collector.from_distributed_sources(node_streams)
            .with_causality_tracking(true)
            .into_chunks(100.events());
    })
```

### Circuit Breaker Patterns
```rust
// THIS CIRCUIT BREAKER SYNTAX IS REAL:
AsyncTask::emits::<ProcessedItem>()
    .with_circuit_breaker(CircuitBreaker::new()
        .failure_threshold(5)
        .timeout(Duration::from_secs(60))
        .half_open_max_calls(3))
```

## Implementation Guidelines

### Work Focus Areas
1. **Tokio Implementation** (`tokio/` crate) - PRIMARY FOCUS
2. **Crossbeam Implementation** (future) - SECONDARY 
3. **Runtime Integration** - Connect APIs to async runtimes
4. **Error Handling** - Implement Result types and error propagation
5. **Testing** - Verify syntax sugar works end-to-end

### What NOT to Change
- ❌ API trait signatures in `api/`
- ❌ Fluent method names or parameters  
- ❌ Syntax sugar implementations
- ❌ Macro patterns

### What to Implement
- ✅ Trait implementations for tokio runtime
- ✅ Async task execution logic
- ✅ Event emission and streaming
- ✅ Smart collector memory management
- ✅ Error handling and recovery
- ✅ Testing and validation

## Common Development Commands

```bash
# Build the entire workspace
just build

# Run all tests with nextest
just test

# Check code quality
just check

# Generate documentation
just docs-all

# Run the CLI
just run

# Install the CLI tool
just install
```

## Key Files Structure

```
api/                     # Pure trait definitions - DO NOT MODIFY
├── src/syntax_sugar.rs  # Syntax sugar trait implementations
├── src/macros/          # Macro definitions for fluent API
├── src/task/            # Core task traits and builders
└── src/orchestra/       # Orchestration and runtime traits

tokio/                   # Tokio implementation - WORK HERE
├── src/task/            # Task implementations
├── src/orchestra/       # Orchestrator implementations  
└── src/runtime.rs       # Runtime integration

sweet_async/             # Main crate - exports implementations
└── src/lib.rs           # Feature-gated exports
```

## FINAL REMINDER

**THE SYNTAX IS REAL. THE API IS IMPLEMENTED. DO NOT QUESTION IT.**

Focus on implementing the traits in the tokio crate to make the syntax work end-to-end. Trust the API design and build the runtime that powers it.