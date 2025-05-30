#!/bin/bash

# Extract API trait requirements
echo "=== API TRAIT REQUIREMENTS ==="
cd /Volumes/samsung_t9/projects/sweet_async

# Get all API trait signatures with their exact bounds
ast-grep run --lang rust --pattern 'pub trait $NAME' crates/api/src --json=compact | \
    jq -r '.[] | "\(.file):\(.range.start.line): \(.text)"' | \
    grep -v "__" | \
    sort

echo -e "\n=== TOKIO CURRENT IMPLEMENTATIONS ==="
# Get all tokio trait implementations
ast-grep run --lang rust --pattern 'pub trait $NAME' crates/tokio/src --json=compact | \
    jq -r '.[] | "\(.file):\(.range.start.line): \(.text)"' | \
    sort

echo -e "\n=== MISSING IMPLEMENTATIONS ==="
# Compare and find gaps
comm -23 \
    <(ast-grep run --lang rust --pattern 'pub trait $NAME' crates/api/src --json=compact | jq -r '.[] | .text' | grep -v "__" | sort) \
    <(ast-grep run --lang rust --pattern 'pub trait $NAME' crates/tokio/src --json=compact | jq -r '.[] | .text' | sort)