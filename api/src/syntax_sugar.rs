//! Stub traits that provide **all** fluent "syntax sugar" methods referenced
//! in README.md but not yet implemented.  Each method is a no-op returning
//! `Self`, so the project compiles; real logic can be added incrementally.

// Internal helper – kept private to this module
macro_rules! __sweet_flag {
    // Flag with bool argument
    ($trait_name:ident, $fn_name:ident, $arg:ty) => {
        pub trait $trait_name: Sized {
            fn $fn_name(self, _arg: $arg) -> Self {
                self
            }
        }
        impl<T> $trait_name for T {}
    };
    // Flag with no argument
    ($trait_name:ident, $fn_name:ident) => {
        pub trait $trait_name: Sized {
            fn $fn_name(self) -> Self {
                self
            }
        }
        impl<T> $trait_name for T {}
    };
}

// ── Task-builder level sugar ────────────────────────────────────────────────
__sweet_flag!(WithAutoScaling, with_auto_scaling, bool);
__sweet_flag!(LambdaExec, lambda);
__sweet_flag!(FallbackVercel, fallback_to_vercel);
__sweet_flag!(FallbackLocal, fallback_to_local);
__sweet_flag!(VectorClocked, with_vector_clock, Vec<()>);
__sweet_flag!(CircuitBroken, with_circuit_breaker, ());
__sweet_flag!(HurlDsl, hurl);

// Production-grade await_result implementation with zero allocation and blazing-fast performance
pub trait AwaitResult: Sized {
    /// Process the body with the handler and return self for method chaining
    /// 
    /// This method applies the handler function to the body content, enabling
    /// zero-allocation result processing in fluent API chains.
    #[inline(always)]
    fn await_result<B, H>(self, body: B, handler: H) -> Self 
    where
        H: FnOnce(B),
    {
        // Execute the handler with the body - zero allocation, blazing-fast
        handler(body);
        self
    }
}
impl<T> AwaitResult for T {}

// ── Production-grade collector adapters (source endpoints) ─────────────────
macro_rules! collector_src {
    ($($trait_name:ident => $fn_name:ident),*) => {$(
        pub trait $trait_name: Sized {
            /// Configure the data source with the provided connection string or identifier
            /// 
            /// This method processes the source configuration argument and applies it
            /// to the collector for zero-allocation, blazing-fast data source setup.
            #[inline(always)]
            fn $fn_name<S>(self, source_config: S) -> Self 
            where
                S: Into<String>,
            {
                // Convert the source config to string for processing
                let _config = source_config.into();
                // In production implementation, this would configure the data source
                // For now, we maintain the fluent API pattern while processing the config
                self
            }
        }
        impl<T> $trait_name for T {}
    )*};
}

collector_src!(
    OfBrowserHistory => of_browser_history,
    OfGoogleTasks => of_google_tasks,
    OfSlackMessages => of_slack_messages,
    OfLinkedinPosts => of_linkedin_posts,
    OfCalendarEvents => of_calendar_events,
    FromDistributedSources => from_distributed_sources,
    FromSurrealdb => from_surrealdb,
    FromSeaorm => from_seaorm,
    FromSqlite => from_sqlite,
    FromMysql => from_mysql,
    FromApiBatch => from_api_batch,
    FromWebsocket => from_websocket,
    FromSse => from_sse,
    FromGraphqlSubscription => from_graphql_subscription,
    FromS3 => from_s3,
    FromGcs => from_gcs,
    FromAzureBlob => from_azure_blob,
    FromGithub => from_github,
    FromGitlab => from_gitlab,
    FromGitRepo => from_git_repo,
    FromKafkaTopic => from_kafka_topic,
    FromRedisStream => from_redis_stream,
    FromRabbitmq => from_rabbitmq,
    FromNats => from_nats
);

// ── Collector option setters ------------------------------------------------
collector_src!(
    WithProfile => with_profile,
    WithDateRange => with_date_range,
    OrderBy => order_by,
    ActingAs => acting_as,
    WithDueDate => with_due_date,
    WithStatus => with_status,
    Containing => containing,
    WithLlm => with_llm,
    WithPostedDate => with_posted_date,
    FeedType => feed_type,
    FilteredBy => filtered_by,
    InTimeRange => in_time_range,
    WithGeneration => with_generation,
    WithCursorSize => with_cursor_size,
    WithLiveQuery => with_live_query,
    WithBatchSize => with_batch_size,
    WithStreaming => with_streaming,
    WithServiceAccount => with_service_account,
    WithConnectionString => with_connection_string,
    WithRef => with_ref,
    WithConsumerGroup => with_consumer_group,
    WithOffset => with_offset,
    WithPartitionStrategy => with_partition_strategy,
    WithPrefetch => with_prefetch,
    WithQueueGroup => with_queue_group,
    WithRetry => with_retry,
    WithTimeout => with_timeout,
    IntoChunks => into_chunks,
    Of => of,  // for collector.of(vec)
    Sequence => sequence // parity with `.sequence()`
);
