# Multi-stage build for minimal image size

# Build stage
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 altstore && \
    mkdir -p /app /apps && \
    chown -R altstore:altstore /app /apps

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /build/target/release/altstore-repository-server /usr/local/bin/altstore-repository-server

# Copy config.json template (optional)
# COPY config.json /app/config.json

USER altstore

# Environment variables with defaults
ENV LISTEN_URL=0.0.0.0
ENV LISTEN_PORT=8080
ENV APPS_DIR=/apps

# Expose the default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${LISTEN_PORT}/ || exit 1

# Run the application
ENTRYPOINT ["/usr/local/bin/altstore-repository-server"]
CMD []
