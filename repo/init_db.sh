#!/usr/bin/env bash
# =============================================================================
# init_db.sh
#
# THE ONLY DB INITIALIZATION PATH for the knowledgeops project.
#
# What it does:
#   1. Sources bootstrap.sh to populate DATABASE_URL, SESSION_SECRET, and
#      ENCRYPTION_KEY in the environment (required by the binary).
#   2. Waits for PostgreSQL to be reachable.
#   3. If the app is already running (healthy), confirms DB is initialized.
#   4. Otherwise, runs `knowledgeops --init-only` which executes embedded
#      Diesel migrations and seeds roles + demo users, then exits cleanly.
#
# The bare `knowledgeops --init-only` binary requires DATABASE_URL,
# SESSION_SECRET, and ENCRYPTION_KEY to be set. Do not invoke it directly
# without first sourcing bootstrap.sh or otherwise providing those values.
# Always use this script as the canonical initialization entry point.
#
# Usage:
#   # From inside the running app container:
#   docker compose exec app /app/init_db.sh
#
#   # Standalone (e.g., before the app starts, or manual re-init):
#   /app/init_db.sh
#
# NOTE: This is for local development bootstrap only and is NOT the
#       production secret-management path.
# =============================================================================

set -euo pipefail

log() {
    echo "[init_db] $*" >&2
}

# 1. Ensure DATABASE_URL is available
if [[ -z "${DATABASE_URL:-}" ]]; then
    if [[ -f "/usr/local/bin/bootstrap.sh" ]]; then
        log "DATABASE_URL not set; sourcing bootstrap.sh..."
        source /usr/local/bin/bootstrap.sh
    else
        echo "ERROR: DATABASE_URL is not set and bootstrap.sh not found." >&2
        exit 1
    fi
fi

# 2. Wait for PostgreSQL
POSTGRES_HOST="${POSTGRES_HOST:-db}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"

log "Waiting for PostgreSQL at ${POSTGRES_HOST}:${POSTGRES_PORT}..."
MAX_RETRIES=30
retries=0
until pg_isready -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" -q 2>/dev/null; do
    retries=$((retries + 1))
    if [[ ${retries} -ge ${MAX_RETRIES} ]]; then
        log "ERROR: PostgreSQL not reachable after ${MAX_RETRIES} attempts."
        exit 1
    fi
    sleep 2
done
log "PostgreSQL is reachable."

# 3. Initialize the database
if curl -sf http://localhost:8080/api/v1/health >/dev/null 2>&1; then
    log "App is already running and healthy — DB is initialized."
else
    log "Running knowledgeops --init-only (migrations + seeding)..."
    /usr/local/bin/knowledgeops --init-only
    log "Initialization complete."
fi

log ""
log "============================================================"
log "  DB initialized successfully."
log "  Demo credentials (change immediately!):"
log "  admin    / changeme123!  (Administrator)"
log "  author   / changeme123!  (Author)"
log "  reviewer / changeme123!  (Reviewer)"
log "  analyst  / changeme123!  (Analyst)"
log "============================================================"
