# Test Coverage Audit

## Scope and Method

- Audit mode: static inspection only (no code/test/script/container execution).
- Endpoint source of truth: `src/api/mod.rs:8` to `src/api/mod.rs:98`.
- API test source: `tests/api_integration_tests.sh:1` to `tests/api_integration_tests.sh:1541`.
- Unit test source: `tests/unit_tests.rs:1` to `tests/unit_tests.rs:499`.
- README source: `README.md`.

## Project Type Detection

- Declared in README: `backend` (`README.md:3` to `README.md:5`).
- Inferred type: backend (consistent with code structure and test surfaces).

## Backend Endpoint Inventory

Resolved base prefix: `/api/v1` from `web::scope("/api/v1")` at `src/api/mod.rs:10`.

1. GET `/api/v1/health`
2. POST `/api/v1/auth/login`
3. POST `/api/v1/auth/captcha/challenge`
4. POST `/api/v1/auth/logout`
5. GET `/api/v1/auth/me`
6. GET `/api/v1/users`
7. POST `/api/v1/users`
8. PATCH `/api/v1/users/{user_id}`
9. POST `/api/v1/users/{user_id}/reset-password`
10. GET `/api/v1/channels`
11. POST `/api/v1/channels`
12. PATCH `/api/v1/channels/{channel_id}`
13. GET `/api/v1/tags`
14. POST `/api/v1/tags`
15. POST `/api/v1/templates`
16. GET `/api/v1/templates`
17. GET `/api/v1/templates/{template_id}`
18. POST `/api/v1/templates/{template_id}/versions`
19. GET `/api/v1/templates/{template_id}/versions`
20. GET `/api/v1/templates/{template_id}/versions/{version_id}`
21. POST `/api/v1/templates/{template_id}/versions/{version_id}/activate`
22. POST `/api/v1/items`
23. GET `/api/v1/items`
24. GET `/api/v1/items/{item_id}`
25. PATCH `/api/v1/items/{item_id}`
26. GET `/api/v1/items/{item_id}/versions`
27. GET `/api/v1/items/{item_id}/versions/{version_id}`
28. POST `/api/v1/items/{item_id}/rollback`
29. POST `/api/v1/items/{item_id}/transitions`
30. POST `/api/v1/items/{item_id}/publish`
31. GET `/api/v1/search`
32. GET `/api/v1/search/suggestions`
33. GET `/api/v1/search/trending`
34. GET `/api/v1/search/history`
35. DELETE `/api/v1/search/history`
36. GET `/api/v1/imports/templates/{template_version_id}`
37. POST `/api/v1/imports`
38. GET `/api/v1/imports`
39. GET `/api/v1/imports/{import_id}`
40. GET `/api/v1/imports/{import_id}/errors`
41. GET `/api/v1/imports/{import_id}/result`
42. POST `/api/v1/exports`
43. GET `/api/v1/exports`
44. GET `/api/v1/exports/{export_id}`
45. GET `/api/v1/exports/{export_id}/download`
46. POST `/api/v1/schema-mappings`
47. GET `/api/v1/schema-mappings`
48. GET `/api/v1/schema-mappings/{mapping_id}`
49. POST `/api/v1/schema-mappings/{mapping_id}/versions`
50. GET `/api/v1/schema-mappings/{mapping_id}/versions`
51. POST `/api/v1/standardization/jobs`
52. GET `/api/v1/standardization/jobs`
53. GET `/api/v1/standardization/jobs/{job_id}`
54. GET `/api/v1/standardization/models`
55. GET `/api/v1/standardization/models/{model_id}`
56. GET `/api/v1/standardization/models/{model_id}/records`
57. POST `/api/v1/events`
58. GET `/api/v1/events`
59. POST `/api/v1/metrics/snapshots`
60. GET `/api/v1/metrics/snapshots`
61. GET `/api/v1/analytics/kpis`
62. GET `/api/v1/analytics/operational`
63. POST `/api/v1/analytics/export`
64. GET `/api/v1/feature-flags`
65. POST `/api/v1/feature-flags`
66. PATCH `/api/v1/feature-flags/{key}`
67. GET `/api/v1/ops/alerts`
68. POST `/api/v1/ops/alerts/{alert_id}/ack`
69. POST `/api/v1/ops/diagnostic/error`
70. POST `/api/v1/ops/diagnostic/job-failure`

## API Test Mapping Table

| Endpoint | Covered | Test type | Test files | Evidence |
|---|---|---|---|---|
| `GET /api/v1/health` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:129` |
| `POST /api/v1/auth/login` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:142` |
| `POST /api/v1/auth/captcha/challenge` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:626` |
| `POST /api/v1/auth/logout` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:189` |
| `GET /api/v1/auth/me` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:160` |
| `GET /api/v1/users` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:205` |
| `POST /api/v1/users` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:600` |
| `PATCH /api/v1/users/{user_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:611` |
| `POST /api/v1/users/{user_id}/reset-password` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:674` |
| `GET /api/v1/channels` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:261` |
| `POST /api/v1/channels` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:245` |
| `PATCH /api/v1/channels/{channel_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:688` |
| `GET /api/v1/tags` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:694` |
| `POST /api/v1/tags` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:266` |
| `POST /api/v1/templates` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:280` |
| `GET /api/v1/templates` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:291` |
| `GET /api/v1/templates/{template_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:298` |
| `POST /api/v1/templates/{template_id}/versions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:306` |
| `GET /api/v1/templates/{template_id}/versions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:320` |
| `GET /api/v1/templates/{template_id}/versions/{version_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:326` |
| `POST /api/v1/templates/{template_id}/versions/{version_id}/activate` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:332` |
| `POST /api/v1/items` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:348` |
| `GET /api/v1/items` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:700` |
| `GET /api/v1/items/{item_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:429` |
| `PATCH /api/v1/items/{item_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:375` |
| `GET /api/v1/items/{item_id}/versions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:381` |
| `GET /api/v1/items/{item_id}/versions/{version_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:713` |
| `POST /api/v1/items/{item_id}/rollback` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:470` |
| `POST /api/v1/items/{item_id}/transitions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:389` |
| `POST /api/v1/items/{item_id}/publish` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:432` |
| `GET /api/v1/search` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:482` |
| `GET /api/v1/search/suggestions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:486` |
| `GET /api/v1/search/trending` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:490` |
| `GET /api/v1/search/history` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:494` |
| `DELETE /api/v1/search/history` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:498` |
| `GET /api/v1/imports/templates/{template_version_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:638` |
| `POST /api/v1/imports` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:720` |
| `GET /api/v1/imports` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:643` |
| `GET /api/v1/imports/{import_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:733` |
| `GET /api/v1/imports/{import_id}/errors` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:739` |
| `GET /api/v1/imports/{import_id}/result` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:744` |
| `POST /api/v1/exports` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:220` |
| `GET /api/v1/exports` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:658` |
| `GET /api/v1/exports/{export_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:662` |
| `GET /api/v1/exports/{export_id}/download` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:750` |
| `POST /api/v1/schema-mappings` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:553` |
| `GET /api/v1/schema-mappings` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:563` |
| `GET /api/v1/schema-mappings/{mapping_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:755` |
| `POST /api/v1/schema-mappings/{mapping_id}/versions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:558` |
| `GET /api/v1/schema-mappings/{mapping_id}/versions` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:761` |
| `POST /api/v1/standardization/jobs` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:567` |
| `GET /api/v1/standardization/jobs` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:571` |
| `GET /api/v1/standardization/jobs/{job_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:768` |
| `GET /api/v1/standardization/models` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:575` |
| `GET /api/v1/standardization/models/{model_id}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:779` |
| `GET /api/v1/standardization/models/{model_id}/records` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:784` |
| `POST /api/v1/events` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:517` |
| `GET /api/v1/events` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:521` |
| `POST /api/v1/metrics/snapshots` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:525` |
| `GET /api/v1/metrics/snapshots` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:529` |
| `GET /api/v1/analytics/kpis` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:508` |
| `GET /api/v1/analytics/operational` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:513` |
| `POST /api/v1/analytics/export` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:801` |
| `GET /api/v1/feature-flags` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:230` |
| `POST /api/v1/feature-flags` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:235` |
| `PATCH /api/v1/feature-flags/{key}` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:543` |
| `GET /api/v1/ops/alerts` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:585` |
| `POST /api/v1/ops/alerts/{alert_id}/ack` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:810` |
| `POST /api/v1/ops/diagnostic/error` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:1321` |
| `POST /api/v1/ops/diagnostic/job-failure` | yes | true no-mock HTTP | `tests/api_integration_tests.sh` | `tests/api_integration_tests.sh:1329` |

## API Test Classification

1. True no-mock HTTP
   - `tests/api_integration_tests.sh` (real HTTP helper calls and raw `curl` requests, e.g., `tests/api_integration_tests.sh:64` to `tests/api_integration_tests.sh:112`, `tests/api_integration_tests.sh:720`).
2. HTTP with mocking
   - None found by static scan.
3. Non-HTTP (unit/integration without HTTP)
   - `tests/unit_tests.rs`.

## Mock Detection Results

- `jest.mock`, `vi.mock`, `sinon.stub`: not found under `tests/`.
- Dependency-injection override patterns in API tests: not found.
- Direct controller/service call bypassing HTTP in API integration tests: not found.
- Note: fake alert ID fallback in `tests/api_integration_tests.sh:814` is test data variation, not mocking.

## Coverage Summary

- Total endpoints: **70**
- Endpoints with HTTP tests: **70**
- Endpoints with TRUE no-mock tests: **70**
- HTTP coverage %: **100.00%**
- True API coverage %: **100.00%**

## Unit Test Analysis

### Backend Unit Tests

- Unit test file: `tests/unit_tests.rs`.
- Modules covered:
  - utility/domain-heavy coverage: crypto, search, import/export helpers, audit redaction, pagination, standardization, db instrumentation, analytics filter validation.
  - evidence examples: `tests/unit_tests.rs:15`, `tests/unit_tests.rs:114`, `tests/unit_tests.rs:159`, `tests/unit_tests.rs:255`, `tests/unit_tests.rs:302`, `tests/unit_tests.rs:426`.
- Important backend modules not directly unit-tested:
  - API handlers: `src/api/handlers/*`
  - middleware: `src/api/middleware/*`
  - most service modules under `src/services/*` beyond `analytics::AnalyticsFilter`
  - repository/persistence modules as isolated units

### Frontend Unit Tests (Strict Requirement)

- Project type is backend, not fullstack/web.
- Frontend test files: **NONE**.
- Frontend framework/tools detected: **NONE**.
- Frontend components/modules covered: **NONE**.
- Important frontend components/modules not tested: not applicable (no frontend layer detected in repository).
- Mandatory verdict: **Frontend unit tests: MISSING**.
- Critical gap rule applicability: not triggered (project type is backend).

### Cross-Layer Observation

- No frontend layer present; backend-only testing profile is expected.

## API Observability Check

- Positive: tests clearly name method/path and assert request outcomes across the suite.
- Positive: many tests include body/query/param inputs and response-content assertions (examples at `tests/api_integration_tests.sh:294` to `tests/api_integration_tests.sh:303`, `tests/api_integration_tests.sh:1379` to `tests/api_integration_tests.sh:1412`).
- Weakness: some endpoints remain status-dominant checks with lighter schema assertions.

## Test Quality & Sufficiency (Tests Check)

- Success paths: strong and broad across endpoint families.
- Failure/negative paths: strong (401/403/404/409/429/400 and transition/validation failures).
- Edge cases: present (CAPTCHA misuse binding, rate limiting bursts, query safety, import/export edge cases).
- Auth/permissions and object-level authorization: strong coverage.
- Integration boundaries: real HTTP layer exercised throughout API suite.
- `run_tests.sh` model: Docker-contained broad test orchestration (`run_tests.sh:66` to `run_tests.sh:76`, `run_tests.sh:124` to `run_tests.sh:127`), no manual host dependency installation required.

## End-to-End Expectations

- Fullstack FE↔BE E2E expectation: not applicable (backend project type).
- Backend compensating evidence: complete API HTTP coverage plus utility-focused unit tests.

## Test Coverage Score (0–100)

**96/100**

## Score Rationale

- + Full endpoint-level HTTP coverage.
- + Full true no-mock API coverage by static evidence.
- + Strong negative-path and RBAC/object-scope checks.
- - Unit tests are not balanced across all backend layers (service/middleware/handler units are limited).
- - A subset of API checks remains shallow (status-only style).

## Key Gaps

1. Limited direct unit tests for middleware and service orchestration layers.
2. Some endpoint validations could be stricter on response contract fields beyond status.

## Confidence & Assumptions

- Confidence: **High**.
- Assumptions:
  - Mapping is based strictly on repository-visible static evidence.
  - No hidden/private test suites outside this repository.
  - “True no-mock” status is inferred from source patterns; no runtime instrumentation was executed.

---

# README Audit

## README Location

- Required README present at `README.md`.

## Hard Gates

### Formatting

- PASS: clean markdown, clear sections/tables/code blocks.

### Startup Instructions

- PASS (backend/fullstack requirement): includes `docker compose up --build` (`README.md:37`) and legacy compatibility `docker-compose up` (`README.md:43`).

### Access Method

- PASS: backend URL/port and access paths documented (`README.md:50` to `README.md:71`).

### Verification Method

- PASS: explicit API verification commands and broad command `./run_tests.sh` (`README.md:52` to `README.md:60`, `README.md:101` to `README.md:103`).

### Environment Rules (Docker-contained, no runtime installs)

- PASS: no README startup dependency on `npm install`, `pip install`, `apt-get`, manual runtime package installation, or manual DB setup.

### Demo Credentials (Conditional)

- PASS: authentication exists and all role demo credentials are documented (`README.md:127` to `README.md:132`).
- Additional quick seeded demo credential is explicitly provided (`README.md:77` to `README.md:82`).

## Engineering Quality

- Tech stack clarity: strong (`README.md:21` to `README.md:30`).
- Architecture explanation: strong (`README.md:200` onward).
- Security/role/workflow clarity: strong (`README.md:136` onward and security notes later in file).
- Testing/operations clarity: strong and now aligned with 70-endpoint inventory (`README.md:119`, `README.md:192`).

## High Priority Issues

- None.

## Medium Priority Issues

- None.

## Low Priority Issues

- None material.

## Hard Gate Failures

- None.

## README Verdict (PASS / PARTIAL PASS / FAIL)

**PASS**

---

## Final Verdicts

1. **Test Coverage Audit:** PASS
2. **README Audit:** PASS
