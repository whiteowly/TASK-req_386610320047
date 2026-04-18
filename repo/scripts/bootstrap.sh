#!/usr/bin/env bash
# =============================================================================
# scripts/bootstrap.sh
#
# DEV-ONLY ephemeral secret generation.
#
# WARNING: This script is designed for local development and CI environments
# only. It generates secrets at container startup using /dev/urandom and
# stores them in /run/secrets/ (tmpfs-backed in Docker). These secrets are
# ephemeral — they are lost when the container stops.
#
# DO NOT use this approach in production. In production:
#   - Use a secrets manager (AWS Secrets Manager, Vault, Kubernetes Secrets)
#   - Mount secrets as files from an external store
#   - Never generate secrets from within the container
#
# Usage: source /usr/local/bin/bootstrap.sh
#        (or called from entrypoint.sh)
# =============================================================================

set -euo pipefail

SECRETS_DIR="${SECRETS_DIR:-/run/secrets}"

log() {
    echo "[bootstrap] $*" >&2
}

# Ensure the secrets directory exists (should be tmpfs-mounted)
mkdir -p "${SECRETS_DIR}"

# ---------------------------------------------------------------------------
# Generate DB_PASSWORD
# Only generate a new one if it doesn't already exist on the shared volume.
# This ensures the app and db containers agree on the same password.
# ---------------------------------------------------------------------------
DB_PASSWORD_FILE="${SECRETS_DIR}/db_password"

if [[ ! -f "${DB_PASSWORD_FILE}" ]]; then
    log "Generating ephemeral DB password..."
    # 32 bytes -> 64 hex characters
    DB_PASSWORD="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    printf '%s' "${DB_PASSWORD}" > "${DB_PASSWORD_FILE}"
    chmod 600 "${DB_PASSWORD_FILE}"
    log "DB password written to ${DB_PASSWORD_FILE}"
else
    log "Reusing existing DB password from ${DB_PASSWORD_FILE}"
    DB_PASSWORD="$(cat "${DB_PASSWORD_FILE}")"
fi

# ---------------------------------------------------------------------------
# Generate SESSION_SECRET (64 hex chars = 32 bytes of entropy)
# ---------------------------------------------------------------------------
SESSION_SECRET_FILE="${SECRETS_DIR}/session_secret"

if [[ ! -f "${SESSION_SECRET_FILE}" ]]; then
    log "Generating ephemeral session secret..."
    SESSION_SECRET="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    printf '%s' "${SESSION_SECRET}" > "${SESSION_SECRET_FILE}"
    chmod 600 "${SESSION_SECRET_FILE}"
else
    SESSION_SECRET="$(cat "${SESSION_SECRET_FILE}")"
fi

# ---------------------------------------------------------------------------
# Generate ENCRYPTION_KEY (32 bytes -> base64)
# Suitable for AES-256-GCM
# ---------------------------------------------------------------------------
ENCRYPTION_KEY_FILE="${SECRETS_DIR}/encryption_key"

if [[ ! -f "${ENCRYPTION_KEY_FILE}" ]]; then
    log "Generating ephemeral encryption key..."
    ENCRYPTION_KEY="$(head -c 32 /dev/urandom | base64 | tr -d '\n')"
    printf '%s' "${ENCRYPTION_KEY}" > "${ENCRYPTION_KEY_FILE}"
    chmod 600 "${ENCRYPTION_KEY_FILE}"
else
    ENCRYPTION_KEY="$(cat "${ENCRYPTION_KEY_FILE}")"
fi

# ---------------------------------------------------------------------------
# Construct DATABASE_URL from environment + generated password
# ---------------------------------------------------------------------------
POSTGRES_HOST="${POSTGRES_HOST:-db}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
POSTGRES_USER="${POSTGRES_USER:-knowledgeops}"
POSTGRES_DB="${POSTGRES_DB:-knowledgeops_db}"

DATABASE_URL="postgres://${POSTGRES_USER}:${DB_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_DB}"

# ---------------------------------------------------------------------------
# Export all secrets into the current environment
# ---------------------------------------------------------------------------
export DATABASE_URL
export SESSION_SECRET
export ENCRYPTION_KEY
export DB_PASSWORD

log "Bootstrap complete. Secrets are in memory and ${SECRETS_DIR}/ (tmpfs)."
log "DATABASE_URL points to: postgres://${POSTGRES_USER}:***@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_DB}"
