#!/bin/bash
set -e

echo "Restarting FoundationDB to apply new configuration..."
echo ""

# Stop FoundationDB
echo "Stopping FoundationDB..."
sudo launchctl stop com.foundationdb.fdbmonitor

# Wait a moment
sleep 2

# Clean old data (fresh start)
echo "Cleaning old data directory..."
sudo rm -rf /usr/local/foundationdb/data/4689/*

# Start FoundationDB
echo "Starting FoundationDB..."
sudo launchctl start com.foundationdb.fdbmonitor

# Wait for service to start
echo "Waiting for service to start..."
sleep 3

# Configure new database
echo "Configuring new database..."
fdbcli --exec "configure new single ssd"

# Wait for initialization
sleep 2

# Check status
echo ""
echo "Database status:"
fdbcli --exec "status minimal"

echo ""
echo "✓ FoundationDB restarted and configured!"

