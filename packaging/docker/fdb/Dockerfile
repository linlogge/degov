FROM foundationdb/foundationdb:latest

# Copy fixed scripts that properly handle IPv6 addresses
COPY fdb.bash /fdb.bash
COPY fdb_single.bash /fdb_single.bash

# Make scripts executable
RUN chmod +x /fdb.bash /fdb_single.bash

# The original entrypoint and sidecar remain unchanged
# The fixed scripts will now properly format IPv6 addresses with brackets
# when constructing the IP:PORT strings for FoundationDB
