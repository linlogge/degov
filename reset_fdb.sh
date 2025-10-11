#!/bin/bash
set -e

echo "Completely resetting FoundationDB..."
echo "WARNING: This will delete all data!"
echo ""

# Stop FoundationDB
echo "Stopping FoundationDB..."
sudo launchctl stop com.foundationdb.fdbmonitor
sleep 2

# Remove all data and coordination files
echo "Removing all database files..."
sudo rm -rf /usr/local/foundationdb/data/*

# Start FoundationDB
echo "Starting FoundationDB..."
sudo launchctl start com.foundationdb.fdbmonitor
sleep 5

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
echo "✓ FoundationDB has been completely reset and configured!"

