#!/usr/bin/env bash
# =============================================================================
# scripts/entrypoint.sh
#
# Main container entrypoint for the knowledgeops application.
#
# Steps:
#   1. Source bootstrap.sh to load ephemeral secrets into environment.
#   2. Wait for PostgreSQL to be reachable.
#   3. Start the application binary.
#      (The binary runs embedded Diesel migrations and seeds roles/users
#       before starting the HTTP server.)
#
# This is for local development only. Not the production secret path.
# =============================================================================

set -euo pipefail

log() {
    echo "[entrypoint] $*" >&2
}

# 1. Bootstrap secrets
log "Running bootstrap..."
source /usr/local/bin/bootstrap.sh

# 2. Wait for PostgreSQL
POSTGRES_HOST="${POSTGRES_HOST:-db}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
POSTGRES_USER="${POSTGRES_USER:-knowledgeops}"
POSTGRES_DB="${POSTGRES_DB:-knowledgeops_db}"

log "Waiting for PostgreSQL at ${POSTGRES_HOST}:${POSTGRES_PORT}..."

MAX_RETRIES=30
RETRY_INTERVAL=2
retries=0

until pg_isready -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -q 2>/dev/null; do
    retries=$((retries + 1))
    if [[ ${retries} -ge ${MAX_RETRIES} ]]; then
        log "ERROR: PostgreSQL did not become ready after $((MAX_RETRIES * RETRY_INTERVAL)) seconds."
        exit 1
    fi
    log "  Postgres not ready (attempt ${retries}/${MAX_RETRIES}), retrying..."
    sleep "${RETRY_INTERVAL}"
done

log "PostgreSQL is ready."

# 3. Start the application
#    The binary handles: embedded Diesel migrations, role seeding,
#    demo user seeding, background job scheduler, HTTP server.
log "Starting knowledgeops..."
exec /usr/local/bin/knowledgeops "$@"
