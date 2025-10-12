#!/bin/sh
set -e

# Parse API_URL into components
# Default: http://localhost:3030/api
API_URL=${API_URL:-http://localhost:3030/api}

# Remove protocol
URL_WITHOUT_PROTOCOL=$(echo "$API_URL" | sed 's|^[^:]*://||')

# Extract host and port+path
HOST_AND_PORT=$(echo "$URL_WITHOUT_PROTOCOL" | cut -d'/' -f1)
PATH_PART="/$(echo "$URL_WITHOUT_PROTOCOL" | cut -d'/' -f2-)"

# Extract host and port
if echo "$HOST_AND_PORT" | grep -q ':'; then
    export API_HOST=$(echo "$HOST_AND_PORT" | cut -d':' -f1)
    export API_PORT=$(echo "$HOST_AND_PORT" | cut -d':' -f2)
else
    export API_HOST="$HOST_AND_PORT"
    export API_PORT=80
fi

# Handle path (remove trailing slash if present, we'll add it in nginx)
export API_PATH=$(echo "$PATH_PART" | sed 's|/$||')

# If path is just "/", make it empty
if [ "$API_PATH" = "/" ]; then
    export API_PATH=""
fi

echo "Configuring nginx to proxy /rpc to $API_HOST:$API_PORT$API_PATH"

# Run the default nginx entrypoint with our environment variables
exec /docker-entrypoint.sh "$@"

