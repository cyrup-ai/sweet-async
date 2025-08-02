# STREAMING COLLECTORS IMPLEMENTATION

**OBJECTIVE**: Implement complete zero-allocation, blazing-fast, lock-free streaming collector system for Sweet Async

**SUCCESS CRITERIA**: CSV example compiles and executes with full streaming collector functionality

## PHASE 1: CORE TRAIT IMPLEMENTATION

### 1.1 Fix EmittingTaskBuilder Implementation
**File**: `src/task/emit/channel_builder.rs`
**Lines**: 577-582, 647-652, 501-574
**Details**: 
- Remove extra trait bounds (`Sync`, `From<AsyncTaskError>`, `Default`, `From<Uuid>`)
- Copy exact trait signatures from `../api/src/task/emit/builder.rs`
- EmittingTaskBuilder bounds: `T: Clone + Send + 'static, C: Send + 'static, E: Send + 'static, I: TaskId`
- SenderBuilder bounds: Same as above
- ReceiverBuilder bounds: Same as above
- Method signatures must match API exactly: `sender<F>(self, sender: F, strategy: SenderStrategy)`

### 1.2 Implement Correct EmittingTask Trait  
**File**: `src/task/emit/task.rs`
**Lines**: 32-47, 584-590
**Details**:
- Copy exact `EmittingTask` trait from `../api/src/task/emit/task.rs`
- `await_final_event<Handler, R>(self, handler: Handler) -> R`
- Handler signature: `Fn(Self::Final, &dyn std::any::Any) -> R + Send + 'static`
- Implement `is_complete()` and `cancel()` methods
- Connect to StreamCollector system

## PHASE 2: HIGH-PERFORMANCE SOURCE ADAPTERS

### 2.1 Implement of_file() Source Adapter
**File**: `src/task/emit/sources/file_source.rs` (new file)
**Details**:
- Async file streaming with tokio::fs::File and BufReader
- Zero-allocation line reading with reusable buffers
- Configurable read buffer size (default 64KB)
- Support for different delimiters (NewLine, Comma, Custom)
- Memory pool for line buffers to avoid allocations
- Error handling without unwrap()/expect()

### 2.2 Implement from_database() Source Adapter  
**File**: `src/task/emit/sources/database_source.rs` (new file)
**Details**:
- Generic database connection interface
- Cursor-based streaming for large result sets
- Connection pooling integration
- Prepared statement optimization
- Row batching for efficiency
- Support for PostgreSQL, MySQL, SQLite

### 2.3 Implement from_api_batch() Source Adapter
**File**: `src/task/emit/sources/api_source.rs` (new file)  
**Details**:
- Concurrent HTTP requests with configurable concurrency
- Adaptive backpressure based on response times
- Connection pool reuse
- Retry logic with exponential backoff
- Request/response buffering
- Rate limiting support

### 2.4 Implement of() Vector Source Adapter
**File**: `src/task/emit/sources/vector_source.rs` (new file)
**Details**:
- Zero-copy vector chunking
- SIMD-optimized iteration where applicable
- Iterator-based approach for memory efficiency  
- Support for different item types
- Chunk size adaptation based on processing speed

## PHASE 3: ZERO-ALLOCATION CHUNKING SYSTEM

### 3.1 Build Smart Chunking Engine
**File**: `src/task/emit/chunking/mod.rs` (new file)
**Details**:
- ChunkSize enum: Rows(usize), Items(usize), Bytes(usize), Lines(usize)
- Adaptive chunking based on data characteristics
- Memory pool for chunk buffers
- NUMA-aware allocation strategies
- Lock-free chunk queue
- Backpressure handling

### 3.2 Implement Extension Traits for Chunking
**File**: `src/task/mod.rs` 
**Lines**: Add after existing extension traits
**Details**:
- `trait RowsExt { fn rows(self) -> ChunkSize; }`
- `trait ItemsExt { fn items(self) -> ChunkSize; }`  
- `trait LinesExt { fn lines(self) -> ChunkSize; }`
- `trait BytesExt { fn bytes(self) -> ChunkSize; }`
- Implement for usize with #[inline] for zero-cost abstractions

## PHASE 4: EVENT PIPELINE INTEGRATION

### 4.1 Implement StreamingEvent for CSV Records
**File**: `src/task/emit/csv_event.rs` (new file)
**Details**:
- Implement StreamingEvent trait from API
- Zero-allocation event creation
- Timestamp management with atomic clocks
- Event ID generation with UUID v4
- Task ID correlation
- Event type handling (Final, Continue, Error, Cancellation)

### 4.2 Connect Chunking to Event Emission
**File**: `src/task/emit/channel_builder.rs`
**Lines**: 215-332 (await_final_event method)
**Details**:
- Replace current file processing with source adapter system
- Integrate chunking engine
- Event pipeline: Source → Chunks → Events → Receiver → Collection
- Lock-free event queuing with mpsc channels
- Backpressure propagation

## PHASE 5: PERFORMANCE OPTIMIZATIONS

### 5.1 Apply Hot Path Optimizations
**Files**: All implementation files
**Details**:
- Add #[inline] to all performance-critical methods
- Use #[cold] for error handling paths
- Profile-guided optimization hints
- Branch prediction optimization with likely/unlikely
- SIMD instructions for data parsing where applicable

### 5.2 Implement Lock-Free Metrics
**File**: `src/task/emit/metrics.rs` (new file)
**Details**:
- Atomic counters for throughput metrics
- Lock-free histogram for latency tracking
- Memory usage tracking without allocations
- Processing rate adaptation
- Error rate monitoring

### 5.3 Zero-Allocation Error Handling
**Files**: All implementation files  
**Details**:
- Custom error types that implement Error trait
- Error propagation without heap allocations
- Result<T, E> patterns throughout
- Error context preservation
- Structured error information

## PHASE 6: FINAL INTEGRATION AND VALIDATION

### 6.1 Complete CSV Example Integration
**File**: `examples/csv.rs`
**Details**:
- Ensure all syntax works: `.sender()`, `.receiver()`, `.await_final_event()`
- Verify collector.collect() and collector.collected() work
- Test with real CSV file processing
- Validate zero-allocation operation
- Performance benchmarking

### 6.2 API Compatibility Verification
**Files**: All trait implementations
**Details**:
- Verify exact match with API trait signatures
- Test all method combinations from README examples
- Ensure no extra trait bounds
- Validate return types match expectations
- Integration test with all source adapters