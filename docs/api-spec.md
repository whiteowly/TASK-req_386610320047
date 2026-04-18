# KnowledgeOps Backend API Specification (`/api/v1`)

## 1. Global API Conventions

### 1.1 Base path and format
- Base path: `/api/v1`
- Content type: `application/json` (except file upload/download endpoints)
- Timestamp format: ISO-8601 with timezone offset; normalization authority is `America/New_York` where business rules require local-day semantics.

### 1.2 Authentication model
- Opaque DB-backed session token, returned on login.
- Session inactivity timeout: 12 hours (sliding).
- Transport: HttpOnly cookie `ko_session` and optional `X-Session-Token` header for non-browser clients.

### 1.3 Standard success envelope
```json
{
  "data": {},
  "meta": {"request_id": "..."}
}
```

### 1.4 Standard error envelope
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "One or more fields failed validation",
    "details": [],
    "request_id": "..."
  }
}
```

### 1.5 Shared status/error notes
- `400` malformed input
- `401` unauthenticated/session expired
- `403` forbidden by role/object policy
- `404` resource not found
- `409` conflict/invalid transition/duplicate
- `413` upload too large (>10 MB)
- `415` unsupported media/signature
- `422` domain rule violation
- `429` rate limit exceeded or captcha required

## 2. Health & Operational Readiness

| METHOD | PATH | Auth | Request | Response | Key validations / notes |
|---|---|---|---|---|---|
| GET | `/api/v1/health` | None | None | service/db/spool/scheduler status summary | Must not leak secrets/internal paths |

## 3. Authentication & Sessions

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| POST | `/api/v1/auth/login` | None | `{ username, password, captcha_id?, captcha_answer? }` | session token (cookie/header), user role/profile | CAPTCHA required after repeated failures in rolling window; `401` bad creds; `429 CAPTCHA_REQUIRED` |
| POST | `/api/v1/auth/captcha/challenge` | None | `{ username }` or empty | `{ captcha_id, challenge_prompt, expires_at }` | Local challenge only; no external providers |
| POST | `/api/v1/auth/logout` | Any authenticated role | None | logout confirmation | Invalidates current session |
| GET | `/api/v1/auth/me` | Any authenticated role | None | current user identity, role, session expiry info | `401` if expired/not present |

> First Administrator bootstrap is script-based via `./init_db.sh` and intentionally has **no public API endpoint**.

## 4. User & Role Governance

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| GET | `/api/v1/users` | Administrator | query: pagination, role filter, active flag | user list page | No password/sensitive columns in response |
| POST | `/api/v1/users` | Administrator | `{ username, password, role, email?, phone?, active? }` | created user | Username unique (case-insensitive), role in allowed set |
| PATCH | `/api/v1/users/{user_id}` | Administrator | mutable profile/active fields, optional role change | updated user | Role transitions audited |
| POST | `/api/v1/users/{user_id}/reset-password` | Administrator | `{ new_password }` | reset confirmation | Argon2id hash regeneration, audit required |

## 5. Channels & Tags Registry

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| GET | `/api/v1/channels` | Any authenticated role | optional search/pagination | channel list | Unique normalized names |
| POST | `/api/v1/channels` | Administrator | `{ name, description? }` | created channel | duplicate name => `409` |
| PATCH | `/api/v1/channels/{channel_id}` | Administrator | `{ name?, description?, active? }` | updated channel | archived/inactive channels not allowed for new publishes |
| GET | `/api/v1/tags` | Any authenticated role | optional prefix/search/pagination | tag list | - |
| POST | `/api/v1/tags` | Administrator, Author | `{ name }` | created tag | normalized unique tag names |

## 6. Template & Template Version APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| POST | `/api/v1/templates` | Administrator | `{ name, slug, description?, channel_scope? }` | template record | slug unique |
| GET | `/api/v1/templates` | Any authenticated role | filters/pagination | template list | includes active version metadata |
| GET | `/api/v1/templates/{template_id}` | Any authenticated role | - | template details | `404` if missing |
| POST | `/api/v1/templates/{template_id}/versions` | Administrator | field schema + constraints + cross-field rules + change_note | created immutable template version | field type enum enforced; text max 2000; regex compilable; enum options non-empty |
| GET | `/api/v1/templates/{template_id}/versions` | Any authenticated role | pagination | version list | immutable history |
| GET | `/api/v1/templates/{template_id}/versions/{version_id}` | Any authenticated role | - | one version schema | - |
| POST | `/api/v1/templates/{template_id}/versions/{version_id}/activate` | Administrator | optional activation note | template active version updated | cannot mutate activated version payload; only pointer changes |

## 7. Item, Item Version, Workflow, Publish APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| POST | `/api/v1/items` | Author, Administrator | `{ template_id, template_version_id?, channel_id, title, body, fields, tags[] }` | created item + version + auto_number | auto-number `KO-YYYYMMDD-#####`; schema and cross-field validation required |
| GET | `/api/v1/items` | Any authenticated role | filters (status/channel/tag/time), pagination, sort | item list | object-scope filtering by role |
| GET | `/api/v1/items/{item_id}` | Any authenticated role | - | item aggregate with latest version pointer | object-level access checks |
| PATCH | `/api/v1/items/{item_id}` | Author(owner), Administrator | `{ title?, body?, fields?, tags?, change_note? }` | new latest item version | allowed only when item status=Draft |
| GET | `/api/v1/items/{item_id}/versions` | Any authenticated role | pagination | version history | immutable list descending |
| GET | `/api/v1/items/{item_id}/versions/{version_id}` | Any authenticated role | - | version snapshot including template context | - |
| POST | `/api/v1/items/{item_id}/rollback` | Author(owner), Administrator | `{ source_version_id, reason }` | new latest version cloned from source | source must be among previous 10 versions; clone-forward only |
| POST | `/api/v1/items/{item_id}/transitions` | Role-based by transition | `{ to_status, reason? }` | updated status + audit id | state machine enforcement + `409 INVALID_TRANSITION` |
| POST | `/api/v1/items/{item_id}/publish` | Author(owner), Administrator | `{ item_version_id, publish_note? }` | published item with bound item/template version ids | item must be Approved; publish binds selected item version and its template-version context |

### 7.1 Transition permission notes
- `In Review -> Approved`: **Reviewer only**.
- `In Review -> Draft`: Reviewer/Administrator/manual or system auto-revert.
- `Published -> Archived`: Administrator only.

## 8. Search & Retrieval APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| GET | `/api/v1/search` | Any authenticated role | query: `q`, `sort=relevance|newest`, `channel`, `tag`, `from`, `to`, pagination | ranked items + snippets/highlights metadata | records normalized query in local search store |
| GET | `/api/v1/search/suggestions` | Any authenticated role | `prefix`, optional `limit` | suggestion list | derived from normalized last-30-day local queries |
| GET | `/api/v1/search/trending` | Any authenticated role | optional `window_days` (default 30) | trending terms + counts | derived from daily computed local aggregates |
| GET | `/api/v1/search/history` | Any authenticated role | pagination | per-user newest-first history (max retained 200) | capped to newest 200 entries/user |
| DELETE | `/api/v1/search/history` | Any authenticated role | optional `before` timestamp else all | clear confirmation | clears current user history |

## 9. Bulk Import APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| GET | `/api/v1/imports/templates/{template_version_id}` | Author, Administrator, Analyst | query: `format=csv|xlsx` | downloadable import template artifact | generated from template version definition |
| POST | `/api/v1/imports` | Author, Administrator, Analyst | multipart upload + metadata (`template_version_id`, `channel_id`, options) | import job accepted with id | CSV/XLSX only, max 10 MB, signature+extension checks |
| GET | `/api/v1/imports` | Author, Administrator, Analyst | filters/pagination | import jobs list | object-scope visibility by role |
| GET | `/api/v1/imports/{import_id}` | Author(owner), Administrator, Analyst | - | import summary (`received/accepted/rejected`, status) | includes partial success counts |
| GET | `/api/v1/imports/{import_id}/errors` | Author(owner), Administrator, Analyst | pagination | row-level diagnostics | includes row number, field, code, message |
| GET | `/api/v1/imports/{import_id}/result` | Author(owner), Administrator, Analyst | - | persisted row refs + rejected row refs | duplicate checks: auto-number and normalized title+channel in 90-day window |

## 10. Export APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| POST | `/api/v1/exports` | Analyst, Administrator | `{ scope_filters, format, include_explanations, mask_sensitive }` | accepted export job id | options audited; format constrained |
| GET | `/api/v1/exports` | Analyst, Administrator | filters/pagination | export jobs list | - |
| GET | `/api/v1/exports/{export_id}` | Analyst, Administrator | - | export metadata/status | includes masking/explanation settings used |
| GET | `/api/v1/exports/{export_id}/download` | Analyst, Administrator | - | downloadable artifact | masking rules apply if requested; sensitive-field masking + pattern masking |

## 11. Schema Mapping & Standardization Pipeline APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| POST | `/api/v1/schema-mappings` | Analyst, Administrator | `{ name, source_scope, description? }` | created schema mapping | unique name per org |
| GET | `/api/v1/schema-mappings` | Analyst, Administrator | filters/pagination | mapping list | - |
| GET | `/api/v1/schema-mappings/{mapping_id}` | Analyst, Administrator | - | mapping details | - |
| POST | `/api/v1/schema-mappings/{mapping_id}/versions` | Analyst, Administrator | `{ mapping_rules, explicit_defaults, unit_rules, timezone_rules, fingerprint_keys, pii_fields }` | immutable mapping version | explicit defaults required for imputation; forbid inferred PII imputation |
| GET | `/api/v1/schema-mappings/{mapping_id}/versions` | Analyst, Administrator | pagination | mapping version list | immutable history |
| POST | `/api/v1/standardization/jobs` | Analyst, Administrator | `{ mapping_version_id, source_filters, run_label? }` | queued job id | idempotency key supported |
| GET | `/api/v1/standardization/jobs` | Analyst, Administrator | filters/pagination | jobs list | includes state/retry counters |
| GET | `/api/v1/standardization/jobs/{job_id}` | Analyst, Administrator | - | job detail, counters, failure info | - |
| GET | `/api/v1/standardization/models` | Analyst, Administrator | filters/pagination | standardized model versions | versioned outputs only |
| GET | `/api/v1/standardization/models/{model_id}` | Analyst, Administrator | - | model metadata + mapping version linkage | includes source window + quality stats |
| GET | `/api/v1/standardization/models/{model_id}/records` | Analyst, Administrator | pagination/filters | standardized records | includes dedupe markers, outlier flags, preserved raw values |

## 12. Events, Metrics, Analytics, Feature Flags

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| POST | `/api/v1/events` | Any authenticated role | `{ event_type, occurred_at?, payload }` | accepted event id | payload size/type guardrails; sensitive field redaction before persistence/logging |
| GET | `/api/v1/events` | Analyst, Administrator | filters/pagination | event stream page | - |
| POST | `/api/v1/metrics/snapshots` | Analyst, Administrator | `{ range, dimensions?, force? }` | snapshot job/record | supports scheduled + manual runs |
| GET | `/api/v1/metrics/snapshots` | Analyst, Administrator | filters/pagination | snapshot list | - |
| GET | `/api/v1/analytics/kpis` | Analyst, Administrator | filters (`from`,`to`,`channel`,`status`) | KPI aggregates | local-only data sources |
| GET | `/api/v1/analytics/operational` | Analyst, Administrator | filters | operational counters/error rates/latencies | combines metrics snapshots + live counters |
| POST | `/api/v1/analytics/export` | Analyst, Administrator | `{ report_type, filters, format }` | export job id | asynchronous artifact generation |
| GET | `/api/v1/feature-flags` | Administrator, Analyst(read-only) | filters | feature flag list | includes rollout/variant config |
| POST | `/api/v1/feature-flags` | Administrator | `{ key, enabled, variants?, allocation? }` | created flag | key uniqueness and allocation validity |
| PATCH | `/api/v1/feature-flags/{key}` | Administrator | `{ enabled?, variants?, allocation? }` | updated flag | audit required |

## 13. Alert Queue / Ops APIs

| METHOD | PATH | Auth | Request expectations | Response expectations | Key validations / status notes |
|---|---|---|---|---|---|
| GET | `/api/v1/ops/alerts` | Administrator | filters/pagination (`state`, `from`, `to`) | alert spool index listing | reads durable on-disk queue index |
| POST | `/api/v1/ops/alerts/{alert_id}/ack` | Administrator | `{ note? }` | ack confirmation | audit required |

## 14. Audit Coverage Expectations (Contractual)

Audit records are mandatory for:
- user create/update/reset-password/role change
- template creation/version creation/activation
- item creation/update/rollback/status transition/publish/archive
- import/export job triggers and terminal outcomes
- feature flag changes
- alert acknowledgements

## 15. Idempotency/Concurrency Notes

- Import/export/standardization trigger endpoints support optional idempotency key header.
- Auto-number generation is transactional and NY-day bounded.
- Rollback source-version eligibility validated against latest 10 historical versions at transaction time.
- Publish endpoint rechecks status and version pointers inside transaction to prevent stale approvals.
