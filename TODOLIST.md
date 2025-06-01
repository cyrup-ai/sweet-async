# Sweet Async TODO List

## 🚀 MVP Core Features

### API Foundation

- [ ] Simplify sender/receiver strategies to Serial/Parallel only
- [ ] Remove all parameter pollution from lambda signatures
- [ ] Implement `.with_mutable()` and `.with_immutable()` dependency injection
- [ ] Channel-based shared state (no Arc clones in hot path)
- [ ] Adaptive concurrency for parallel strategy (auto-scale workers)
- [ ] `.sequence()` method for order-dependent workflows

### Configuration Chain

- [ ] Dependencies first: `.with_mutable()`, `.with_immutable()`
- [ ] Configuration next: `.with_batch_size()`, `.with_timeout()`
- [ ] Work logic last: `.sender(|collector| ...)`, `.receiver(...)`
- [ ] Clean lambda signatures: `|collector|` and `|event, collector|`

### Smart Collectors (RAG Integration)

- [ ] `collector.file_by_line(file)` - Line-by-line file streaming
- [ ] `collector.of(big_vec)` - Auto-chunk large collections  
- [ ] `collector.from_stream(async_stream)` - Any async iterator
- [ ] `collector.from_database(conn, query)` - Database streaming
- [ ] `collector.from_api_batch(urls)` - Concurrent HTTP calls
- [ ] Leverage existing RAG chunking code

### Macro Infrastructure

- [ ] Compile-time dependency injection magic
- [ ] Zero-cost abstraction validation
- [ ] Type-safe builder pattern generation
- [ ] Channel setup automation

## 🌐 HURL Integration

### Core HURL Syntax

- [ ] `SweetAsync::to::<T>().hurl({ ... }).await?` syntax
- [ ] Macro that converts HURL DSL to HTTP client code
- [ ] Type-safe request/response handling
- [ ] Built-in timeout, retry, error handling

### Multi-Language Code Generation

- [ ] **Rust**: Native macro DSL (priority 1)
- [ ] **TypeScript**: Method chaining builders  
- [ ] **Go**: Fluent API patterns
- [ ] **Python**: Best effort support
- [ ] **C++**: Template-based builders

### HURL Features

- [ ] Bearer auth, basic auth, custom headers
- [ ] JSON body serialization/deserialization
- [ ] Status code validation
- [ ] Response capture with JSONPath
- [ ] Fallback/retry strategies

## ☁️ Cloud Orchestration

### Lambda Execution

- [ ] `.lambda(|data| { ... })` - Ship closures to AWS Lambda
- [ ] `.vercel(|request| { ... })` - Vercel Edge Functions
- [ ] `.local(|data| { ... })` - Local development fallback
- [ ] Automatic serialization/deserialization
- [ ] Error handling and retry logic

### Platform Integration

- [ ] AWS Lambda SDK integration
- [ ] Vercel Functions API
- [ ] Automatic packaging and deployment
- [ ] Environment detection and routing

## 📊 Kafka Compatible API

### Wire Protocol Compatibility

- [ ] Kafka producer/consumer protocol support
- [ ] Drop-in replacement for existing clients
- [ ] Topic and partition compatibility
- [ ] Consumer group semantics

### Performance Optimization

- [ ] Rust implementation (vs JVM overhead)
- [ ] Zero-copy where possible
- [ ] Adaptive backpressure
- [ ] Memory efficiency optimizations

### Operational Simplicity

- [ ] No ZooKeeper requirement
- [ ] Single binary deployment
- [ ] Built-in monitoring and metrics
- [ ] Configuration-free clustering

## 🔧 Database & Ecosystem Integration

### Database Support

- [ ] SurrealDB integration
- [ ] SeaORM compatibility
- [ ] Neon Postgres support
- [ ] Generic connection streaming

### File & Stream Support

- [ ] CSV processing optimizations
- [ ] JSON Lines support
- [ ] Directory traversal
- [ ] Compression support (gzip, etc.)

### Iterator Patterns

- [ ] Generic `Iterator<Item>` support
- [ ] Tokio channel integration
- [ ] Stream combinators

## 📈 Advanced Features

### Vector Clock Sequencing

- [ ] Distributed causality tracking
- [ ] Smart parallel execution with ordering constraints
- [ ] Cross-node coordination
- [ ] Event dependency resolution

### Monitoring & Observability

- [ ] Built-in metrics collection
- [ ] OpenTelemetry integration
- [ ] Performance profiling hooks
- [ ] Distributed tracing

### Error Handling & Recovery

- [ ] Structured error propagation
- [ ] Automatic retry with backoff
- [ ] Circuit breaker patterns
- [ ] Fallback value injection

## 📝 Documentation & Examples

### README Updates

- [ ] Update streaming example to use CSV processing
- [ ] Remove SenderStrategy parameters
- [ ] Add HURL integration examples
- [ ] Performance comparison with Kafka

### Documentation

- [ ] API reference generation
- [ ] Integration guides
- [ ] Performance benchmarks
- [ ] Migration guides from Kafka/HTTP clients

### Examples

- [ ] Ping tree implementation
- [ ] Large file processing
- [ ] API orchestration workflows
- [ ] Database ETL pipelines

## 🎯 Commercial Preparation

### Market Positioning

- [ ] Kafka compatible API messaging
- [ ] Modal.com comparison (cloud compute)
- [ ] Performance benchmarking suite
- [ ] Customer case studies

### Enterprise Features

- [ ] Security audit trail
- [ ] Enterprise authentication
- [ ] Compliance reporting
- [ ] SLA monitoring

### Packaging & Distribution

- [ ] Multi-language package managers
- [ ] Docker containers
- [ ] Cloud marketplace listings
- [ ] Enterprise licensing

## 🧪 Testing & Validation

### Performance Testing

- [ ] Kafka comparison benchmarks
- [ ] Memory usage profiling
- [ ] Latency distribution analysis
- [ ] Stress testing under load

### Integration Testing

- [ ] Multi-language client compatibility
- [ ] Cloud platform integration
- [ ] Database connector validation
- [ ] Real-world workflow testing

### Developer Experience

- [ ] IDE integration and autocomplete
- [ ] Error message quality
- [ ] Learning curve analysis
- [ ] Documentation completeness

---

## Priority Order

1. **MVP Core Features** - Foundation for everything else
2. **HURL Integration** - Differentiating feature for HTTP workflows  
3. **Kafka Compatible API** - Immediate commercial opportunity
4. **Cloud Orchestration** - Revolutionary distributed computing
5. **Advanced Features** - Enterprise and scale requirements

**Target: Build and ship MVP within 3 months, iterate based on customer feedback.**
