# Stage 1: Planner — generates recipe.json from manifests only
# zenii-desktop and zenii-mobile are stripped from the workspace before prepare
# to avoid pulling in GTK/webkit2gtk deps that are not needed for the daemon.
FROM rust:1.88-bookworm AS planner
WORKDIR /app
RUN cargo install cargo-chef --locked
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY wiki/ wiki/
RUN sed -i '/"crates\/zenii-desktop"/d; /"crates\/zenii-mobile"/d' Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Cacher — compile deps (cached unless Cargo.toml/Cargo.lock change)
FROM rust:1.88-bookworm AS cacher
WORKDIR /app
RUN apt-get update && apt-get install -y libsqlite3-dev libdbus-1-dev pkg-config && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --profile ci-release --recipe-path recipe.json \
      --features keyring,channels,channels-telegram,channels-slack,channels-discord,scheduler,workflows,web-dashboard,api-docs

# Stage 3: Builder — compile only source (deps already cached above)
FROM rust:1.88-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y libsqlite3-dev libdbus-1-dev pkg-config && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY wiki/ wiki/
RUN sed -i '/"crates\/zenii-desktop"/d; /"crates\/zenii-mobile"/d' Cargo.toml
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --profile ci-release -p zenii-daemon \
      --features keyring,channels,channels-telegram,channels-slack,channels-discord,scheduler,workflows,web-dashboard,api-docs && \
    cargo build --profile ci-release -p zenii-mcp-server

# Stage 4: Runtime
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
