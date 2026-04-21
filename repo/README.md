# KnowledgeOps Content & Data Standardization Backend

## Project Type

`server`

## Overview

Backend-only platform for internal KnowledgeOps teams to author, govern, and publish knowledge items with:
- Template-driven content creation with immutable version history
- Controlled status lifecycle (Draft -> In Review -> Approved -> Published -> Archived)
- Reviewer-only approval workflow
- Full-text search (title, body, and tags), query suggestions, and trending terms
- CSV/XLSX bulk import with partial success and row-level diagnostics
- Export with optional masking and explanations
- Local async standardization pipeline with versioned outputs
- Feature flags, analytics, events, and operational monitoring

Single-organization, offline-capable deployment. No external service dependencies.

## Tech Stack

- **Language:** Rust (stable, 2021 edition)
- **Web framework:** Actix-web 4
- **ORM:** Diesel 2 (PostgreSQL)
- **Database:** PostgreSQL 16+
- **Password hashing:** Argon2id
- **Sensitive column encryption:** AES-256-GCM (app-level envelope encryption)
- **Container runtime:** Docker Compose
- **Background jobs:** In-process async workers (Tokio)

## Startup Instructions

Canonical startup command:

```bash
docker compose up --build
```

Legacy compatibility:

```bash
docker-compose up
```

No manual `export`, `.env` files, or host-side setup required. The `bootstrap-secrets` init container generates all secrets ephemerally at startup using `/dev/urandom`. The app binary runs embedded Diesel migrations and seeds roles + demo users automatically on first boot.

## Access Method

After `docker compose up --build`, the API is available inside the container network at port 8080. The standard access method for verification is via `docker compose exec`:

```bash
# Health check (recommended verification method)
docker compose exec app curl -s http://localhost:8080/api/v1/health

# Login as admin
docker compose exec app curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"changeme123!"}'
```

The port is also mapped to the host as `0.0.0.0:8080`, so in standard Docker environments the API is accessible at:

```
http://127.0.0.1:8080/api/v1
```

```bash
# Host-side access (works in standard Docker environments)
curl http://127.0.0.1:8080/api/v1/health
```

> **Note:** Some sandboxed or restricted Docker environments may block host-to-container port forwarding. If host-side `curl` times out, use the `docker compose exec` method shown above — it always works because it runs inside the container.

Use the returned `token` value from login in subsequent requests via the `X-Session-Token` header or the `ko_session` cookie.

Quick seeded demo credential for smoke testing:

```text
username: admin
password: changeme123!
```

### Reviewer verification example

```bash
# Full lifecycle verification from inside the container:
docker compose exec app curl -s http://localhost:8080/api/v1/health
# Expected: {"status":"healthy","components":{"database":"connected",...}}

docker compose exec app curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"reviewer","password":"changeme123!"}'
# Expected: {"data":{"token":"...","user":{"role":"Reviewer",...}},...}
```

## Verification Method

Broad test command:

```bash
./run_tests.sh
```

This runs both test layers against the live Docker stack:

**Layer 1 — Rust-native unit tests:** `cargo test` is run inside the test container (compose `test` profile). These `#[test]` functions in `tests/unit_tests.rs` cover crypto, search normalization, file signature checking, masking, audit redaction, pagination, z-score outlier detection, fingerprint determinism, and analytics filter validation.

**Layer 2 — True no-mock HTTP integration tests:** Real `curl` requests to the live Actix-web server with real auth middleware, real rate-limit middleware, and a real PostgreSQL database. No mocking at any layer. Includes RBAC/object-level authorization tests, search semantics, export/import checks, analytics filters, rate limiting, and ops alert diagnostics.

No measurable line coverage is produced by default; cargo-tarpaulin is installed but requires `--security-opt seccomp=unconfined` to run. The test runner:
1. Builds and starts the full app stack via `docker compose`
2. Waits for the app to be healthy
3. Runs `cargo test` in the test container (Layer 1 — Rust unit tests)
4. Runs `tests/api_integration_tests.sh` inside the app container (Layer 2 — HTTP integration tests)
5. Reports pass/fail results with per-test detail
6. Cleans up containers and volumes

Current route inventory is 70 unique `METHOD + PATH` endpoints under `/api/v1`, and all 70 have direct HTTP test coverage in `tests/api_integration_tests.sh`.

During development, targeted verification was done via `docker compose exec app curl` against individual endpoints.

## Authentication

Demo credentials (seeded automatically by the app binary on first boot):

| Role | Username | Password |
|---|---|---|
| Administrator | `admin` | `changeme123!` |
| Author | `author` | `changeme123!` |
| Reviewer | `reviewer` | `changeme123!` |
| Analyst | `analyst` | `changeme123!` |

All demo passwords must be changed immediately after first login. Passwords are stored as Argon2id hashes. The seeding path is for local development only and is not the production secret-management path.

## Roles and Workflows

### Roles

| Role | Capabilities |
|---|---|
| **Administrator** | Full governance: user/role management, channel/template creation, feature flags, operational oversight, archive controls. Cannot perform Reviewer-only approval. |
| **Author** | Create/edit own items from templates, submit for review, publish own approved items. Import access. No export authority. |
| **Reviewer** | Review items in review queue, approve or reject. Reviewer-only approval is a prompt-critical restriction. |
| **Analyst** | Search/retrieve, analytics, imports, exports, standardization mappings/jobs. No item approval or content edits. |

### Key Workflows

1. **Item lifecycle:** Draft -> In Review -> Approved -> Published -> Archived
2. **Reviewer-only approval:** Only Reviewers can transition items from In Review to Approved
3. **Auto-revert:** Items idle in In Review for 14 days automatically revert to Draft
4. **Publish binding:** Publishing is only available via the dedicated `POST /items/{id}/publish` endpoint, which locks a specific item_version_id and its template_version_id context. The generic transition endpoint blocks `Approved -> Published` to prevent unbounded publishing.
5. **Rollback:** Clone-forward only, limited to previous 10 versions
6. **Auto-numbering:** `KO-YYYYMMDD-#####` format, America/New_York boundary, daily reset
7. **Rate limiting:** 60 requests/minute per authenticated user
8. **CAPTCHA:** Local arithmetic CAPTCHA after 5 failed login attempts in 15 minutes

## Main Repo Contents

```
Cargo.toml              Rust project manifest
Dockerfile              Multi-stage production build
Dockerfile.test         Test runner image (for cargo-based tests)
docker-compose.yml      Bootstrap-secrets + DB + App services
init_db.sh              Canonical DB initialization path (delegates to app binary)
run_tests.sh            Broad Docker-contained test runner (Layer 1 + Layer 2)
diesel.toml             Diesel ORM configuration
tests/
  unit_tests.rs             Rust-native unit tests (run via cargo test)
  api_integration_tests.sh  True no-mock HTTP test suite (curl-based)
scripts/
  bootstrap.sh          Ephemeral secret generation (dev-only, not production)
  entrypoint.sh         Container entrypoint (wait for PG, start app)
  test_entrypoint.sh    Test container entrypoint
migrations/
  2024-01-01-.../       Initial schema (32 tables, indexes, triggers)
src/
  main.rs               App entrypoint: migrations, seeding, server bootstrap
  config.rs             Central config (all env reads here)
  schema.rs             Diesel schema definitions
  models.rs             Data models and DTOs
  errors.rs             Normalized error types and API error responses
  audit.rs              Shared audit logging with sensitive field redaction
  crypto.rs             Argon2id hashing, AES-256-GCM encryption, token generation
  logging.rs            Structured JSON logging
  alerts.rs             Durable on-disk alert spool (atomic write, fsync)
  search/               Query normalization and tsquery helpers
  import_export/        File signature checks, title normalization, masking
  standardization/      Pipeline execution, fingerprinting, unit normalization
  jobs/                 Background scheduler (auto-revert, trending, metrics, cleanup)
  api/
    mod.rs              Route configuration (70 endpoints)
    dto/                Response envelope helpers
    middleware/         Auth, rate-limit, request-id middleware
    handlers/           Route handlers for all endpoint groups
  services/             Business logic (auth, users, channels, tags, templates,
                        items, search, imports, exports, analytics, feature_flags)
```

## Architecture

### Layered Module Design

1. **API layer** — Actix-web routes, DTOs, handlers, auth/rate-limit guards under `/api/v1`
2. **Service layer** — Workflow orchestration, domain rules, business logic
3. **Persistence** — Diesel repositories, direct PostgreSQL queries, transactions
4. **Infrastructure** — Crypto, file storage, alert spool, background scheduler

### Cross-Cutting Contracts

- **Central config:** All environment variables read once in `config.rs`
- **Auth middleware:** Opaque session token validation with 12-hour sliding expiry
- **RBAC:** Role checks at route handler level, object-level ownership in services
- **Audit:** All mutations logged with actor, action, object, before/after state, redacted sensitive fields
- **Error normalization:** Consistent `{error: {code, message, details, request_id}}` envelope
- **Request ID:** UUID generated per request, threaded through logs and responses
- **Rate limiting:** Per-user 60 req/min via DB-backed sliding window

### Background Jobs

In-process Tokio scheduler handles:
- In-review idle auto-revert (14 days, checked every 15 minutes)
- Daily trending term recomputation from normalized search queries
- Hourly metrics snapshots
- Session and audit retention cleanup
- Standardization job processing from queue

### Data Model

32 PostgreSQL tables with UUID primary keys, including:
- Core: roles, users, sessions, channels, tags, templates, template_versions, items, item_versions
- Search: searches, search_history, search_trending_daily
- Import/Export: imports, import_rows, exports, export_artifacts
- Standardization: schema_mappings, schema_mapping_versions, standardization_jobs, standardized_models, standardized_records
- Ops: audits, events, metrics_snapshots, feature_flags, login_attempts, captcha_challenges, daily_counters, rate_limits

GIN full-text index on item_versions.search_vector with auto-update trigger. Search vectors include title (weight A), body (weight B), and tag names (weight C) for full-text relevance ranking.

## Important Notes

### Feature Flags

Feature flags are managed via the `/api/v1/feature-flags` endpoints. Administrator can create, enable/disable, and configure variant allocation. Analyst has read-only access. Flags are stored in the `feature_flags` table with optional variant/allocation JSON.

### Bootstrap Path

`./init_db.sh` is the canonical and only supported DB initialization path. It bootstraps required environment variables (DATABASE_URL, SESSION_SECRET, ENCRYPTION_KEY) via `scripts/bootstrap.sh`, waits for PostgreSQL, then either detects a healthy running app or invokes `knowledgeops --init-only`. The `--init-only` flag runs embedded Diesel migrations and idempotent role/user seeding, then exits cleanly. The bare `knowledgeops --init-only` binary requires DATABASE_URL, SESSION_SECRET, and ENCRYPTION_KEY to be present in the environment; it does not generate them itself. Always use `init_db.sh` rather than calling the binary directly. Usage: `docker compose exec app /app/init_db.sh`. This is a local-development bootstrap mechanism only.

### Offline/Local-Only Operation

This system operates entirely offline with no external service dependencies:
- No external CAPTCHA providers — local arithmetic challenges
- No external search engines — PostgreSQL full-text search
- No external message queues — DB-backed job queue
- No external alert services — durable on-disk alert spool (`/app/alerts_spool/`)
- Alert spool uses atomic write (tmpfile + fsync + rename) for durability
- Alert spool receives alerts for: all scheduler job failures (auto-revert, trending, metrics, retention, standardization) and internal server errors (5xx, rate-limited to one alert per 60 seconds to avoid spam)
- Diagnostic alert triggers: `POST /ops/diagnostic/error` (triggers INTERNAL_ERROR alert via real 5xx path) and `POST /ops/diagnostic/job-failure` (writes JOB_FAILURE alert via the same `write_alert` function the scheduler uses). Both are Administrator-only and audit-logged. These endpoints exist for integration test verification of the alert pipeline.
- Export format: CSV and XLSX are supported; unsupported formats return a clear error

### Security Disclosures

- Passwords stored as Argon2id hashes
- Sensitive columns (email, phone) encrypted at rest via AES-256-GCM with runtime-generated keys
- Session tokens stored as SHA-256 hashes; raw tokens never persisted
- Encryption keys generated ephemerally by `bootstrap-secrets` container — keys are lost on `docker compose down -v` (by design for dev; production needs external key management)
- SQL injection prevention via Diesel parameterized queries exclusively
- Export masking covers email/phone/SSN patterns plus template-designated sensitive fields
- Audit records redact sensitive fields (password, token, secret, hash, encrypted)
- 7-year audit retention with scheduled cleanup
- Slow query logging threshold: 500ms
- API responses never expose raw stack traces or internal paths

### Analytics Custom Filters

Both `GET /analytics/kpis` and `GET /analytics/operational` accept optional query parameters to scope results:

| Parameter | Type | Description |
|---|---|---|
| `channel_id` | UUID | Filter items/imports by channel |
| `status` | String | Filter items by status (Draft, InReview, Approved, Published, Archived) |
| `from` | Date (YYYY-MM-DD) | Include only records created on or after this date |
| `to` | Date (YYYY-MM-DD) | Include only records created on or before this date |
| `owner_user_id` | UUID | Filter items by owner (KPI endpoint only) |

Invalid status values return 400. Invalid date formats return 400.

### Import Template and Duplicate Detection

Import CSV/XLSX templates include the `auto_number` column alongside `title`, `body`, and schema-defined fields. During import, duplicate detection is performed in two ways:

1. **By auto-number:** If `auto_number` is provided and matches an existing item, the row is rejected.
2. **By normalized title + channel within 90 days:** Both the imported title and stored titles are normalized (trimmed, lowercased, whitespace-collapsed) before comparison. Matches within the same channel in the last 90 days are rejected.

Auto-number generation for accepted rows always uses the standard `KO-YYYYMMDD-#####` format regardless of any `auto_number` value in the import data.

### Rate Limiting and CAPTCHA

- Authenticated: 60 requests/minute per user
- Login: CAPTCHA required after 5 failed attempts in 15-minute window
- CAPTCHA challenges expire after 5 minutes
