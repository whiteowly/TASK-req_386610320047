# Audit Report 2 - Fix Check (Static)

## Verdict

- Overall: **All 6 issues from `.tmp/audit_report-2.md` appear fixed by static evidence.**
- Boundary: This is static-only validation; runtime execution is still **Manual Verification Required**.

## Issue-by-Issue Status

1. **High - Full-text search did not include tags**
   - **Status:** Fixed (static)
   - **What changed:** Added migration to include tag text in `item_versions.search_vector` and keep it updated on tag insert/delete.
   - **Evidence:** `migrations/2024-01-04-000001_search_vector_tags/up.sql:6`, `migrations/2024-01-04-000001_search_vector_tags/up.sql:20`, `migrations/2024-01-04-000001_search_vector_tags/up.sql:53`
   - **Test evidence:** Tag-only search case added.
   - **Evidence:** `tests/api_integration_tests.sh:1217`, `tests/api_integration_tests.sh:1237`

2. **High - Normalized title+channel duplicate check was incorrect**
   - **Status:** Fixed (static)
   - **What changed:** Duplicate query now normalizes stored titles before comparing to normalized input.
   - **Evidence:** `src/services/imports.rs:246`, `src/services/imports.rs:261`
   - **Test evidence:** Normalized duplicate import rejection scenario added.
   - **Evidence:** `tests/api_integration_tests.sh:1251`, `tests/api_integration_tests.sh:1278`

3. **High - Exception-to-alert-queue behavior incomplete**
   - **Status:** Fixed (static)
   - **What changed (code):**
     - Scheduler writes `JOB_FAILURE` alerts across all major failure branches.
     - 5xx response path writes rate-limited `INTERNAL_ERROR` alerts.
   - **Evidence:** `src/jobs/scheduler.rs:20`, `src/jobs/scheduler.rs:34`, `src/jobs/scheduler.rs:48`, `src/jobs/scheduler.rs:62`, `src/jobs/scheduler.rs:75`, `src/errors.rs:183`, `src/errors.rs:193`
   - **What changed (deterministic verification):** Added admin-only diagnostic routes and explicit spool-content assertions.
   - **Evidence:** `src/api/mod.rs:97`, `src/api/mod.rs:98`, `src/api/handlers/ops.rs:27`, `src/api/handlers/ops.rs:38`, `tests/api_integration_tests.sh:1291`, `tests/api_integration_tests.sh:1329`, `tests/api_integration_tests.sh:1350`

4. **High - Analytics APIs did not implement custom filters**
   - **Status:** Fixed (static)
   - **What changed:** Added `AnalyticsFilter`, validation, and filter application in KPI and operational queries.
   - **Evidence:** `src/services/analytics.rs:25`, `src/services/analytics.rs:37`, `src/services/analytics.rs:172`, `src/services/analytics.rs:203`
   - **Handler wiring evidence:** `src/api/handlers/analytics.rs:8`, `src/api/handlers/analytics.rs:15`
   - **Test evidence:** channel/status/date filter checks and invalid status 400 case.
   - **Evidence:** `tests/api_integration_tests.sh:1324`, `tests/api_integration_tests.sh:1352`, `tests/api_integration_tests.sh:1357`

5. **Medium - Export generation assumed non-null body**
   - **Status:** Fixed (static)
   - **What changed:** Export query now reads nullable body and safely falls back with `unwrap_or("")`.
   - **Evidence:** `src/services/exports.rs:122`, `src/services/exports.rs:124`, `src/services/exports.rs:168`
   - **Test evidence:** explicit export flow including item with null body.
   - **Evidence:** `tests/api_integration_tests.sh:1376`, `tests/api_integration_tests.sh:1382`, `tests/api_integration_tests.sh:1390`

6. **Medium - Auto-number duplicate precheck in import flow was undercut**
   - **Status:** Fixed (static)
   - **What changed:** Import template now includes `auto_number`, and import validation checks duplicate auto-number values.
   - **Evidence:** `src/services/imports.rs:232`, `src/services/imports.rs:579`
   - **Test evidence:** template column check + duplicate auto-number rejection check.
   - **Evidence:** `tests/api_integration_tests.sh:1408`, `tests/api_integration_tests.sh:1412`, `tests/api_integration_tests.sh:1428`, `tests/api_integration_tests.sh:1434`

## Documentation Alignment Check

- README now documents these updated behaviors:
  - alert spool coverage details,
  - analytics filter parameters,
  - import template/duplicate policy.
- **Evidence:** `README.md:250`, `README.md:266`, `README.md:282`

## Remaining Boundary

- Static audit can confirm code/test presence and logical alignment, but cannot confirm runtime execution in this pass.
- **Manual Verification Required:** run integration suite to verify deterministic alert triggers and spool assertions pass in environment.
