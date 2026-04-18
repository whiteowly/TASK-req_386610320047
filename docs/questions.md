# Questions

This file records only prompt items that needed interpretation because they were unclear, incomplete, or materially ambiguous.

### 1. API Style and Versioning
- Question: Should the backend contract use versioned REST, RPC-style endpoints, or something else?
- My Understanding: The prompt specified backend APIs but did not state the contract style or versioning strategy. We needed a clear convention before planning could begin.
- Solution: Expose JSON REST APIs under a stable `/api/v1` namespace. This preserves the required offline API shape, keeps the surface reviewable, and leaves room for future evolution.

### 2. Session Transport Model
- Question: Should sessions use cookies, JWTs, or opaque tokens?
- My Understanding: The prompt required login, RBAC, and 12-hour inactivity expiration but did not specify the session transport mechanism. The prompt already requires a `sessions` table, which strongly suggests server-managed sessions.
- Solution: Use database-backed opaque session tokens with inactivity enforced via `last_activity_at` sliding expiration and revocation handled server-side.

### 3. First Administrator Bootstrap
- Question: How does the first Administrator account appear in a fresh local deployment?
- My Understanding: The prompt defined roles but gave no bootstrap path. The system must be operable as a single offline Dockerized service, so an internal bootstrap mechanism is necessary.
- Solution: Include a local-only first-run seeding mechanism that creates an Administrator account without relying on external services.

### 4. Deployment Tenancy Scope
- Question: Should the system support multi-tenant organization separation?
- My Understanding: The prompt described internal teams but did not require tenant isolation. Adding multi-tenant layers would invent scope beyond what was asked.
- Solution: Treat the product as a single-organization internal deployment. Do not add tenant scoping, organization isolation, or multi-tenant billing/admin layers unless later evidence requires them.

### 5. Template and Item Version Rollback Semantics
- Question: Does rollback mutate history or create a new head version?
- My Understanding: The prompt required immutable histories and rollback to the last 10 versions but did not define the rollback mechanism. Immutability must be preserved.
- Solution: Implement rollback as creation of a new latest version cloned from one of the previous 10 versions, rather than reactivating or editing an old stored version.

### 6. Publish Target Fidelity
- Question: Must published content preserve both the item version and the template version linkage?
- My Understanding: The prompt said publishing always targets a specific version but did not spell out whether template version context must also be locked. Reproducibility and auditability require both.
- Solution: Treat published content as bound to a specific item version and the template version used when that content version was created or validated.

### 7. Daily Auto-Number Reset Boundary
- Question: Which timezone controls the daily counter reset for `KO-YYYYMMDD-#####`?
- My Understanding: The prompt required a daily counter reset but did not define the timezone boundary. A hidden second timezone rule would create confusion.
- Solution: Reset the counter on the America/New_York day boundary, aligning with the declared timestamp normalization standard.

### 8. In-Review Idle Auto-Revert Job Cadence
- Question: How and when is the 14-day idle review revert rule evaluated?
- My Understanding: The prompt required automatic revert to Draft after 14 idle days in review but did not state the enforcement mechanism. It must work without external schedulers.
- Solution: Run a local asynchronous review-expiry sweep at least daily, reverting any `In Review` item whose tracked review activity has been idle for 14 days.

### 9. Search Suggestion Source
- Question: Are query suggestions global, personalized, or based on content corpus terms?
- My Understanding: The prompt required suggestions derived from the last 30 days but did not define the source. The wording points to search activity rather than content text.
- Solution: Generate suggestions from normalized search queries from the last 30 days.

### 10. Trending Term Computation Basis
- Question: Are trending terms computed from searches, content text, or combined analytics?
- My Understanding: The prompt required daily trending terms but did not define the signal source. Using search behavior keeps it local, explainable, and consistent with the suggestion requirements.
- Solution: Compute daily trending terms from recent normalized search queries with simple stop-word filtering and persisted daily snapshots.

### 11. Search History Cap Behavior
- Question: How should the 200-entry per-user search history cap be enforced?
- My Understanding: The prompt capped history at 200 entries but did not define trim behavior. Preserving newest entries and dropping oldest is the natural fit.
- Solution: On insert, keep the most recent 200 entries per user and delete anything older beyond that cap. The clear operation removes only that user's stored history.

### 12. CAPTCHA Escalation Rule
- Question: What triggers the local CAPTCHA challenge and what form does it take?
- My Understanding: The prompt required a local CAPTCHA after repeated failed logins but defined neither the threshold nor the challenge form. It must stay offline and be straightforward to review.
- Solution: Trigger a locally generated arithmetic CAPTCHA after five failed login attempts within a short rolling window for the same username-plus-client context.

### 13. Export Masking Defaults
- Question: What is the default masking behavior and scope for exports?
- My Understanding: The prompt allowed optional masking and optional explanatory content but did not define defaults. Masking should be opt-in per request while protecting known sensitive patterns when enabled.
- Solution: Support request flags for including explanations and for masking sensitive strings, with masking applied to known patterns (email, phone) plus any fields designated sensitive by governance rules.

### 14. Sensitive Column Encryption Scope
- Question: Which columns are considered sensitive and require encryption at rest?
- My Understanding: The prompt required encrypted at-rest storage for sensitive columns but did not enumerate them. Searchable title/body fields cannot be blindly encrypted without breaking the mandated search surface.
- Solution: Encrypt passwords, stored session secrets, explicitly sensitive template/item field values, and governed export/import payload columns. Searchable title/body remain governed separately.

### 15. Additional Persistence Beyond the Listed Minimum Tables
- Question: Can planning add tables beyond the listed minimum set?
- My Understanding: The prompt listed required tables but also required features (schema-mapped standardization pipeline, job tracking) that need additional persistence. The named tables are a required minimum, not a ceiling.
- Solution: Planning may add supporting tables for schema mappings, job tracking, standardized outputs, or similar prompt-required capabilities beyond the listed minimum set.

### 16. Duplicate Detection Scope for Imports
- Question: Does duplicate detection span all lifecycle states?
- My Understanding: The prompt required duplicate checks by auto-number or normalized title plus channel within 90 days but did not define whether the comparison includes all states. Governance intent requires broad coverage.
- Solution: Pre-import duplicate checks compare against existing items regardless of active non-deleted lifecycle state, using exact auto-number matches and normalized title-plus-channel matches within the 90-day window.

### 17. Role Capability Boundaries
- Question: What is each role's full capability set beyond the explicit approval restriction?
- My Understanding: The prompt named four roles and one specific rule (only Reviewers can approve) but did not fully enumerate each role's wider capabilities. Least privilege around named workflows is the safest default.
- Solution: Administrators manage users, roles, feature flags, and governed operations. Authors create and revise their items/templates. Reviewers control review and approval decisions. Analysts focus on search, exports, analytics, and standardization review.

### 18. Partial-Success Import Transaction Semantics
- Question: Should valid rows commit when some rows fail during import?
- My Understanding: The prompt required partial success and row-level errors but did not explicitly state that valid rows persist independently. The partial-success requirement implies they should.
- Solution: Valid rows are committed, invalid rows are rejected with row-level diagnostics, and the import record stores counts plus a downloadable error report.

### 19. Alert Queue Form
- Question: Is the on-disk alert queue a database table, filesystem spool, or both?
- My Understanding: The prompt required exception alerts in local logs and an on-disk alert queue but did not define the queue form. The explicit "on-disk" wording suggests a filesystem artifact.
- Solution: Use local log files plus a durable on-disk queue/spool artifact for exception alerts. Database metrics/events remain complementary rather than the only alert channel.

### 20. Standardized Model Persistence Shape
- Question: Can planning add persistence for schema mappings and standardized outputs beyond the minimum table list?
- My Understanding: The prompt required a local async cleansing pipeline producing versioned standardized models via schema mappings, but the minimum table list did not explicitly include these. Without additional persistence, the pipeline would be under-modeled.
- Solution: Include supplemental persistence for schema mappings, pipeline jobs, and versioned standardized outputs as needed to implement the required data-governance workflow.
