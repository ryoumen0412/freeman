# Multi-stage Dockerfile for Freeman TUI

# --- Stage 1: Build static binary with dependency layer caching ---
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependencies in a separate Docker layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy actual source code and build final release binary
COPY src ./src
RUN touch src/main.rs && cargo build --release

# --- Stage 2: Final minimal runtime image ---
FROM alpine:latest

RUN apk add --no-cache ca-certificates

# Set default working directory to /workspace for mounting host projects (-v $(pwd):/workspace)
WORKDIR /workspace

# Copy compiled binary from builder stage
COPY --from=builder /app/target/release/freeman /usr/local/bin/freeman

ENTRYPOINT ["freeman"]
