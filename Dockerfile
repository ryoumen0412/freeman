# Multi-stage Dockerfile for Freeman TUI

# --- Stage 1: Build static binary ---
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy dependency manifest and source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build optimized release binary
RUN cargo build --release

# --- Stage 2: Final minimal runtime image ---
FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /root/

# Copy compiled binary from builder stage
COPY --from=builder /app/target/release/freeman /usr/local/bin/freeman

ENTRYPOINT ["freeman"]
