# Sweet Async Builder Architecture Flowchart

This document maps the complete Sweet Async immutable builder architecture, showing all pathways from initial creation to execution.

## Core Principle: Immutable Builder Pattern

**CRITICAL**: Nothing executes until `.await`, `.await_result()`, or `.await_final_event()` is called. All builder methods return NEW instances with additional configuration.

```
Builder Chain → Builder Chain → Builder Chain → EXECUTION (.await)
   (config)      (more config)    (final config)    (async work happens)
```

## Entry Points & Main Pathways

### 1. Basic Task Execution Pathway

```
AsyncTask::to::<ReturnType>()
│
├─ Configuration Methods (return new builders)
│  ├─ .timeout(Duration)
│  ├─ .with(dependency: T)
│  ├─ .with_timeout(Duration) 
│  ├─ .hurl({ HTTP config })
│  ├─ .lambda(|event, collector| { ... })
│  ├─ .fallback_to_vercel(|event, collector| { ... })
│  ├─ .fallback_to_local(|event, collector| { ... })
│  ├─ .with_auto_scaling(bool)
│  ├─ .with_vector_clock(Vec<Node>)
│  └─ .with_circuit_breaker(CircuitBreaker)
│
└─ EXECUTION ENDPOINTS (actually run async work)
   ├─ .run(|| async { work }).await → Future<Output = T>
   ├─ .await_result(work, handler).await → T
   └─ .await_result({ block }, handler).await → T
```

### 2. Event Emission Pathway  

```
AsyncTask::emits::<EventType>()
│
├─ Configuration Methods (return new builders)
│  ├─ .sequence() // Sequential processing guarantee
│  ├─ .with(dependency: T)
│  ├─ .with_timeout(Duration)
│  ├─ .lambda(|event, collector| { ... }) // AWS Lambda
│  ├─ .fallback_to_vercel(|event, collector| { ... })
│  ├─ .fallback_to_local(|event, collector| { ... })
│  ├─ .with_auto_scaling(bool)
│  ├─ .with_vector_clock(Vec<Node>)
│  └─ .with_circuit_breaker(CircuitBreaker)
│
├─ Stream Configuration
│  ├─ .sender(|collector| { /* data source config */ })
│  └─ .receiver(|event, collector| { /* processing logic */ })
│
└─ EXECUTION ENDPOINT (actually run async stream)
   └─ .await_final_event(|event, collector| { handler }).await → Stream/Collection
```

## Data Source Configuration (within .sender())

When configuring `.sender(|collector| { ... })`, the collector provides these fluent methods:

### Basic Sources
```
collector
├─ .of(Vec<Item>) // Direct collection
├─ .of_file("path/to/file.csv") 
└─ .from_database(connection, "SELECT * FROM table")
```

### Cloud Storage Sources
```
collector
├─ .from_s3(bucket, "data/*.csv")
│  ├─ .with_region(Region::UsEast1)
│  ├─ .with_credentials(aws_creds)
│  └─ .into_chunks(50.objects())
│
├─ .from_gcs(bucket, "logs/**/*.json")
│  ├─ .with_service_account(gcp_creds)
│  └─ .into_chunks(25.objects())
│
└─ .from_azure_blob(container, "*.parquet")
   ├─ .with_connection_string(azure_conn)
   └─ .into_chunks(100.blobs())
```

### Database Sources
```
collector
├─ .from_surrealdb(conn, "SELECT * FROM records")
│  ├─ .with_live_query(true)
│  └─ .into_chunks(500.records())
│
├─ .from_seaorm(sea_query)
│  ├─ .with_batch_size(2000)
│  └─ .into_chunks(2000.entities())
│
├─ .from_sqlite(conn, "SELECT * FROM logs")
│  ├─ .with_streaming(true)
│  └─ .into_chunks(5000.rows())
│
└─ .from_mysql(conn, "SELECT * FROM events")
   └─ .into_chunks(1500.rows())
```

### API & Network Sources
```
collector
├─ .from_api_batch(url_list)
│  ├─ .with_concurrency(10.concurrent_requests())
│  ├─ .with_retry(3.attempts())
│  ├─ .with_timeout(30.seconds())
│  └─ .into_chunks(25.responses())
│
├─ .from_websocket("wss://api.example.com/stream")
│  ├─ .with_auth(bearer_token)
│  └─ .into_chunks(100.messages())
│
├─ .from_sse("https://api.example.com/events")
│  ├─ .with_headers(custom_headers)
│  └─ .into_chunks(50.events())
│
└─ .from_graphql_subscription(client, query)
   └─ .into_chunks(25.updates())
```

### Social Media & Communication Sources
```
collector
├─ .of_browser_history(Browser::Chrome)
│  ├─ .with_profile(BrowserProfile::Primary)
│  ├─ .with_date_range(last_month)
│  └─ .into_chunks(1000.visits())
│
├─ .of_google_tasks(credentials)
│  ├─ .acting_as("user@domain.com")
│  ├─ .with_due_date(DueDate::Today)
│  ├─ .with_status(TaskStatus::uncompleted())
│  ├─ .order_by(TaskOrder::Priority.desc())
│  └─ .into_chunks(1000.tasks())
│
├─ .of_slack_messages(credentials)
│  ├─ .with_channel("general")
│  ├─ .containing(["sick", "pto", "late"].any())
│  └─ .into_chunks(500.reactions())
│
├─ .of_linkedin_posts(credentials)
│  ├─ .with_llm("jobs for CTOs in the Bay Area".generate())
│  ├─ .with_posted_date(DateRange::LastMonth)
│  ├─ .ordered_by(PostOrder::Relevance.desc())
│  └─ .into_chunks(20.posts())
│
└─ .of_calendar_events(CalendarProvider::Google)
   ├─ .with_credentials(credentials)
   ├─ .acting_as("user@domain.com")
   ├─ .with_event_type(EventType::Meeting)
   ├─ .with_date_range(DateRange::Tomorrow)
   ├─ .ordered_by(EventOrder::StartTime.asc())
   └─ .into_chunks(50.meetings())
```

### Message Queues & Streaming
```
collector
├─ .from_kafka_topic("user-events")
│  ├─ .with_consumer_group("processors")
│  ├─ .with_offset(Offset::Latest)
│  ├─ .with_partition_strategy(PartitionStrategy::RoundRobin)
│  └─ .into_chunks(100.messages())
│
├─ .from_redis_stream(conn, "events:*")
│  ├─ .with_consumer_group("workers")
│  └─ .into_chunks(50.entries())
│
├─ .from_rabbitmq(conn, "task_queue")
│  ├─ .with_prefetch(100)
│  └─ .into_chunks(25.messages())
│
└─ .from_nats(conn, "events.>")
   ├─ .with_queue_group("processors")
   └─ .into_chunks(200.messages())
```

### Version Control Sources
```
collector
├─ .from_github("owner/repo", "data/*.json")
│  ├─ .with_auth(github_token)
│  ├─ .with_branch("main")
│  └─ .into_chunks(10.files())
│
├─ .from_gitlab("group/project", "configs/*.yaml")
│  ├─ .with_token(gitlab_token)
│  └─ .into_chunks(20.files())
│
└─ .from_git_repo("https://github.com/owner/repo.git")
   ├─ .with_path("data/**/*.csv")
   ├─ .with_ref("main")
   └─ .into_chunks(5.files())
```

## Type Flow Through Builder Chain

### Basic Task Execution Types
```
AsyncTask::to::<LLM>()                    // TaskBuilder<(), DefaultTaskId>
    .timeout(30.seconds())                // TaskBuilder<LLM, DefaultTaskId> 
    .run(|| async { download_model() })   // TaskFuture<LLM>
    .await                                // Result<LLM, AsyncTaskError>
```

### Event Emission Types  
```
AsyncTask::emits::<CsvRecord>()           // EmittingTaskBuilder<CsvRecord, (), (), DefaultTaskId>
    .sender(|collector| { ... })          // SenderTaskBuilder<CsvRecord, C, E, DefaultTaskId>
    .receiver(|event, collector| { ... }) // ReceiverTaskBuilder<CsvRecord, C, E, DefaultTaskId>
    .await_final_event(|event, collector| { ... }) // EmittingTaskFuture<CsvRecord, C, E>
    .await                                // FinalEvent<CsvRecord, C, C>
```

## Execution vs Configuration Phases

### Configuration Phase (Immutable Builders)
- **No async work happens**  
- **Returns new builder instances**
- **Stores configuration for later execution**
- **Pure function composition**

```rust
let configured_task = AsyncTask::to::<LLM>()  // Returns builder
    .timeout(30.seconds())                    // Returns NEW builder 
    .with(api_key);                          // Returns NEW builder
// NO EXECUTION YET - just configured builders
```

### Execution Phase (Async Runtime)
- **Triggered by `.await`, `.await_result()`, `.await_final_event()`**
- **Actually runs async work using Tokio**
- **Consumes builder and returns Future/Stream**

```rust
let result = configured_task
    .run(|| async { download_model().await }) // Returns Future<Result<LLM, Error>>
    .await;                                   // EXECUTES HERE - async work happens!
```

## Advanced Patterns

### Cloud Function Execution (Multi-Platform)
```
AsyncTask::emits::<ProcessedItem>()
├─ .sender(|collector| collector.of(dataset).into_chunks(1000.items()))
├─ .lambda(|event, collector| { /* AWS Lambda */ })        // PRIMARY
├─ .fallback_to_vercel(|event, collector| { /* Vercel */ }) // FALLBACK 1  
├─ .fallback_to_local(|event, collector| { /* Local */ })  // FALLBACK 2
├─ .with_auto_scaling(true)
├─ .with_timeout(300.seconds())
└─ .await_final_event(|event, collector| collector.collected()).await
```

### Vector Clock Sequencing (Distributed Systems)
```
AsyncTask::emits::<TimestampedEvent>()
├─ .with_vector_clock(cluster_nodes)
├─ .sender(|collector| {
│      collector.from_distributed_sources(node_streams)
│          .with_causality_tracking(true)
│          .into_chunks(100.events())
│  })
└─ .receiver(|event, collector| {
       process_with_causality(event.data());
       collector.collect(event.vector_clock, event.data());
   })
```

### Circuit Breaker Patterns (Fault Tolerance)
```
AsyncTask::emits::<ProcessedItem>()
├─ .with_circuit_breaker(CircuitBreaker::new()
│      .failure_threshold(5)
│      .timeout(Duration::from_secs(60))
│      .half_open_max_calls(3))
├─ .sender(|collector| {
│      collector.from_unreliable_source(external_api)
│          .with_retry_strategy(RetryStrategy::ExponentialBackoff {
│              initial_delay: Duration::from_millis(100),
│              max_delay: Duration::from_secs(30),
│              multiplier: 2.0
│          })
│          .into_chunks(50.requests())
│  })
└─ .receiver(|event, collector| {
       match risky_processing(event.data()).await {
           Ok(result) => collector.collect(result.id, result),
           Err(e) => collector.collect_error(event.data().id, e)
       }
   })
```

## HTTP Integration with HURL

### Basic HTTP
```
AsyncTask::to::<ApiResponse>()
├─ .hurl({
│      GET https://api.example.com/users/123
│      Authorization: Bearer {{token}}
│      Accept: application/json
│  })
├─ .with_variables([("token", auth_token)])
└─ .await_result(|response| {
       OK(data) => parse_user(data),
       ERR(e) => create_default_user()
   }).await
```

### Complex HTTP Workflows  
```
AsyncTask::emits::<ApiResult>()
├─ .sender(|collector| collector.of(user_ids).into_chunks(10.items()))
└─ .receiver(|event, collector| {
       let response = hurl_request!({
           POST https://api.example.com/enrich
           Authorization: Bearer {{token}}
           Content-Type: application/json
           
           {
               "user_id": {{user_id}},
               "include_profile": true
           }
       }).with_variables([
           ("token", auth_token),
           ("user_id", event.data().to_string())
       ]).await;
       
       collector.collect(event.data(), response);
   })
```

## Key Architectural Insights

1. **Immutable Builders**: Every method returns a NEW instance, never mutating existing ones
2. **Lazy Execution**: No async work until explicit execution methods are called  
3. **Polymorphic Chains**: Different builder types based on configuration path
4. **Type Safety**: Generic types flow through the chain maintaining compile-time safety
5. **Fluent API**: Method chaining with natural language-like configuration
6. **Async Throughout**: When execution happens, everything is 100% async via Tokio

This architecture enables the "synchronous method signatures that return awaitable futures" pattern while maintaining full async behavior and compile-time type safety.