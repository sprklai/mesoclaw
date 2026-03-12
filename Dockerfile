# Stage 1: Build
FROM rust:1.88-bookworm AS builder
WORKDIR /app

# Install SQLite dev libs
RUN apt-get update && apt-get install -y libsqlite3-dev && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build daemon with all features
RUN cargo build --profile ci-release -p mesoclaw-daemon --all-features

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates libsqlite3-0 curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --system --no-create-home --shell /usr/sbin/nologin mesoclaw && \
    mkdir -p /data /config && \
    chown mesoclaw:mesoclaw /data /config

COPY --from=builder /app/target/ci-release/mesoclaw-daemon /usr/local/bin/mesoclaw-daemon

USER mesoclaw

EXPOSE 18981

ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD curl -f http://localhost:18981/health || exit 1

ENTRYPOINT ["mesoclaw-daemon"]
