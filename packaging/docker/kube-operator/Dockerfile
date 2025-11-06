# Build stage
FROM rust:1.91-slim AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY proto/ ./proto/
COPY crates/ ./crates/
COPY bin/ ./bin/

# Build the binary
RUN cargo build --release --bin kube-operator

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /build/target/release/kube-operator /app/kube-operator

# Set the binary AS the entrypoint
ENTRYPOINT ["/app/kube-operator"]

