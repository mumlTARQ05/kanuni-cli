# Multi-stage build for Kanuni CLI
# Supports both AMD64 and ARM64 architectures

# Build stage
FROM rust:1.75-slim AS builder

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /build

# Copy project files
COPY Cargo.toml ./
COPY src ./src

# Build the binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 kanuni

# Copy binary from builder
COPY --from=builder /build/target/release/kanuni /usr/local/bin/kanuni

# Make binary executable
RUN chmod +x /usr/local/bin/kanuni

# Switch to non-root user
USER kanuni

# Set working directory
WORKDIR /workspace

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD kanuni --version || exit 1

# Default command
ENTRYPOINT ["kanuni"]
CMD ["--help"]

# Labels
LABEL org.opencontainers.image.source="https://github.com/v-lawyer/kanuni-cli"
LABEL org.opencontainers.image.description="AI-powered legal intelligence CLI"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"
LABEL maintainer="V-Lawyer Team <opensource@v-lawyer.ai>"