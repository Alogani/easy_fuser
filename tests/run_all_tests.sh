#!/bin/bash

echo "Running tests for parallel feature"
cargo test --features parallel

echo "Running tests for serial feature"
cargo test --features serial --test integration_test

# echo "Running tests for async feature"
# cargo test --features async --test integration_test