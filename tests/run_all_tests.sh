#!/bin/bash

overall_status=0

echo "Running tests for parallel feature"
cargo test --features parallel
parallel_status=$?
overall_status=$((overall_status | parallel_status))

echo "Running tests for serial feature"
cargo test --features serial --test integration_test
serial_status=$?
overall_status=$((overall_status | serial_status))

# echo "Running tests for async feature"
# cargo test --features async --test integration_test
# async_status=$?
# overall_status=$((overall_status | async_status))

# Print final status message
if [ $overall_status -eq 0 ]; then
    echo "All tests passed successfully!"
else
    echo "One or more test suites failed. Please check the output above for details."
fi

exit $overall_status