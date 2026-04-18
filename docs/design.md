# KnowledgeOps Content & Data Standardization Backend — Authoritative Design Plan

## 0. Document Status

- **Purpose:** Implementation-grade execution contract for backend-only delivery.
- **API style:** Versioned JSON REST under `/api/v1`.
- **Deployment model:** Single-organization, offline-capable, single Dockerized service stack (app + PostgreSQL, no external network dependencies).
- **Planning posture:** This file is authoritative; scaffold/development should follow this plan directly.

## 1. Tech Stack Summary

### 1.1 Core stack
- **Language/runtime:** Rust (stable), Tokio async runtime.
- **Web framework:** Actix-web.
- **ORM/query layer:** Diesel + PostgreSQL.
- **DB:** PostgreSQL 16+.
- **Serialization/validation:** Serde + custom validators + regex crates.
- **Password hashing:** Argon2id.
- **Sensitive-column crypto:** Application-level envelope encryption (AES-256-GCM or equivalent) with runtime-generated local key material (not committed).
- **Background jobs:** DB-backed job records + async workers in-process.
- **File handling:** CSV/XLSX parsing libraries, upload signature sniffing, on-disk artifact storage via Docker volumes.

### 1.2 Operational stack
- **Container runtime:** Docker Compose (`docker compose up --build`).
- **DB init path:** `./init_db.sh` only.
- **Broad verification path:** `./run_tests.sh` (Docker-contained).
- **Logging:** Structured JSON logs to stdout + file sink.
- **Alerting:** Local log entries + durable on-disk alert spool queue.

## 2. Product Overview

Backend platform for internal KnowledgeOps teams to:
1. Author and govern publishable knowledge items.
2. Define reusable templates with strict field and cross-field rules.
3. Preserve immutable version history and controlled publishing/rollback.
4. Run governed import/export and local standardization pipelines.
5. Provide searchable retrieval, analytics, feature flags, and auditability.

## 3. System Overview

### 3.1 Primary technical shape
- Monolithic modular backend with explicit domain boundaries.
- PostgreSQL is source of truth for transactional domains.
- File artifacts (imports/exports/alerts) stored locally on disk volumes.
- Async worker loop handles scheduled and queued jobs without external broker.

### 3.2 Cross-cutting contracts fixed early
- Centralized config loading.
- Centralized auth/session middleware.
- Shared RBAC and object-level authorization guards.
- Shared validation and normalized error payloads.
- Shared audit logging service.
- Shared observability (request, query latency, exceptions).

## 4. In-Scope Modules

1. Authentication + sessions + local CAPTCHA + rate limiting.
2. User/role management (Administrator, Author, Reviewer, Analyst).
3. Channels/tags registry.
4. Template authoring + immutable template versions.
5. Knowledge items + immutable item versions + numbering.
6. Lifecycle transitions (Draft → In Review → Approved → Published → Archived) + auto-revert job.
7. Search/retrieval + suggestions + trending + user history.
8. Bulk import/export with validation, partial success, diagnostics, masking/explanations.
9. Schema mappings + local async standardization pipeline + versioned standardized models.
10. Audits, events, feature flags, KPI/operational analytics, metrics snapshots.
11. Health checks, slow query logging, error-rate tracking, durable local alert spool.

## 5. Explicit Out-Of-Scope

- Frontend/UI delivery.
- Multi-tenant or multi-organization partitioning.
- External CAPTCHA providers, email/SMS, webhooks, cloud queues.
- External search engines (Elasticsearch/OpenSearch).
- Real-time push channels (WebSocket/SSE).
- Distributed service decomposition/microservices.

## 6. Actors And Roles

| Actor | Purpose |
|---|---|
| Administrator | Platform governance, user/role administration, channel/template governance, operational oversight, archive controls |
| Author | Create/edit items from templates, submit for review, publish approved items they own |
| Reviewer | Review items in-review, approve or reject back to draft |
| Analyst | Search/retrieve, run data standardization, analytics, exports, import diagnostics |
| System Worker | Scheduled and queued background transitions, trending recompute, snapshots, retention, spool writes |

## 7. Actor Success Paths

### 7.1 Administrator
1. Bootstrap first admin deterministically using `./init_db.sh` path.
2. Login, create users and assign roles.
3. Create channels/tags/templates and activate template versions.
4. Oversee audits, alerts, feature flags, and ops snapshots.

### 7.2 Author
1. Login.
2. Create item using active template version.
3. Fix validation errors until save succeeds.
4. Move Draft → In Review.
5. After Reviewer approval, publish a specific item version bound to template-version context.

### 7.3 Reviewer
1. Login and list In Review items.
2. Evaluate immutable version under review.
3. Approve (Reviewer-only action) or return to Draft with reason.

### 7.4 Analyst
1. Search and retrieve published or role-allowed records.
2. Run imports and inspect row-level diagnostics.
3. Configure schema mappings and execute standardization jobs.
4. Export datasets with optional explanations and masking.
5. Query KPI/operational analytics snapshots.

## 8. Architecture And Module Boundaries

### 8.1 Layered module map
1. **API layer** (`/api/v1` routes, DTOs, handlers, auth guards).
2. **Application services** (workflow orchestration, domain rules).
3. **Domain modules** (auth, templates, items, search, import/export, pipeline, analytics).
4. **Persistence adapters** (Diesel repositories, transactions, query specs).
5. **Infra adapters** (crypto, file storage, alert spool, scheduler).

### 8.2 Boundary rules
- Handlers do transport concerns only.
- Services enforce business rules and call repositories.
- Repositories never enforce role logic.
- Background jobs call same services used by API, not bypass paths.
- All mutating actions route through shared audit service.

## 9. Domain Model And Data Model

### 9.1 Required tables (minimum set, all UUID PK + created_at + updated_at)
- `users`, `roles`, `sessions`
- `templates`, `template_versions`
- `items`, `item_versions`
- `tags`, `channels`
- `audits`, `searches`, `imports`, `exports`
- `metrics_snapshots`, `events`, `feature_flags`

### 9.2 Additional tables required for prompt-critical behavior
- `item_version_tags` (immutable tag association per item version).
- `search_history` (per-user newest-200 history, clearable).
- `search_trending_daily` (precomputed trending terms).
- `import_rows` (row-level validation diagnostics and per-row disposition).
- `export_artifacts` (artifact metadata, checksum, masking/explanation flags).
- `login_attempts` (rolling-window failed logins).
- `captcha_challenges` (local challenge lifecycle).
- `daily_counters` (auto-number concurrency-safe daily sequence).
- `schema_mappings`, `schema_mapping_versions`.
- `standardization_jobs`, `standardized_models`, `standardized_records`.

### 9.3 Critical entity fields/constraints
- `users.username`: unique, case-insensitive normalized index.
- `roles.name`: enum-like unique set {Administrator, Author, Reviewer, Analyst}.
- `sessions.token_hash`: unique; opaque token stored hashed; `last_activity_at` sliding.
- `templates.slug`: unique; template-level metadata only.
- `template_versions`: immutable JSON field definitions + constraints + validation rules.
- `items.auto_number`: unique, format `KO-YYYYMMDD-#####`.
- `items.status`: enum with state-machine guard.
- `item_versions`: immutable snapshot including `template_version_id`, content fields, structured payload, encrypted sensitive payload segment.
- `audits`: actor, action, object type/id, before/after (masked), reason, correlation/request id.
- `imports/exports`: job state, artifact references, options used, summary counts.
- `feature_flags`: key unique, enabled bool, optional variant allocation JSON.

### 9.4 Required indexes
- **GIN full-text:** item title/body search document.
- **BTree:** `items.auto_number`, `items.status`, `items.channel_id`, `items.published_at`.
- Additional: `item_versions(item_id, version_no desc)`, `audits(object_type, object_id, created_at)`, `searches(created_at)`, `search_history(user_id, created_at desc)`.

## 10. Authoritative Business Rules

### 10.1 Authentication/session
- Username/password login only.
- Session model: opaque DB-backed session with **12-hour sliding inactivity expiry**.
- Middleware refreshes `last_activity_at` on authenticated requests.
- Expired sessions invalidated server-side and removed by cleanup job.

### 10.2 Rate limiting + CAPTCHA
- Authenticated APIs: 60 requests/minute/user.
- Unauthenticated login path: IP+username keyed limit and failed-attempt tracking.
- CAPTCHA required after repeated failures in rolling window (default: 5 failures / 15 minutes, configurable).
- CAPTCHA challenge is local, no external service.

### 10.3 Template/form rules
- Field types: `string | number | date | enum | text`.
- Text max length: 2,000 chars.
- Required fields enforced.
- Regex constraints supported for identifier fields.
- Enum values constrained to configured option set.
- Cross-field rules supported (example: if `graded=true`, then `score` must be 0..100).
- Template version immutable after creation.

### 10.4 Item/version rules
- Item updates create new immutable `item_versions` entries.
- Editing content allowed only in Draft.
- Every item has a unique auto-number generated at creation time.
- Publishing references explicit `item_version_id` and bound `template_version_id` from that version.

### 10.5 Auto-numbering
- Format: `KO-YYYYMMDD-#####`.
- Date portion uses **America/New_York** boundary.
- Counter resets daily at NY midnight.
- `daily_counters` row lock (`SELECT ... FOR UPDATE`) ensures concurrency safety.
- If counter exceeds 99,999 in a day, return conflict (`AUTO_NUMBER_DAILY_LIMIT_REACHED`).

### 10.6 Rollback semantics
- Rollback is clone-forward only (immutability preserved).
- Source version must be within previous 10 versions of current latest.
- Rollback creates a **new latest version** copied from source plus rollback metadata.

### 10.7 Duplicate import checks
- Duplicate if auto-number already exists.
- Duplicate if normalized `title + channel` matches an existing item within prior 90 days.
- Normalization: trim, lowercase, collapse whitespace, unicode fold.

### 10.8 Import/export behavior
- Import accepts CSV/XLSX only, max 10 MB, signature + extension checks.
- Pre-import validation runs before persistence decisions.
- Partial success required: valid rows persisted, invalid rows rejected with row diagnostics.
- Export options:
  - include_explanations: optional
  - mask_sensitive: optional (known patterns + designated sensitive fields)

### 10.9 Search/history/trending
- Search over title/body/tags with sort by relevance or newest.
- Filters: channel, tag, time window.
- Suggestions/trending derived from normalized locally stored queries from last 30 days.
- Per-user history keeps newest 200 entries, clear operation supported.

### 10.10 Standardization pipeline
- Runs fully local async jobs.
- Produces versioned standardized model outputs from schema mappings.
- Deduplication via deterministic fingerprint (normalized text + key fields).
- Missing value imputation only via explicit defaults from mapping config.
- Never impute personally identifying fields.
- Unit normalization to US customary.
- Timestamp normalization to America/New_York while preserving raw values.
- Outliers flagged by z-score threshold `|z| >= 3`.

## 11. State Machines And Lifecycles

### 11.1 Item status lifecycle (authoritative)

| From | To | Allowed actor(s) | Notes |
|---|---|---|---|
| Draft | In Review | Author(owner), Administrator | Freeze editable snapshot for review |
| In Review | Draft | Reviewer, System(auto-revert), Administrator | Reviewer rejection or 14-day idle auto-revert |
| In Review | Approved | **Reviewer only** | Prompt-critical restriction |
| Approved | Published | Author(owner), Administrator | Requires explicit item version |
| Published | Archived | Administrator | Terminal archival state |

Illegal transitions return `409 INVALID_TRANSITION`.

### 11.2 In-review idle auto-revert
- Scheduled job scans In Review items.
- If `now_ny - entered_in_review_at >= 14 days` and no approval transition occurred, move to Draft with audit reason `AUTO_REVERT_IDLE_REVIEW`.

### 11.3 Session lifecycle
- Created on login.
- Active while requests occur within 12-hour inactivity window.
- Expired sessions rejected with 401 and deleted asynchronously.

### 11.4 Job lifecycle
- States: `queued -> running -> succeeded | failed | partial_succeeded | cancelled`.
- Retry policy: up to 3 retries for transient errors with exponential backoff.

## 12. Permissions And Authorization Model

### 12.1 Role capabilities (high-level)
- **Administrator:** full governance except Reviewer-only approval restriction.
- **Author:** CRUD own Draft items, submit review, publish own approved items, create/view own import jobs and diagnostics; **no export authority**.
- **Reviewer:** view review queue, approve/reject transitions, read items across channels.
- **Analyst:** search/read, analytics, imports, exports, standardization mappings/jobs; no item approval or item content edits.

### 12.4 Export authority (explicit)
- Export job creation/download is restricted to **Analyst** and **Administrator** roles.
- Author and Reviewer roles cannot trigger or download exports.
- This rule must stay aligned across route guards, service policy checks, and repository query scoping.

### 12.2 Object-level rules
- Authors can mutate only items where `owner_user_id = session.user_id`.
- Reviewer actions apply to items in review across org.
- Admin can read/mutate all objects where operation is not Reviewer-exclusive.

### 12.3 Enforcement points
1. Route guard (role presence).
2. Service-level action policy.
3. Repository query scoping for owner-filtered paths.

## 13. Validation And Error Handling

### 13.1 Validation layers
- Transport validation (required fields/type formats).
- Domain validation (status transitions, template constraints, cross-field rules).
- Persistence validation (unique/foreign-key/conflict mapping).

### 13.2 Normalized error contract
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "One or more fields failed validation",
    "details": [{"field": "score", "reason": "must be between 0 and 100 when graded=true"}],
    "request_id": "..."
  }
}
```

### 13.3 Canonical status mapping
- 400 malformed request
- 401 unauthenticated/expired session
- 403 forbidden
- 404 not found
- 409 conflict/illegal transition/duplicate
- 413 file too large
- 415 unsupported media/signature
- 422 semantic validation failure
- 429 rate limited / CAPTCHA required
- 500 internal error (sanitized)

## 14. Security, Compliance, And Data Governance

1. SQL injection prevention via Diesel parameterization only.
2. Password storage via Argon2id.
3. Sensitive columns encrypted at rest via app-level envelope encryption.
4. Export masking covers:
   - regex patterns (email/phone/identifier patterns)
   - template-designated sensitive fields.
5. Audit records required for create/update/status transitions/publish/rollback/import/export.
6. Audit retention: minimum 7 years (scheduled purge only after retention threshold).
7. No raw stack traces/internal paths exposed in API responses.
8. Sensitive values redacted from logs and audits.

## 15. Offline, Queueing, Reliability, And Background Jobs

### 15.1 Job categories
- Import processing.
- Export generation.
- In-review idle auto-revert.
- Daily trending recompute.
- Metrics snapshot scheduler.
- Audit/session retention cleanup.
- Standardization job execution.

### 15.2 Reliability behavior
- DB-backed job row lock to avoid duplicate execution.
- Idempotency keys for import/export trigger endpoints.
- Retry only transient classes; permanent validation errors are terminal.
- Durable artifact metadata and checksums for import/export outputs.

### 15.3 Alert queue durability
- Exceptions produce:
  1) structured error log entry
  2) atomic write to on-disk spool file (`alerts/{timestamp}_{uuid}.json`).
- Spool writer must fsync/atomic-rename to avoid partial files.

## 16. Reporting, Analytics, Search, Import, Export Behavior

### 16.1 Search semantics
- Query parse + normalization pipeline.
- PostgreSQL full-text query over indexed search doc.
- Sorting:
  - `relevance` (rank desc)
  - `newest` (published_at desc)

### 16.2 Suggestions and trending
- Suggestions from normalized queries in last 30 days with prefix match + frequency weighting.
- Trending terms computed daily and stored in `search_trending_daily`.

### 16.3 KPI/operational analytics
- Filterable metrics endpoint (time range, channel, status, actor role).
- Scheduled snapshots persisted to `metrics_snapshots`.
- Analytics export endpoint supports CSV artifact generation.

### 16.4 Import flow
1. Upload file + options.
2. Signature/size/type checks.
3. Parse rows.
4. Validate required/format/enum/cross-field/duplicate checks.
5. Persist valid rows in transaction batches.
6. Persist row diagnostics for invalid rows.
7. Return partial success summary.

### 16.5 Export flow
1. Submit export request with filters/options.
2. Async generation writes artifact file.
3. Optional explanation columns appended.
4. Optional masking applied before write.
5. Download endpoint serves artifact + metadata.

## 17. Runtime, Config, And Ops Contract

### 17.1 Runtime entrypoints
- Canonical runtime: `docker compose up --build`.
- Legacy compatibility string in README: `docker-compose up`.
- DB initialization only via `./init_db.sh`.
- Broad test gate only via `./run_tests.sh`.

### 17.2 Config model
- Central config module; no scattered env reads in business logic.
- No committed `.env` files.
- Runtime bootstrap script generates local secrets/config values into Docker-managed volume.
- Same bootstrap model used by runtime and `./run_tests.sh`.

### 17.3 Deterministic first-admin bootstrap
- `./init_db.sh` checks for existing Administrator.
- If none exists, creates first admin through deterministic script path (documented CLI + persisted local bootstrap artifact in Docker volume, not repository).

### 17.4 Operational observability
- Health endpoint validates app, DB connectivity, and spool directory writability.
- Slow query log threshold: >500 ms.
- Error-rate counters tracked in process and snapshotted.

### 17.5 README contract (mandatory delivery artifact)
Final `README.md` must explicitly include all of the following:
1. **Project type near top:** exact declaration `backend`.
2. **Startup instructions:** canonical `docker compose up --build` and the exact legacy compatibility string `docker-compose up`.
3. **Access method (concrete):**
   - Base URL after startup: `http://127.0.0.1:8080/api/v1` (or documented resolved host port if different).
   - Health check path: `GET /api/v1/health`.
   - Reviewer verification example with `curl` against the running containerized service.
4. **Verification method:** broad test command `./run_tests.sh`, plus mention of targeted local test cadence used during development.
5. **Demo credentials for every role:** Administrator, Author, Reviewer, Analyst bootstrap/demo accounts and how they are provisioned safely.
6. **Main repo contents:** concise map of key directories/files and their responsibilities.
7. **Architecture summary:** module boundaries and critical cross-cutting contracts (auth, RBAC, validation, audit, jobs, search, pipeline).
8. **Important operational notes/disclosures:** feature-flag behavior, deterministic first-admin bootstrap path, offline/local-only alert queue behavior, and local-development secret/bootstrap disclosures.

## 18. Interface Contracts

- API base `/api/v1`, JSON by default.
- Auth via opaque session token cookie/header.
- Consistent pagination params (`page`, `page_size`, default/safe max).
- Consistent filter params and sort enums.
- All mutating endpoints return audit correlation id.
- Full endpoint inventory and payload shapes: see `docs/api-spec.md`.

## 19. Non-Functional Requirements

1. Deterministic behavior for numbering, rollback, and dedupe.
2. Timezone normalization authority: America/New_York.
3. Service restart must not lose queued jobs or alert spool artifacts.
4. API responses sanitized and traceable via request id.
5. Upload handling bounded by strict size/type constraints.
6. Search and listing endpoints require pagination defaults and bounded page sizes.

## 20. Verification Strategy

- During implementation slices: targeted unit/integration and route-family tests only.
- Broad verification path reserved for `./run_tests.sh`.
- API verification priority: true no-mock HTTP tests against real app + PostgreSQL for critical `METHOD + PATH` surfaces.
- Coverage target: >=90% line coverage measured in Dockerized path (details in `docs/test-coverage.md`).

## 21. Dependency And Parallelism Plan

### 21.1 Serial prerequisites (must settle first)
1. Config/bootstrap/security primitives.
2. Core schema + migrations + shared error/audit/logging paths.
3. RBAC/object-ownership guard framework.
4. Session/captcha/rate-limit middleware.

### 21.2 Parallel work packages after prerequisites
1. **Package A:** Auth/governance + template/item/version/workflow core.
2. **Package B:** Search/import/export + analytics/events/feature-flags.
3. **Package C:** Standardization pipeline + job orchestration + alert/ops reliability.

### 21.3 Fan-in condition
- Merge only after shared contracts (error shape, auth guard, audit schema, job state enum) are stable and tested.

## 22. Implementation Phases

1. **Phase 0 – Foundations**
   - Project scaffold, config, migrations baseline, init scripts, logging/error/audit backbone.
2. **Phase 1 – Auth + Core Lifecycle**
   - Users/roles/sessions/captcha/rate-limit + templates/items/versioning/status transitions/publish/rollback.
3. **Phase 2 – Retrieval + Data Ops APIs**
   - Search/suggestions/trending/history + imports/exports + analytics/events/flags.
4. **Phase 3 – Standardization + Reliability**
   - Schema mappings, standardization jobs, model version outputs, alert spool, schedulers.
5. **Phase 4 – Hardening + Coverage + README finalization**
   - Coverage threshold enforcement, docs consistency, operational scripts polish.

## 23. Phase Checkpoints

### 23.1 Phase 0 exit
- Migrations create required tables + essential extras.
- `./init_db.sh` functional for schema + role seed + first-admin deterministic path.
- Unified error + audit + logging contracts implemented.

### 23.2 Phase 1 exit
- Auth/session/RBAC enforced on real endpoints.
- Template and item version immutability verified.
- Workflow transitions and rollback/publish semantics enforced.

### 23.3 Phase 2 exit
- Full-text search/filter/sort working.
- Suggestion/trending and history cap/clear working.
- Import partial success + row diagnostics working.
- Export masking/explanations working.

### 23.4 Phase 3 exit
- Standardization jobs produce versioned outputs via schema mappings.
- Auto-revert scheduler and daily trending/snapshot jobs operational.
- Durable alert spool verified.

### 23.5 Phase 4 exit
- Test coverage threshold reached and measured.
- README satisfies runtime/access/verification/credentials contract.
- API spec and coverage docs consistent with implementation.

## 24. Definition Of Done

Work is done only when all are true:
1. Prompt-required domains implemented (not placeholder).
2. RBAC + object-level authorization enforced across routes/services/repos.
3. Immutable versioning, rollback clone-forward, and publish binding behavior verified.
4. Search/import/export/analytics/standardization flows operate end-to-end.
5. Security/compliance controls (Argon2id, masking, audits, upload constraints, rate limiting/CAPTCHA) enforced.
6. Scheduled/background behaviors working with durable state.
7. Docker runtime and broad test scripts (`docker compose up --build`, `./init_db.sh`, `./run_tests.sh`) documented and functional.
8. Coverage evidence and endpoint mapping updated in `docs/test-coverage.md`.

## 25. Deliverables

- Actix-web + Diesel backend implementation.
- PostgreSQL migrations and seed/bootstrap logic.
- Runtime scripts: `./init_db.sh`, `./run_tests.sh`.
- Docker Compose runtime/test configuration.
- API endpoints documented in `docs/api-spec.md`.
- Coverage matrix and endpoint-test mapping in `docs/test-coverage.md`.
- Repo `README.md` with startup/access/verification/credentials and architecture summary.

## 26. Assumptions, Dispositions, And Open Items

### 26.1 Locked assumptions (prompt-preserving)
1. Single-role-per-user model (one of Administrator/Author/Reviewer/Analyst) to keep authorization deterministic.
2. CAPTCHA default threshold/window values are configurable with secure defaults (5 failures/15 minutes trigger).
3. Admin bootstrap is script-driven (not public API) to minimize attack surface.

### 26.2 Explicitly unresolved (none blocking)
- None currently blocking planning; all critical behavior contracts above are fixed for scaffold/development.
