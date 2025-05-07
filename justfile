# ────────────────────────────── Cargo ──────────────────────────────
default:
    @just --list

doc:
    cargo rustdoc -Z unstable-options --output-format json

build:
    cargo fmt --all --message-format short --quiet
    cargo build --release --message-format short --quiet

run:
    cargo fmt --all --message-format short --quiet
    cargo run --message-format short --quiet

check:
    cargo fmt --all --message-format short --quiet -- --check
    cargo check --all-targets --all-features --message-format short --quiet 
    cargo clippy --allow-dirty --allow-staged --all-targets --all-features --message-format short --quiet --fix -- -D warnings

test:
    cargo fmt --all --message-format short --quiet
    cargo nextest run --message-format short --quiet

install:
    cargo install --path . 

# ───────────────────────── Documentation ─────────────────────────

# Generate beautiful API documentation
docs-api:
    @echo "Generating API documentation..."
    @mkdir -p docs/generated/api
    RUSTDOCFLAGS="--cfg docsrs" cargo doc --document-private-items --no-deps --all-features
    @echo "✅ API documentation generated in target/doc/"

# Generate comprehensive documentation with mdBook
docs-guide:
    @echo "Generating user guide with mdBook..."
    @if ! command -v mdbook &> /dev/null; then cargo install mdbook; fi
    @mkdir -p docs/guide
    @if [ ! -d "docs/guide/.git" ]; then \
        cd docs/guide && mdbook init; \
    fi
    @cd docs/guide && mdbook build
    @echo "✅ User guide generated in docs/guide/book/"

# Generate cross-crate documentation for the entire workspace
docs-workspace:
    @echo "Generating workspace documentation..."
    @if ! command -v cargo-doc-all &> /dev/null; then cargo install cargo-doc-all; fi
    @mkdir -p docs/generated/workspace
    cargo doc-all --output docs/generated/workspace
    @echo "✅ Workspace documentation generated in docs/generated/workspace/"

# Generate all documentation
docs-all: docs-api docs-guide docs-workspace
    @echo "✅ All documentation generated successfully"

# Generate documentation from OpenTelemetry schema files
weaver-docs:
    @echo "Generating documentation from telemetry schemas..."
    @mkdir -p docs/metrics
    weaver registry generate \
      --templates telemetry/templates \
      --param output=docs/metrics \
      telemetry/schemas
    @echo "✅ Documentation generated in docs/metrics/"

# Generate code from OpenTelemetry schema files
weaver-code:
    @echo "Generating code from telemetry schemas..."
    @mkdir -p src/metrics
    weaver registry generate \
      --templates telemetry/templates \
      --param output=src/metrics \
      telemetry/schemas
    @echo "✅ Code generated in src/metrics/"
    cargo fmt --message-format short --quiet

# Check telemetry schemas for validity
weaver-check:
    @echo "Validating telemetry schemas..."
    weaver registry check telemetry/schemas
    @echo "✅ Schemas validated"

# Run all Weaver-related tasks
weaver-all: weaver-check weaver-code weaver-docs
    @echo "✅ All Weaver tasks completed"

# Setup initial telemetry directory structure
weaver-init:
    @echo "Initializing telemetry directory structure..."
    @mkdir -p telemetry/schemas
    @mkdir -p telemetry/templates/metrics
    @mkdir -p telemetry/templates/docs
    @echo 'groups:\n  async_task:\n    prefix: task\n    stability: stable' > telemetry/schemas/task.yaml
    @echo '{# Template for task metrics #}\n\n{# Example template #}' > telemetry/templates/metrics/task_metrics.j2
    @echo '{# Template for metrics documentation #}\n\n{# Example template #}' > telemetry/templates/docs/metrics_docs.j2
    @echo "✅ Telemetry directory structure initialized"
    @echo "   Next steps:"
    @echo "   1. Edit schemas in telemetry/schemas/"
    @echo "   2. Edit templates in telemetry/templates/"
    @echo "   3. Run 'just weaver-all' to generate docs and code"
    