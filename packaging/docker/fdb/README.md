# FoundationDB Docker Image with IPv6 Fix

This is a custom FoundationDB Docker image that fixes IPv6 address parsing issues.

## Problem

The original FoundationDB Docker image scripts parse the public IP address but don't properly format IPv6 addresses when constructing the `IP:PORT` strings. 

FoundationDB expects:
- `<IPv4>:<PORT>` for IPv4 addresses (e.g., `127.0.0.1:4500`)
- `[<IPv6>]:<PORT>` for IPv6 addresses (e.g., `[2001:db8::1]:4500`)

The original scripts would create malformed addresses like `2001:db8::1:4500` which causes parsing errors.

## Solution

We've added a `format_ip_for_port()` function to both `fdb.bash` and `fdb_single.bash` that:
1. Detects if an IP address is IPv6 (by checking for colons)
2. Wraps IPv6 addresses in brackets `[]`
3. Leaves IPv4 addresses unchanged

The formatted IP is stored in the `FORMATTED_PUBLIC_IP` environment variable and used throughout the scripts when constructing `IP:PORT` strings.

## Changes Made

### Modified Files

1. **fdb.bash** - Added IPv6 detection and formatting
2. **fdb_single.bash** - Added IPv6 detection and formatting  
3. **Dockerfile** - Copies the fixed scripts into the base image

### Key Changes

- Added `format_ip_for_port()` function to detect and format IPv6 addresses
- Modified `create_server_environment()` to format the public IP and store it as `FORMATTED_PUBLIC_IP`
- Updated all references to `$PUBLIC_IP:$FDB_PORT` to use `$FORMATTED_PUBLIC_IP:$FDB_PORT`
- Fixed coordinator IP formatting in cluster file generation

## Building

```bash
chmod +x build.sh
./build.sh
```

Or manually:

```bash
docker build -t degov-fdb:latest .
```

## Usage

### Single Node (IPv4 or IPv6)

```bash
docker run \
  -e FDB_NETWORKING_MODE=container \
  -e FDB_PORT=4500 \
  -p 4500:4500 \
  degov-fdb:latest
```

### With IPv6 Networking

```bash
docker run \
  --network host \
  -e FDB_NETWORKING_MODE=container \
  -e FDB_PORT=4500 \
  degov-fdb:latest
```

### Environment Variables

- `FDB_NETWORKING_MODE` - Set to `container` or `host`
- `FDB_PORT` - Port for FoundationDB (default: 4500)
- `FDB_PROCESS_CLASS` - Process class (default: unset)
- `FDB_COORDINATOR` - Coordinator hostname for multi-node setup
- `FDB_COORDINATOR_PORT` - Coordinator port (default: 4500)
- `FDB_CLUSTER_FILE_CONTENTS` - Direct cluster file contents

## Testing

You can verify the fix works by checking the logs after starting the container:

```bash
docker logs <container-id>
```

You should see:
```
Starting FDB server on [<IPv6>]:<PORT>
```

Instead of the malformed:
```
Starting FDB server on <IPv6>:<PORT>
```

## Original Source

The original scripts are from the [FoundationDB Docker repository](https://github.com/apple/foundationdb/tree/main/packaging/docker).

