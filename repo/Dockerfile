# syntax=docker/dockerfile:1

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM rust:latest AS builder

# Install build-time dependencies for Diesel + PostgreSQL
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy dependency manifests first for better layer caching
COPY Cargo.toml ./
COPY Cargo.lock* ./

# Create a dummy main so cargo can fetch and compile dependencies in isolation
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release 2>/dev/null || true

# Now copy the real source and build
COPY src ./src
COPY migrations ./migrations

# Touch main.rs to ensure Cargo recompiles the real source
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    postgresql-client \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install diesel_cli for running migrations at startup
# We copy a pre-built diesel_cli from builder to avoid re-compiling in runtime
COPY --from=builder /build/target/release/knowledgeops /usr/local/bin/knowledgeops

# Copy scripts
COPY scripts/bootstrap.sh    /usr/local/bin/bootstrap.sh
COPY scripts/entrypoint.sh   /usr/local/bin/entrypoint.sh
COPY init_db.sh              /app/init_db.sh

RUN chmod +x /usr/local/bin/bootstrap.sh \
             /usr/local/bin/entrypoint.sh \
             /app/init_db.sh

# Create secrets mount point (tmpfs in production, generated at runtime in dev)
RUN mkdir -p /run/secrets

# Create non-root user for running the app
RUN useradd -r -s /bin/false -d /app appuser

# Create working directory and volume mount points
RUN mkdir -p /app/alerts_spool /app/data && chown -R appuser:appuser /app

WORKDIR /app

EXPOSE 8080

# The entrypoint calls bootstrap.sh then starts the app
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
