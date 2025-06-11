//! Stub traits that provide **all** fluent "syntax sugar" methods referenced
//! in README.md but not yet implemented.  Each method is a no-op returning
//! `Self`, so the project compiles; real logic can be added incrementally.

// Internal helper – kept private to this module
macro_rules! __sweet_flag {
    // Flag with bool argument
    ($trait_name:ident, $fn_name:ident, $arg:ty) => {
        pub trait $trait_name: Sized {
            fn $fn_name(self, _arg: $arg) -> Self { self }
        }
        impl<T> $trait_name for T {}
    };
    // Flag with no argument
    ($trait_name:ident, $fn_name:ident) => {
        pub trait $trait_name: Sized {
            fn $fn_name(self) -> Self { self }
        }
        impl<T> $trait_name for T {}
    };
}

// ── Task-builder level sugar ────────────────────────────────────────────────
__sweet_flag!(WithAutoScaling,  with_auto_scaling, bool);
__sweet_flag!(LambdaExec,       lambda,            fn(_, _));
__sweet_flag!(FallbackVercel,   fallback_to_vercel, fn(_, _));
__sweet_flag!(FallbackLocal,    fallback_to_local,  fn(_, _));
__sweet_flag!(VectorClocked,    with_vector_clock,  Vec<()>);
__sweet_flag!(CircuitBroken,    with_circuit_breaker, ());
__sweet_flag!(HurlDsl,          hurl,              fn(_)->());

// await_result must remain `async`; we expose a stub that just returns self.
pub trait AwaitResult: Sized {
    fn await_result<B, H>(self, _body: B, _handler: H) -> Self { self }
}
impl<T> AwaitResult for T {}

// ── Collector adapters (source endpoints) ───────────────────────────────────
macro_rules! collector_src {
    ($($fn_name:ident),*) => {$(
        pub trait $fn_name: Sized {
            fn $fn_name(self, _arg: impl Into<String>) -> Self { self }
        }
        impl<T> $fn_name for T {}
    )*};
}

collector_src!(
    of_browser_history,
    of_google_tasks,
    of_slack_messages,
    of_linkedin_posts,
    of_calendar_events,
    from_distributed_sources,
    from_surrealdb,
    from_seaorm,
    from_sqlite,
    from_mysql,
    from_api_batch,
    from_websocket,
    from_sse,
    from_graphql_subscription,
    from_s3,
    from_gcs,
    from_azure_blob,
    from_github,
    from_gitlab,
    from_git_repo,
    from_kafka_topic,
    from_redis_stream,
    from_rabbitmq,
    from_nats
);

// ── Collector option setters ------------------------------------------------
collector_src!(
    with_profile,
    with_date_range,
    order_by,
    acting_as,
    with_due_date,
    with_status,
    containing,
    with_llm,
    with_posted_date,
    feed_type,
    filtered_by,
    in_time_range,
    with_generation,
    with_cursor_size,
    with_live_query,
    with_batch_size,
    with_streaming,
    with_service_account,
    with_connection_string,
    with_ref,
    with_consumer_group,
    with_offset,
    with_partition_strategy,
    with_prefetch,
    with_queue_group,
    with_retry,
    with_timeout,
    into_chunks,
    of,  // for collector.of(vec)
    sequence // parity with `.sequence()`
);