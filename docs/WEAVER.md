# OpenTelemetry Weaver Integration

This document describes how we use OpenTelemetry Weaver for documentation generation, schema validation, and code generation in the Async Task project.

## Table of Contents
- [Overview](#overview)
- [Setup](#setup)
- [Directory Structure](#directory-structure)
- [Defining Metrics](#defining-metrics)
- [Generating Documentation](#generating-documentation)
- [Code Generation](#code-generation)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

[OpenTelemetry Weaver](https://github.com/open-telemetry/weaver) is a comprehensive tool that helps us:

1. **Define, validate, and document semantic conventions** for our telemetry data
2. **Generate Rust code** for metrics, traces, and logs
3. **Enforce consistency** across our observability implementation
4. **Produce documentation** that stays in sync with our code

We use Weaver to maintain a schema-first approach to observability, ensuring our metrics are well-defined, consistent, and properly documented.

## Setup

### Installation

```bash
# Install from cargo
cargo install --git https://github.com/open-telemetry/weaver.git

# Verify installation
weaver --version
```

### Project Configuration

In our project, Weaver configuration lives in the `telemetry/` directory:

```
telemetry/
├── weaver.yaml                 # Main configuration
├── schemas/                    # Telemetry schemas
│   ├── task.yaml               # Task metrics schema
│   ├── memory.yaml             # Memory metrics schema 
│   └── io.yaml                 # IO metrics schema
└── templates/                  # Code generation templates
    ├── metrics/                # Metrics templates
    │   └── task_metrics.j2     # Task metrics template
    ├── spans/                  # Span templates
    │   └── task_spans.j2       # Task span template
    └── docs/                   # Documentation templates
        └── metrics_docs.j2     # Metrics documentation template
```

## Directory Structure

### Schemas

Our telemetry schemas define the metrics, traces, and logs that our system produces. Here's an example of our task metrics schema:

```yaml
# telemetry/schemas/task.yaml
groups:
  async_task:
    prefix: task
    stability: stable
    attributes:
      task.id:
        type: string
        description: Unique identifier for the task
        examples: ["task_12345", "backup_task_1"]
      task.name:
        type: string
        description: Human-readable name of the task
        examples: ["data_processing", "user_backup"]
      task.priority:
        type: string
        description: Priority level of the task
        examples: ["high", "low", "critical"]
    metrics:
      task.duration:
        type: histogram
        unit: s
        description: The duration of task execution
        attributes: [task.id, task.name, task.priority]
      task.memory.usage:
        type: gauge
        unit: By
        description: Current memory usage of the task
        attributes: [task.id, task.name]
      task.cpu.usage:
        type: gauge
        unit: 1
        description: CPU utilization of the task (0.0-1.0 per core)
        attributes: [task.id, task.name]
```

### Templates

We use Jinja2 templates to generate code and documentation from our schemas. Here's an example of a metrics code generation template:

```jinja
{# telemetry/templates/metrics/task_metrics.j2 #}
//! Task metrics implementation
//! 
//! This file is auto-generated using OpenTelemetry Weaver. DO NOT MODIFY.

use opentelemetry::metrics::{Counter, Histogram, Meter, Unit};
use std::sync::Arc;

/// Metrics for tracking task execution
pub struct TaskMetrics {
    meter: Meter,
    {% for metric in metrics %}
    {{ metric.name|snake_case }}: {{ metric.type|metric_type }},
    {% endfor %}
}

impl TaskMetrics {
    /// Create a new instance of TaskMetrics
    pub fn new(meter: Meter) -> Self {
        let instance = Self {
            meter: meter.clone(),
            {% for metric in metrics %}
            {{ metric.name|snake_case }}: meter
                .{{ metric.type|meter_method }}("{{ metric.name }}")
                .with_description("{{ metric.description }}")
                .with_unit(Unit::new("{{ metric.unit }}"))
                .build(),
            {% endfor %}
        };
        
        instance
    }
    
    {% for metric in metrics %}
    /// Record {{ metric.description|lower }}
    pub fn record_{{ metric.name|method_name }}(&self, value: {{ metric.type|value_type }}, attributes: &[KeyValue]) {
        self.{{ metric.name|snake_case }}.{{ metric.type|record_method }}(value, attributes);
    }
    {% endfor %}
}
```

## Defining Metrics

When defining new metrics for our system, follow these guidelines:

1. **Use the proper metric type**:
   - **Counter**: For values that only increase (e.g., task_completed_count)
   - **Histogram**: For distributions (e.g., task_duration)
   - **Gauge**: For values that can go up and down (e.g., memory_usage)

2. **Use standard units**:
   - Time: `s` (seconds), `ms` (milliseconds), `us` (microseconds)
   - Memory: `By` (bytes), `KiBy` (kibibytes), `MiBy` (mebibytes)
   - CPU: `1` (ratio/fraction) for utilization from 0.0 to 1.0 per core

3. **Name metrics consistently**:
   - Use the object.action.unit pattern where possible
   - Examples: `task.duration.seconds`, `task.memory.bytes`

## Generating Documentation

To generate documentation based on our schemas:

```bash
# Generate Markdown documentation
weaver registry generate \
  --templates telemetry/templates \
  --param output=docs/metrics \
  telemetry/schemas

# Generate code
weaver registry generate \
  --templates telemetry/templates \
  --param output=src/metrics \
  telemetry/schemas
```

The generated documentation will be placed in `docs/metrics/` and can be included in our API documentation.

## Code Generation

We use code generation for:

1. **Metric definitions** - Struct and methods for recording metrics
2. **Semantic conventions** - Constants for attribute names and values
3. **Helper functions** - Utilities for working with telemetry

Generated code should never be modified directly. Instead:

1. Modify the schema (for changes to metrics or attributes)
2. Modify the templates (for changes to code generation patterns)
3. Re-run the generation process

## Best Practices

1. **Schema First**: Always define metrics in schema files before implementing
2. **Version Control**: Include generated code in version control
3. **Regenerate on Schema Changes**: Always regenerate code when schemas change
4. **Write Unit Tests**: Test that metrics are recorded correctly
5. **Use Standard Attributes**: Reuse common attributes like `task.id` instead of creating new ones
6. **Document Context**: Include the context in which metrics should be recorded

## Troubleshooting

### Common Issues

1. **Invalid Schema**: Check schema validation errors
   ```bash
   weaver registry check telemetry/schemas
   ```

2. **Template Errors**: Look for syntax errors in templates
   ```bash
   # Validate templates
   weaver registry generate --dry-run --templates telemetry/templates telemetry/schemas
   ```

3. **Missing Dependencies**: Ensure opentelemetry crates are in your Cargo.toml
   ```toml
   [dependencies]
   opentelemetry = "0.19"
   opentelemetry_sdk = "0.19"
   ```

### Getting Help

If you encounter issues:
1. Check [Weaver documentation](https://github.com/open-telemetry/weaver/blob/main/docs/usage.md)
2. Consult with the team's observability expert
3. Look for similar issues in the [Weaver GitHub repository](https://github.com/open-telemetry/weaver/issues)
