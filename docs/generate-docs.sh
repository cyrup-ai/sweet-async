#!/bin/bash
#
# Generate beautiful documentation for the async_task crates
#

# Set the root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_CRATE_DIR="$ROOT_DIR/crates/api"
DOCS_DIR="$ROOT_DIR/docs/generated"

# Create the docs directory if it doesn't exist
mkdir -p "$DOCS_DIR"

# Install cargo-doc-all if not already installed
if ! command -v cargo-doc-all &> /dev/null; then
  echo "Installing cargo-doc-all..."
  cargo install cargo-doc-all
fi

# Install mdbook for the guide if not already installed
if ! command -v mdbook &> /dev/null; then
  echo "Installing mdbook..."
  cargo install mdbook
fi

# Clean any previous docs
echo "Cleaning previous docs..."
rm -rf "$DOCS_DIR/api"
rm -rf "$DOCS_DIR/guide"

# Generate API documentation with a custom theme
echo "Generating API documentation..."
cd "$API_CRATE_DIR" || exit 1
RUSTDOCFLAGS="--cfg docsrs" cargo doc --document-private-items --no-deps --all-features

# Copy the generated docs to the docs directory
mkdir -p "$DOCS_DIR/api"
cp -r "$API_CRATE_DIR/target/doc/." "$DOCS_DIR/api/"

# Generate cross-crate documentation for the entire workspace
echo "Generating workspace documentation..."
cd "$ROOT_DIR" || exit 1
cargo doc-all --output "$DOCS_DIR/workspace"

# Initialize the guide book if it doesn't exist
if [ ! -d "$DOCS_DIR/guide" ]; then
  echo "Initializing guide book..."
  mkdir -p "$DOCS_DIR/guide"
  cd "$DOCS_DIR/guide" || exit 1
  mdbook init
  
  # Configure the book
  cat > "$DOCS_DIR/guide/book.toml" << EOF
[book]
authors = ["David Maple"]
language = "en"
multilingual = false
src = "src"
title = "AsyncTask Guide"

[output.html]
mathjax-support = false
git-repository-url = "https://github.com/cyrup-ai/async_task"
edit-url-template = "https://github.com/cyrup-ai/async_task/edit/main/docs/guide/{path}"
site-url = "/async_task/"
additional-css = ["theme/custom.css"]

[output.html.playground]
editable = true
line-numbers = true

[output.html.search]
limit-results = 20
use-boolean-and = true
boost-title = 2
boost-hierarchy = 2
boost-paragraph = 1
expand = true
heading-split-level = 2
EOF

  # Create a custom theme
  mkdir -p "$DOCS_DIR/guide/theme"
  cat > "$DOCS_DIR/guide/theme/custom.css" << EOF
:root {
    --primary-color: #5592d7;
    --content-max-width: 1000px;
}

.content main {
    margin-top: 0;
    padding: 0 15px 50px 15px;
}

h1, h2, h3, h4, h5, h6 {
    margin-top: 2em;
    margin-bottom: 0.5em;
}

h1 { color: var(--primary-color); }

pre {
    padding: 1em;
    border-radius: 5px;
}

a {
    color: var(--primary-color);
}

.menu-title {
    font-weight: bold;
    color: var(--primary-color);
}
EOF

  # Create the initial content structure
  cat > "$DOCS_DIR/guide/src/SUMMARY.md" << EOF
# Summary

[Introduction](./introduction.md)

# User Guide

- [Getting Started](./guide/getting-started.md)
- [Basic Concepts](./guide/basic-concepts.md)
- [Future Execution](./guide/future-execution.md)
- [Stream Execution](./guide/stream-execution.md)
- [Error Handling](./guide/error-handling.md)
- [Task Cancellation](./guide/task-cancellation.md)
- [Structured Concurrency](./guide/structured-concurrency.md)
- [Task Orchestration](./guide/task-orchestration.md)
- [Metrics & Monitoring](./guide/metrics-monitoring.md)

# API Reference

- [Core API](./api/core.md)
- [Task Traits](./api/task-traits.md)
- [Builder](./api/builder.md)
- [Orchestration](./api/orchestration.md)
- [Error Handling](./api/error-handling.md)

# Advanced Topics

- [Custom Implementations](./advanced/custom-implementations.md)
- [Runtime Integration](./advanced/runtime-integration.md)
- [Performance Considerations](./advanced/performance.md)

# Meta

- [Design Philosophy](./meta/design-philosophy.md)
- [Comparison to Alternatives](./meta/comparison.md)
- [Roadmap](./meta/roadmap.md)
- [Contributing](./meta/contributing.md)
EOF

  # Create the introduction page
  cat > "$DOCS_DIR/guide/src/introduction.md" << EOF
# Introduction

Welcome to the AsyncTask library, a comprehensive trait-based API for asynchronous task management in Rust.

This guide aims to provide thorough documentation for using AsyncTask in your projects.

## What is AsyncTask?

AsyncTask provides a type-safe, ergonomic API for working with asynchronous tasks in Rust. It offers a clean abstraction over different async runtimes, enabling consistent task management across your application.

The library focuses on making async Rust more intuitive by providing:

- A unified interface for both Future and Stream-based execution
- Powerful task priority, cancellation, and monitoring capabilities
- Structured concurrency for safer resource management
- Comprehensive observability through metrics and tracing
- An ergonomic builder pattern for task configuration

## Key Features

- **Pure Trait Abstraction:** A clean, zero-implementation trait API that lets you customize as needed
- **Dual Execution Modes:** Support for both Future-based (`spawn`) and Stream-based (`emit`) task execution
- **Priority Management:** Built-in priority queueing for intelligent resource allocation
- **Structured Concurrency:** Parents and children with automatic cancellation propagation
- **Rich Observability:** Metrics collection and tracing support built into the core design
- **Cancellation Propagation:** Fine-grained control over task cancellation with escalation policies
- **Error Recovery:** Structured error handling with retry capabilities
- **Resource Monitoring:** CPU, memory, and I/O usage metrics collection
EOF
fi

# Build the guide
echo "Building guide..."
cd "$DOCS_DIR/guide" || exit 1
mdbook build

# Create an index page that links to both the API docs and the guide
cat > "$DOCS_DIR/index.html" << EOF
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AsyncTask Documentation</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
        }
        h1, h2 {
            color: #5592d7;
        }
        a {
            color: #5592d7;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        .card {
            border: 1px solid #ddd;
            border-radius: 4px;
            padding: 20px;
            margin-bottom: 20px;
            transition: all 0.3s ease;
        }
        .card:hover {
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
            transform: translateY(-2px);
        }
        .logo {
            font-size: 3em;
            margin-bottom: 10px;
        }
    </style>
</head>
<body>
    <h1>AsyncTask Documentation</h1>
    <p>Welcome to the AsyncTask documentation portal. Here you'll find comprehensive resources for working with the AsyncTask library.</p>

    <div class="card">
        <h2>📘 User Guide</h2>
        <p>Start here for a guided tour of AsyncTask's features and how to use them effectively in your projects.</p>
        <p><a href="guide/book/">Go to User Guide →</a></p>
    </div>

    <div class="card">
        <h2>🔍 API Reference</h2>
        <p>Detailed documentation of the AsyncTask API, including all traits, methods, and types.</p>
        <p><a href="api/async_task_api/">Go to API Reference →</a></p>
    </div>

    <div class="card">
        <h2>🧩 Workspace Documentation</h2>
        <p>Cross-referenced documentation for the entire AsyncTask workspace, including all crates.</p>
        <p><a href="workspace/">Go to Workspace Docs →</a></p>
    </div>

    <footer>
        <p>AsyncTask is open source software, licensed under MIT.</p>
        <p><a href="https://github.com/cyrup-ai/async_task">GitHub Repository</a></p>
    </footer>
</body>
</html>
EOF

echo "Documentation generated successfully at $DOCS_DIR"
echo "Open $DOCS_DIR/index.html in your browser to view it"