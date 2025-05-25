# Sweet Async Tokio Tests

This directory contains integration tests for the Sweet Async Tokio implementation.

## Running Tests

Run all tests with nextest:
```bash
cargo nextest run
```

Run a specific test:
```bash
cargo nextest run test_basic_api
```

Run tests with output:
```bash
cargo nextest run --nocapture
```

## Test Organization

- `common/` - Shared test utilities and helpers
- `test_basic_api.rs` - Basic API functionality tests
- `test_emit_api.rs` - Event emission API tests
- `test_full_api.rs` - Comprehensive API tests
- `metrics_aggregation.rs` - Distributed metrics aggregation tests
- `vector_clock.rs` - Vector clock implementation tests

## Writing New Tests

1. Create a new test file in this directory
2. Import common utilities with `mod common;`
3. Use `#[tokio::test]` for async tests
4. Follow the naming convention: `test_<feature>.rs`

## Test Guidelines

- Each test should be independent and not rely on shared state
- Use descriptive test names that explain what is being tested
- Add comments explaining complex test scenarios
- Clean up any resources created during tests
- Use the common module for shared test utilities