#!/bin/bash
set -e

echo "Building project..."
cargo build -p degov-cli --release --quiet
cargo build -p degov-api --example simple --quiet

echo ""
echo "=== Starting DeGov Server ==="
# Start server in background
./target/release/degov-cli server start --did "did:example:test" &
SERVER_PID=$!

# Give server time to start
sleep 2

echo ""
echo "=== Running Client ==="
./target/debug/examples/simple

echo ""
echo "=== Stopping Server ==="
# Stop the server
kill $SERVER_PID 2>/dev/null || true

echo ""
echo "Test complete!"

