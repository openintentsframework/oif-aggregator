# Production Dockerfile for OIF Aggregator using Chainguard secure images
FROM --platform=${BUILDPLATFORM} cgr.dev/chainguard/rust:latest@sha256:faf49718aaa95c798ed1dfdf3e4edee2cdbc3790c8994705ca6ef35972128459 AS base

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
      "linux/amd64")  echo x86_64-unknown-linux-musl  > /rust_target ;; \
      "linux/arm64")  echo aarch64-unknown-linux-musl > /rust_target ;; \
      *)              echo x86_64-unknown-linux-musl  > /rust_target ;; \
    esac \
 && rustup target add $(cat /rust_target)

USER root

# Install system dependencies needed for compilation
RUN apk update && apk add --no-cache \
    pkgconf \
    openssl-dev \
    ca-certificates \
    perl

# Set PKG_CONFIG_PATH for proper linking
ENV PKG_CONFIG_PATH=/usr/lib/pkgconfig

# Install cargo-chef with build cache mount for faster installs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-chef

WORKDIR /app

# Planner stage - analyze dependencies
FROM base AS planner

# Copy workspace files to analyze dependencies
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./
COPY crates/ ./crates/
COPY src/ ./src/

# Generate dependency recipe
RUN cargo chef prepare --recipe-path recipe.json

# Build stage - compile dependencies and application with cache mounts
FROM base AS builder

# Copy dependency recipe from planner stage
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies with cache mounts for maximum speed
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# Copy source code and build the application
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./
COPY crates/ ./crates/
COPY src/ ./src/

# Build the application in release mode with cache mounts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp /app/target/release/oif-aggregator /usr/local/bin/oif-aggregator

# Runtime stage - using Chainguard's minimal Wolfi base
FROM cgr.dev/chainguard/wolfi-base:latest

# Install only runtime dependencies (minimal set)
USER root
RUN apk add --no-cache \
    ca-certificates \
    libssl3

# Remove apk tools for security (reduces attack surface)
RUN apk del apk-tools

# Create application directory
WORKDIR /app

# Copy the binary from builder stage with proper ownership
COPY --from=builder --chown=nonroot:nonroot /usr/local/bin/oif-aggregator /usr/local/bin/oif-aggregator

# Copy configuration files with proper ownership
COPY --chown=nonroot:nonroot config/ ./config/

# Make binary executable
RUN chmod +x /usr/local/bin/oif-aggregator

# Switch to non-root user for runtime
USER nonroot

# Expose the application port
EXPOSE 4000

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/oif-aggregator"]
