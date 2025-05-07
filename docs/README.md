# Async Task Documentation

This directory contains documentation for the Async Task system.

## Contents

- [WEAVER.md](WEAVER.md) - Documentation generation with OpenTelemetry Weaver
- [CONVENTIONS.md](CONVENTIONS.md) - Coding conventions and practices
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture and design decisions

## Overview

The Async Task system provides a robust framework for managing asynchronous tasks in Rust applications. It focuses on clean API design that hides async complexity while providing comprehensive observability, error handling, and cancellation capabilities.

Key features include:

- **Intuitive API** - Clean interface that hides async complexity
- **Flexible Task Prioritization** - Granular control over task execution priority
- **Structured Concurrency** - Parent-child task relationships with automatic cleanup
- **Robust Cancellation** - Graceful shutdown with escalation capabilities
- **Comprehensive Observability** - CPU, memory, and IO metrics using OpenTelemetry
- **Error Handling** - Rich context-aware error reporting

## Getting Started

See [CONVENTIONS.md](CONVENTIONS.md) for our approach to async code and implementation best practices.

For integration with observability tools and documentation generation, see [WEAVER.md](WEAVER.md).

## License

This project is licensed under the [MIT License](../LICENSE).
