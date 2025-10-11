#!/bin/bash
set -e

echo "Setting up FoundationDB..."
echo "This will configure a new single-node database (existing data will be lost)"
echo ""

# Configure new database
echo "Configuring database..."
fdbcli --exec "configure new single ssd"

# Wait a moment for initialization
echo "Waiting for database to initialize..."
sleep 2

# Check status
echo ""
echo "Database status:"
fdbcli --exec "status minimal"

echo ""
echo "✓ FoundationDB setup complete!"

