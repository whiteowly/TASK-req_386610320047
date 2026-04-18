# Audit Report 1 - Fix Check (Static)

Scope: static verification only (no runtime/test execution).

## Summary
- 6 issues reviewed
- Fixed: 6
- Partial: 0
- Unfixed: 0

## Detailed Status

1) Import template API lacked real XLSX output  
**Status:** Fixed  
**Evidence:** `src/services/imports.rs:557`, `src/services/imports.rs:575`, `src/services/imports.rs:589`, `src/services/imports.rs:592`  
**Notes:** `generate_import_template` now has explicit `xlsx` branch, returns XLSX MIME type, and rejects unsupported formats.

2) `events.updated_at` missing from required data-model contract  
**Status:** Fixed  
**Evidence:** `migrations/2024-01-03-000001_add_events_updated_at/up.sql:2`, `src/schema.rs:62`, `src/schema.rs:69`, `src/models.rs:306`, `src/models.rs:313`, `src/services/analytics.rs:61`  
**Notes:** Migration adds `events.updated_at`; schema/model include it; event insert sets `updated_at`.

3) Slow-query logging requirement not implemented for DB operations  
**Status:** Fixed (static evidence)  
**Evidence:** helper `src/db_instrumentation.rs:8`; threshold usage and timed DB ops in: `src/services/auth.rs:111`, `src/services/auth.rs:129`, `src/services/items.rs:97`, `src/services/items.rs:247`, `src/services/search.rs:77`, `src/services/search.rs:182`, `src/services/imports.rs:62`, `src/services/exports.rs:43`, `src/services/exports.rs:122`, `src/services/exports.rs:201`, `src/services/analytics.rs:47`, `src/services/analytics.rs:78`, `src/services/analytics.rs:93`  
**Notes:** Config-driven threshold is now applied across major DB-heavy service paths.

4) Audit logger panic on DB connection failure  
**Status:** Fixed  
**Evidence:** `src/audit.rs:29`, `src/audit.rs:32`, `src/audit.rs:33`  
**Notes:** Panic path removed; function now logs and returns safely when pool connection fails.

5) `list_events` total count ignored active filters  
**Status:** Fixed  
**Evidence:** `src/services/analytics.rs:72`, `src/services/analytics.rs:73`, `src/services/analytics.rs:75`, `src/services/analytics.rs:76`, `src/services/analytics.rs:78`; handler query wiring `src/api/handlers/events.rs:10`, `src/api/handlers/events.rs:13`, `src/api/handlers/events.rs:26`, `src/api/handlers/events.rs:27`  
**Notes:** Count query now mirrors event-type filter.

6) High-risk integration tests were shallow (status-only)  
**Status:** Fixed  
**Evidence:**
- Relevance vs newest ordering assertions: `tests/api_integration_tests.sh:945`, `tests/api_integration_tests.sh:973`, `tests/api_integration_tests.sh:987`, `tests/api_integration_tests.sh:1001`
- CAPTCHA cross-user misuse with computed correct answer: `tests/api_integration_tests.sh:1034`, `tests/api_integration_tests.sh:1041`, `tests/api_integration_tests.sh:1049`, `tests/api_integration_tests.sh:1053`
- Rate limit burst + `RATE_LIMITED` check: `tests/api_integration_tests.sh:1059`, `tests/api_integration_tests.sh:1063`, `tests/api_integration_tests.sh:1073`, `tests/api_integration_tests.sh:1075`
- Import template XLSX response check: `tests/api_integration_tests.sh:1087`, `tests/api_integration_tests.sh:1092`, `tests/api_integration_tests.sh:1101`
- Filtered events total check: `tests/api_integration_tests.sh:1124`, `tests/api_integration_tests.sh:1127`, `tests/api_integration_tests.sh:1129`
- Search query safety checks: `tests/api_integration_tests.sh:1138`, `tests/api_integration_tests.sh:1144`, `tests/api_integration_tests.sh:1158`, `tests/api_integration_tests.sh:1170`

## Final Check Conclusion
All six previously tracked issues are now fixed based on static code evidence. Runtime behavior and pass/fail execution of tests remain manual verification steps.
