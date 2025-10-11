# Stage 2: Build the Rust application
FROM rust:1.90-bookworm AS rust-builder

# FoundationDB version
ARG FDB_VERSION=7.3.69

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    wget \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Install FoundationDB client library
RUN ARCH=$(dpkg --print-architecture) && \
    wget -q --timeout=30 --tries=3 https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/foundationdb-clients_${FDB_VERSION}-1_${ARCH}.deb && \
    dpkg -i foundationdb-clients_${FDB_VERSION}-1_${ARCH}.deb || apt-get install -f -y && \
    rm -f foundationdb-clients_${FDB_VERSION}-1_${ARCH}.deb

# Create working directory
WORKDIR /build

# Copy workspace configuration files
COPY Cargo.toml Cargo.lock ./

# Copy all crates (since degov-cli depends on workspace crates)
COPY crates/ ./crates/

# Build the application in release mode
RUN cargo build --release --package degov-cli

# Stage 3: Create minimal runtime image
FROM debian:bookworm-slim

# FoundationDB version (must match builder)
ARG FDB_VERSION=7.3.69

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install FoundationDB client library (runtime)
RUN ARCH=$(dpkg --print-architecture) && \
    wget -q --timeout=30 --tries=3 https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/foundationdb-clients_${FDB_VERSION}-1_${ARCH}.deb && \
    dpkg -i foundationdb-clients_${FDB_VERSION}-1_${ARCH}.deb || apt-get install -f -y && \
    rm -f foundationdb-clients_${FDB_VERSION}-1_${ARCH}.deb

# Create a non-root user
RUN useradd -m -u 1000 degov

# Copy the built binary from rust-builder
COPY --from=rust-builder /build/target/release/degov-cli /usr/local/bin/degov

# Copy the start script
COPY packaging/docker/cli/start-server.bash /app/start-server.bash

# Copy the cluster file
COPY packaging/docker/cli/fdb.cluster /app/fdb.cluster

# Set ownership
RUN chown -R degov:degov /app /usr/local/bin/degov

# Switch to non-root user
USER degov

# Set working directory
WORKDIR /app

# Set the entrypoint
ENTRYPOINT ["degov"]
CMD ["--help"]

