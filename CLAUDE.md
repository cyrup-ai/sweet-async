# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

```bash
# Build the entire workspace
just build

# Run all tests with nextest
just test

# Check code quality (format, check, clippy)
just check

# Generate all documentation
just docs-all

# Clean build artifacts
just clean

# Run the sweet_async CLI
just run

# Install the CLI tool
just install

# Upgrade dependencies
just upgrade

# Run a single test
cargo nextest run <test_name>

# Build specific crate with features
cargo build --release --features tokio --manifest-path crates/sweet_async/Cargo.toml
```

## Architecture Overview

Sweet Async is a Rust library providing an immutable, fluent builder API for orchestrating asynchronous work. It offers powerful abstractions for complex async workflows while maintaining a clean, intuitive interface.

### Core Design Principles

1. **100% Asynchronous**: Despite having synchronous method signatures, all operations return futures/streams and execute asynchronously
2. **Fluent Builder Pattern**: Chainable API for intuitive task configuration
3. **Zero Blocking**: No `block_on` usage in async contexts - uses atomics and try_lock patterns
4. **Structured Concurrency**: Built-in support for managing related async tasks
5. **Memory-Safe Streaming**: Smart collectors handle large datasets without memory issues

### Workspace Structure

- `api/`: Core trait definitions and contracts for the Sweet Async API
- `sweet_async/`: Main crate with CLI and library exports
- `tokio/`: Tokio-based implementation of the async runtime
- `aws/`, `shuttle/`, `spin/`, `vercel/`: Cloud platform integrations
- `zkp/`: Zero-knowledge proof integration (future)
- `telemetry/`: OpenTelemetry integration

### Key API Patterns

1. **Basic Task Execution**:
   - `AsyncTask::to::<T>()` - Creates a task builder
   - `.timeout()`, `.with()` - Configure the task
   - `.run()` - Returns a Future (not blocking!)
   - `.await` - Execute asynchronously

2. **Event Emission Pattern**:
   - `AsyncTask::emits::<T>()` - Creates an event emitter
   - `.sender()` - Define how events are generated
   - `.receiver()` - Define how events are processed
   - `.await_final_event()` - Handle completion

3. **Smart Collectors**:
   - Automatic chunking for large datasets
   - Streaming from files, databases, APIs
   - Memory-efficient processing
   - Built-in error handling

### Critical Implementation Details

- **Async Model**: Methods with sync signatures return `impl Future` or `impl Stream`
- **No Panics**: Avoid `unwrap()`, `expect()`, or `panic!()` in production code
- **Error Handling**: All operations return `Result<T, E>`
- **Testing**: Use `cargo nextest` for async test execution
- **Dependencies**: External cryypt dependencies require relative paths

### Testing Strategy

- Unit tests in `src/` modules test individual components
- Integration tests in `tests/` verify cross-module behavior
- Use `cargo nextest run` for parallel test execution
- Mock external dependencies in tests

### Documentation

- API docs: `cargo doc --workspace --all-features --no-deps`
- Wiki docs: `just docs-wiki` (requires Zola)
- User guide: `just docs-guide` (requires mdBook)
- All docs: `just docs-all`

### Development Workflow

1. Make changes following the builder pattern conventions
2. Run `just check` to ensure code quality
3. Run `just test` to verify functionality
4. Update documentation if APIs change
5. Use atomic operations for state that needs synchronous access