#!/usr/bin/env bash
# =============================================================================
# run_tests.sh
#
# Broad test runner for the KnowledgeOps backend.
#
# Runs two test layers:
#   Layer 1: Rust-native unit tests via `cargo test` inside the test container.
#            Tests pass/fail as the gate. Line coverage measurement via
#            cargo-tarpaulin is attempted but requires --security-opt
#            seccomp=unconfined, which is not available in default Docker;
#            coverage is best-effort/optional and currently skipped.
#   Layer 2: True no-mock HTTP integration tests (curl-based) against the live
#            Docker stack. These verify real endpoint behavior end-to-end.
#
# Usage:
#   ./run_tests.sh [--no-cleanup]
# =============================================================================

set -euo pipefail

CLEANUP=true
COMPOSE_PROJECT="${COMPOSE_PROJECT_NAME:-knowledgeops_test}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
COMPOSE_FILE_PATH="${SCRIPT_DIR}/docker-compose.yml"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-cleanup) CLEANUP=false; shift ;;
        --help|-h)
            echo "Usage: ./run_tests.sh [--no-cleanup]"
            exit 0
            ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

log() {
    echo "[run_tests] $*"
}

compose_cmd() {
    docker compose \
        --project-name "${COMPOSE_PROJECT}" \
        --file "${COMPOSE_FILE_PATH}" \
        "$@"
}

cleanup() {
    if [[ "${CLEANUP}" == "true" ]]; then
        log "Cleaning up test containers and volumes..."
        compose_cmd down --volumes --remove-orphans 2>/dev/null || true
        log "Cleanup complete."
    else
        log "Skipping cleanup (--no-cleanup). To clean up manually:"
        log "  docker compose --project-name ${COMPOSE_PROJECT} --file ${COMPOSE_FILE_PATH} down --volumes"
    fi
}

trap cleanup EXIT

log "Ensuring clean test stack state..."
compose_cmd down --volumes --remove-orphans >/dev/null 2>&1 || true

if ! command -v docker &>/dev/null || ! docker compose version &>/dev/null 2>&1; then
    echo "ERROR: docker compose (v2) is required." >&2
    exit 1
fi

# =========================================================================
# Layer 1: Rust-native unit tests (in test container)
# =========================================================================
log "=== Layer 1: Rust-native unit tests ==="
log "Building test image..."
compose_cmd --profile test build test 2>&1

log "Starting db for Rust tests..."
compose_cmd --profile test up -d bootstrap-secrets db 2>&1

# Wait for db health
log "Waiting for db to be healthy..."
for i in $(seq 1 30); do
    if compose_cmd exec -T db \
        pg_isready -U knowledgeops -d knowledgeops_db -q 2>/dev/null; then
        break
    fi
    sleep 2
done

log "Running cargo test (unit tests)..."
RUST_EXIT=0
docker compose \
    --project-name "${COMPOSE_PROJECT}" \
    --file "${COMPOSE_FILE_PATH}" \
    --profile test \
    run --rm -T test \
    bash -c '
        source /usr/local/bin/bootstrap.sh 2>/dev/null
        cd /workspace
        echo "[cargo-test] Running Rust-native unit tests..."
        cargo test --test unit_tests 2>&1
        TEST_EXIT=$?
        echo ""
        if [ $TEST_EXIT -eq 0 ]; then
            echo "[cargo-test] Attempting coverage measurement with tarpaulin..."
            if command -v cargo-tarpaulin &>/dev/null; then
                cargo tarpaulin --test unit_tests --out Stdout --skip-clean 2>&1 || \
                    echo "[cargo-test] tarpaulin failed (likely needs --security-opt seccomp=unconfined). Coverage skipped."
            else
                echo "[cargo-test] tarpaulin not available. Coverage measurement skipped."
            fi
        fi
        exit $TEST_EXIT
    ' || RUST_EXIT=$?

if [[ $RUST_EXIT -ne 0 ]]; then
    log "ERROR: Rust-native tests failed (exit code $RUST_EXIT)."
    exit $RUST_EXIT
fi
log "Rust-native unit tests passed."

# =========================================================================
# Layer 2: True no-mock HTTP integration tests (curl-based)
# =========================================================================
log ""
log "=== Layer 2: HTTP integration tests ==="
log "Building and starting the full stack..."
compose_cmd up --build -d app 2>&1

log "Waiting for app to be healthy..."
MAX_WAIT=120
ELAPSED=0
while true; do
    if compose_cmd exec -T app \
        curl -sf http://localhost:8080/api/v1/health >/dev/null 2>&1; then
        log "App is healthy."
        break
    fi
    ELAPSED=$((ELAPSED + 3))
    if [[ $ELAPSED -ge $MAX_WAIT ]]; then
        log "ERROR: App did not become healthy within ${MAX_WAIT}s."
        compose_cmd logs app --tail 30 2>&1
        exit 1
    fi
    sleep 3
done

log "Running HTTP integration tests..."
compose_cmd cp "${SCRIPT_DIR}/tests/api_integration_tests.sh" app:/tmp/api_integration_tests.sh

# Run inside container, capture output to file to avoid exec buffer truncation,
# then retrieve and display
compose_cmd exec -T app bash -c 'set +e; bash /tmp/api_integration_tests.sh > /tmp/http_results.txt 2>&1; echo $? > /tmp/http_exit.txt'

HTTP_EXIT=$(compose_cmd exec -T app cat /tmp/http_exit.txt 2>/dev/null | tr -d '[:space:]')
HTTP_EXIT=${HTTP_EXIT:-1}

# Display results
compose_cmd exec -T app cat /tmp/http_results.txt

# =========================================================================
# Report
# =========================================================================
echo ""
echo "============================================================"
echo "  Layer 1 (Rust unit tests):     $([ $RUST_EXIT -eq 0 ] && echo PASSED || echo FAILED)"
echo "  Layer 2 (HTTP integration):    $([ $HTTP_EXIT -eq 0 ] && echo PASSED || echo FAILED)"
echo "============================================================"

FINAL_EXIT=$((RUST_EXIT + HTTP_EXIT))
if [[ $FINAL_EXIT -eq 0 ]]; then
    echo "  OVERALL: PASSED"
else
    echo "  OVERALL: FAILED"
fi
echo "============================================================"
echo ""

exit $FINAL_EXIT
