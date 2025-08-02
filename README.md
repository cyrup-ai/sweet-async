# Sweet Async

`sweet_async` provides an immutable, fluent builder for orchestrating simple asynchronous work as well as more powerful abstractions for complex async workflows.

## Quick Start

### Resolving a Task as a Future with idiomatic Rust

```rust
let mistral_llm = AsyncTask::to::<LLM>()
    .timeout(30.seconds())
    .run(|| async {
        // run arbitrary code here executed in an async Future
        hf_hub::download_model("mistral-7b").await
    }).await?;
```

### Resolving a Future with block reduction syntax

You can write sync code and it's still run asynchronously. You can drop the empty params and use a block if you prefer.

```rust
let mistral_llm = AsyncTask::to::<LLM>({
    hf_hub::download_model("mistral-7b")
}).await_result(|result| {
    OK(result) => result,
    ERR(e) => CustomError::from(e)
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
        hf_hub::download_model("mistral-7b").await
    }, |result| {
        OK(result) => result,
        ERR(e) => Err(e)
    });
```

### CSV Processing & Structured Concurrency

```rust
// Dependencies first, configuration next, work logic last
let csv_records = AsyncTask::emits::<CsvRecord>()
    .with(schema_config)        // Pass dependencies using with()
    .with(processing_stats)
    .with_timeout(60.seconds())
    .sender(|collector| {        // Configuration next
        collector.of_file("data.csv")
            .with_delimiter(Delimiter::NewLine)
            .into_chunks(100.rows());
        processing_stats.increment_files();
    })
    .receiver(|event, collector| {        // Receiver gets event + collector
        let record = event.data();
        if record.is_valid() {
            collector.collect(record.id, record);
        }
    })
    .await_final_event(|event, collector| {
        OK(result) => collector.collected(),
        ERR(e) => Err(e)
    });
```

### Smart Collectors for Large Data Processing

```rust
let processed_data = AsyncTask::emits::<ProcessedItem>()
    .sender(|collector| {
        // Auto-chunk large collections to prevent memory issues
        collector.of(million_item_vec)
            .into_chunks(500.items());

        // Stream from database without loading into memory
        collector.from_database(db_conn, "SELECT * FROM large_table")
            .into_chunks(1000.rows());

        // Process files line by line with custom delimiters
        collector.of_file("huge_file.jsonl")
            .with_delimiter(Delimiter::NewLine)
            .into_chunks(250.lines());

        // Concurrent API calls with batching
        collector.from_api_batch(api_urls)
            .into_chunks(50.requests());
    })
    .receiver(|event, collector| {
        let item = event.data();
        let processed = expensive_processing(item);
        collector.collect(processed.id, processed);
    })
    .await_final_event(|event, collector| {
        OK(result) => collector.collected(),
        ERR(e) => Err(e)
    });
```

### Sequential Processing with .sequence()

```rust
// For workflows that MUST run in order (migrations, ordered logs, etc.)
let migration_result = AsyncTask::emits::<MigrationStep>()
    .sequence()                           // Guarantees sequential execution
    .sender(|collector| {
        collector.of(migration_steps)
            .into_chunks(1.item());       // Process one at a time
    })
    .receiver(|event, collector| {
        let step = event.data();
        step.execute_migration().await;    // Each step waits for previous
        collector.collect(step.id, step.result);
    })
    .await_final_event(|event, collector| {
        OK(result) => collector.collected(),
        ERR(e) => Err(e)
    });
```

### HURL HTTP Integration

```rust
// BASIC HTTP
let response = AsyncTask::to::<ApiResponse>()
    .hurl({
        GET https://api.example.com/users/123
        Authorization: Bearer {{token}}
        Accept: application/json
    })
    .with_variables([("token", auth_token)])
    .await_result(|response| {
        OK(data) => parse_user(data),
        ERR(e) => create_default_user()
    });

// COMPLEX HTTP WORKFLOWS
let batch_results = AsyncTask::emits::<ApiResult>()
    .sender(|collector| {
        collector.of(user_ids)
            .into_chunks(10.items());
    })
    .receiver(|event, collector| {
        let user_id = event.data();
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
            ("user_id", user_id.to_string())
        ]).await;

        collector.collect(user_id, response);
    });
```

### Cloud Function Execution

```rust
// MULTI-PLATFORM CLOUD
let cloud_results = AsyncTask::emits::<ProcessedItem>()
    .sender(|collector| {
        collector.of(massive_dataset)
            .into_chunks(1000.items());
    })
    .lambda(|event, collector| {          // AWS Lambda
        let item = event.data();
        let result = heavy_computation(item);
        collector.collect(result.id, result);
    })
    .fallback_to_vercel(|event, collector| {  // Vercel Edge
        let item = event.data();
        let result = edge_computation(item);
        collector.collect(result.id, result);
    })
    .fallback_to_local(|event, collector| {   // Local development
        let item = event.data();
        let result = local_computation(item);
        collector.collect(result.id, result);
    })
    .with_auto_scaling(true)
    .with_timeout(300.seconds())
    .await_final_event(|event, collector| {
        OK(results) => collector.collected(),
        ERR(e) => Err(e)
    });
```

### Advanced Patterns

```rust
// VECTOR CLOCK SEQUENCING
let ordered_results = AsyncTask::emits::<TimestampedEvent>()
    .with_vector_clock(cluster_nodes)
    .sender(|collector| {
        collector.from_distributed_sources(node_streams)
            .with_causality_tracking(true)
            .into_chunks(100.events());
    })
    .receiver(|event, collector| {
        let ordered_event = event.data();
        process_with_causality(ordered_event);
        collector.collect(ordered_event.vector_clock, ordered_event);
    });

// CIRCUIT BREAKER PATTERNS
let resilient_processing = AsyncTask::emits::<ProcessedItem>()
    .with_circuit_breaker(CircuitBreaker::new()
        .failure_threshold(5)
        .timeout(Duration::from_secs(60))
        .half_open_max_calls(3))
    .sender(|collector| {
        collector.from_unreliable_source(external_api)
            .with_retry_strategy(RetryStrategy::ExponentialBackoff {
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(30),
                multiplier: 2.0
            })
            .into_chunks(50.requests());
    })
    .receiver(|event, collector| {
        let item = event.data();
        match risky_processing(item).await {
            Ok(result) => collector.collect(result.id, result),
            Err(e) => {
                // Circuit breaker handles failure tracking
                collector.collect_error(item.id, e);
            }
        }
    });
```

### Input Sources

```rust
.sender(|collector| {
    // YOUR BROWSER HISTORY (for digital archaeology)
    collector.of_browser_history(Browser::Chrome)
        .with_profile(BrowserProfile::Primary)
        .with_date_range(last_month)
        .into_chunks(1000.visits());

    collector.of_google_tasks(credentials)
        .acting_as("david@cyrup.ai")
        .with_due_date(DueDate::Today)
        .with_status(TaskStatus::uncompleted())
        .with_date_range(last_month)
        .order_by(TaskOrder::Priority.desc())
        .into_chunks(1000.tasks());

    // SLACK EMOJI REACTIONS (for team sentiment analysis)
    collector.of_slack_messages(credentials)
        .acting_as("david@cyrup.ai")
        .with_channel("general")
        .containing(["sick", "pto", "late"].any())
        .into_chunks(500.reactions());

    // LINKEDIN FLEXES (for cringe detection)
    collector.of_linkedin_posts(credentials)
        //.with_keyterm("hiring").stemmed().expanded())
        .with_llm("jobs for CTOs in the Bay Area".generate())
        .with_posted_date(DateRange::LastMonth)
        .ordered_by(PostOrder::Relevance.desc())
        .into_chunks(20.posts());

    // MEETING CALENDAR (for existential dread measurement)
    collector.of_calendar_events(CalendarProvider::Google)
        .with_credentials(credentials)
        .acting_as("david@cyrup.ai")
        .with_event_type(EventType::Meeting)
        .with_date_range(DateRange::Tomorrow)
        .ordered_by(EventOrder::StartTime.asc())
        .into_chunks(50.meetings());
})
```

### Social Media & Communication

```rust
.sender(|collector| {
    // TWITTER/X DUMPSTER FIRE
    collector.from_twitter_stream("@elonmusk")
        .feed_type(FeedType::Replies)
        .filtered_by(PublicSentiment::Negative)
        .filtered_by(TwitterFollowers::Minimum(10000))
        .in_time_range(TimeRange::LastHour)
        .ordered_by(ResultOrder::Influence.desc())
        .into_chunks(100.tweets());

    // DISCORD CHAOS
    collector.of_discord(credentials)
        .with_server("792347892349234")
        .with_generation(
            LLM::claude4_opus(Credential::env())
               .with_role(LLMRole::named("Discord Query Master"))
               .with_prompt("new ratatui project")
               .with_max_tokens(200)
               .with_format(Format::JsonArray)
        )
        .filtered_by(Upthumbs::min(20))
        .into_chunks(500.messages());
})
```

### Database Streaming

```rust
.sender(|collector| {
    // POSTGRESQL
    collector.from_database(postgres_conn, "SELECT * FROM huge_table")
        .with_cursor_size(1000)
        .into_chunks(1000.rows());

    // SURREALDB
    collector.from_surrealdb(surreal_conn, "SELECT * FROM records")
        .with_live_query(true)
        .into_chunks(500.records());

    // SEAORM
    collector.from_seaorm(sea_query)
        .with_batch_size(2000)
        .into_chunks(2000.entities());

    // SQLITE
    collector.from_sqlite(sqlite_conn, "SELECT * FROM logs ORDER BY timestamp")
        .with_streaming(true)
        .into_chunks(5000.rows());

    // MYSQL
    collector.from_mysql(mysql_conn, "SELECT * FROM events")
        .into_chunks(1500.rows());
})
```

### API & Network Sources

```rust
.sender(|collector| {
    // BATCH API CALLS
    collector.from_api_batch(url_list)
        .with_concurrency(10.concurrent_requests())
        .with_retry(3.attempts())
        .with_timeout(30.seconds())
        .into_chunks(25.responses());

    // WEBSOCKET STREAMS
    collector.from_websocket("wss://api.example.com/stream")
        .with_auth(bearer_token)
        .into_chunks(100.messages());

    // SSE STREAMS
    collector.from_sse("https://api.example.com/events")
        .with_headers(custom_headers)
        .into_chunks(50.events());

    // GRAPHQL SUBSCRIPTIONS
    collector.from_graphql_subscription(graphql_client, subscription_query)
        .into_chunks(25.updates());
})
```

### Cloud Storage

```rust
.sender(|collector| {
    // AWS S3
    collector.from_s3(bucket, "data/*.csv")
        .with_region(Region::UsEast1)
        .with_credentials(aws_creds)
        .into_chunks(50.objects());

    // GOOGLE CLOUD STORAGE
    collector.from_gcs(bucket, "logs/**/*.json")
        .with_service_account(gcp_creds)
        .into_chunks(25.objects());

    // AZURE BLOB STORAGE
    collector.from_azure_blob(container, "*.parquet")
        .with_connection_string(azure_conn)
        .into_chunks(100.blobs());
})
```

### Version Control

```rust
.sender(|collector| {
    // GITHUB REPOSITORIES
    collector.from_github("owner/repo", "data/*.json")
        .with_auth(github_token)
        .with_branch("main")
        .into_chunks(10.files());

    // GITLAB
    collector.from_gitlab("group/project", "configs/*.yaml")
        .with_token(gitlab_token)
        .into_chunks(20.files());

    // GIT REPOSITORIES
    collector.from_git_repo("https://github.com/owner/repo.git")
        .with_path("data/**/*.csv")
        .with_ref("main")
        .into_chunks(5.files());
})
```

### Message Queues & Streaming

```rust
.sender(|collector| {
    // KAFKA (COMPATIBLE)
    collector.from_kafka_topic("user-events")
        .with_consumer_group("processors")
        .with_offset(Offset::Latest)
        .with_partition_strategy(PartitionStrategy::RoundRobin)
        .into_chunks(100.messages());

    // REDIS STREAMS
    collector.from_redis_stream(redis_conn, "events:*")
        .with_consumer_group("workers")
        .into_chunks(50.entries());

    // RABBITMQ
    collector.from_rabbitmq(amqp_conn, "task_queue")
        .with_prefetch(100)
        .into_chunks(25.messages());

    // NATS
    collector.from_nats(nats_conn, "events.>")
        .with_queue_group("processors")
        .into_chunks(200.messages());
})
```

## Note

- All asynchronous methods return awaitable handles or results, never blocking the main thread.
- Smart collectors automatically handle memory management and streaming for large datasets.
- Chunk sizes are configured per data source for optimal performance.
- Configuration follows Dependencies → Config → Work pattern for clean separation of concerns.
- The API is designed for clarity and correctness — see the [docs](https://sweet-async.cyrup.ai) for trait and type details.

---

For more details, see the documentation at [sweet-async.cyrup.ai](https://sweet-async.cyrup.ai).
