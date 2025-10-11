ARG FDB_VERSION=7.3.63

FROM foundationdb/foundationdb:${FDB_VERSION}

# Copy fixed scripts that properly handle IPv6 addresses
# These scripts must be placed in /var/fdb/scripts/ as that's where
# the base image's entrypoint expects them
COPY fdb.bash /var/fdb/scripts/fdb.bash
COPY fdb_single.bash /var/fdb/scripts/fdb_single.bash

# Make scripts executable
RUN chmod +x /var/fdb/scripts/fdb.bash /var/fdb/scripts/fdb_single.bash

# The original entrypoint remains unchanged and will use our fixed scripts
# The fixed scripts will now properly format IPv6 addresses with brackets
# when constructing the IP:PORT strings for FoundationDB
