# Stage 1: Build
FROM rust:1.88-bookworm AS builder
WORKDIR /app

# Install SQLite dev libs
RUN apt-get update && apt-get install -y libsqlite3-dev libdbus-1-dev pkg-config && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build daemon and MCP server
RUN cargo build --profile ci-release -p zenii-daemon --all-features && \
    cargo build --profile ci-release -p zenii-mcp-server

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates libsqlite3-0 libdbus-1-3 curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --system --no-create-home --shell /usr/sbin/nologin zenii && \
    mkdir -p /data /config && \
    chown zenii:zenii /data /config

COPY --from=builder /app/target/ci-release/zenii-daemon /usr/local/bin/zenii-daemon
COPY --from=builder /app/target/ci-release/zenii-mcp-server /usr/local/bin/zenii-mcp-server

USER zenii

EXPOSE 18981

ENV RUST_LOG=info

# Default: MCP server (stdio transport for MCP clients).
# Override with --entrypoint zenii-daemon for the HTTP API server.
ENTRYPOINT ["zenii-mcp-server"]
CMD ["--transport", "stdio"]
