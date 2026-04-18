#!/usr/bin/env bash
# =============================================================================
# scripts/test_entrypoint.sh
#
# Test container entrypoint. Runs Rust-native unit tests.
# Does NOT run diesel CLI migrations — the app binary handles those.
# =============================================================================

set -euo pipefail

log() {
    echo "[test_entrypoint] $*" >&2
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
retries=0
until pg_isready -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -q 2>/dev/null; do
    retries=$((retries + 1))
    if [[ ${retries} -ge ${MAX_RETRIES} ]]; then
        log "ERROR: PostgreSQL did not become ready."
        exit 1
    fi
    sleep 2
done
log "PostgreSQL is ready."

# 3. Run unit tests — cargo test is the primary gate, tarpaulin is optional
cd /workspace
log "Running Rust-native unit tests..."
cargo test --test unit_tests 2>&1
TEST_EXIT=$?

if [[ $TEST_EXIT -eq 0 ]]; then
    log "All Rust-native tests passed."
else
    log "Rust-native tests FAILED (exit code $TEST_EXIT)."
fi

exit $TEST_EXIT
