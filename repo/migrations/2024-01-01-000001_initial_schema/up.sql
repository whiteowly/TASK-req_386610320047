-- KnowledgeOps initial schema migration
-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "citext";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- ROLES
-- ============================================================
CREATE TABLE roles (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        VARCHAR     UNIQUE NOT NULL
                            CHECK (name IN ('Administrator', 'Author', 'Reviewer', 'Analyst')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- USERS
-- ============================================================
CREATE TABLE users (
    id                UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    username          CITEXT      UNIQUE NOT NULL,
    password_hash     TEXT        NOT NULL,
    email_encrypted   TEXT,
    phone_encrypted   TEXT,
    role_id           UUID        NOT NULL REFERENCES roles (id),
    active            BOOLEAN     NOT NULL DEFAULT true,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SESSIONS
-- ============================================================
CREATE TABLE sessions (
    id                UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id           UUID        NOT NULL REFERENCES users (id),
    token_hash        VARCHAR     UNIQUE NOT NULL,
    last_activity_at  TIMESTAMPTZ NOT NULL,
    expires_at        TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- CHANNELS
-- ============================================================
CREATE TABLE channels (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        VARCHAR     UNIQUE NOT NULL,
    description TEXT,
    active      BOOLEAN     NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- TAGS
-- ============================================================
CREATE TABLE tags (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        VARCHAR     UNIQUE NOT NULL,   -- stored normalized/lowercase
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- TEMPLATES  (active_version_id added as nullable; FK added after template_versions)
-- ============================================================
CREATE TABLE templates (
    id                UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name              VARCHAR     NOT NULL,
    slug              VARCHAR     UNIQUE NOT NULL,
    description       TEXT,
    channel_scope     UUID        REFERENCES channels (id),
    active_version_id UUID,                         -- FK to template_versions added below
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- TEMPLATE VERSIONS  (immutable – no updated_at)
-- ============================================================
CREATE TABLE template_versions (
    id                   UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    template_id          UUID        NOT NULL REFERENCES templates (id),
    version_number       INTEGER     NOT NULL,
    field_schema         JSONB       NOT NULL,
    constraints_schema   JSONB,
    cross_field_rules    JSONB,
    change_note          TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (template_id, version_number)
);

-- Now add the deferred FK from templates.active_version_id
ALTER TABLE templates
    ADD CONSTRAINT fk_templates_active_version
    FOREIGN KEY (active_version_id) REFERENCES template_versions (id);

-- ============================================================
-- ITEMS  (current_version_id FK added after item_versions)
-- ============================================================
CREATE TABLE items (
    id                           UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    template_id                  UUID        NOT NULL REFERENCES templates (id),
    channel_id                   UUID        NOT NULL REFERENCES channels (id),
    owner_user_id                UUID        NOT NULL REFERENCES users (id),
    auto_number                  VARCHAR     UNIQUE NOT NULL,
    status                       VARCHAR     NOT NULL DEFAULT 'Draft'
                                             CHECK (status IN ('Draft', 'InReview', 'Approved', 'Published', 'Archived')),
    current_version_id           UUID,                  -- FK added after item_versions
    published_at                 TIMESTAMPTZ,
    published_version_id         UUID,
    published_template_version_id UUID,
    entered_in_review_at         TIMESTAMPTZ,
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- ITEM VERSIONS  (immutable – no updated_at)
-- ============================================================
CREATE TABLE item_versions (
    id                          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    item_id                     UUID        NOT NULL REFERENCES items (id),
    version_number              INTEGER     NOT NULL,
    template_version_id         UUID        NOT NULL REFERENCES template_versions (id),
    title                       TEXT        NOT NULL,
    body                        TEXT,
    fields                      JSONB       NOT NULL,
    sensitive_fields_encrypted  TEXT,
    change_note                 TEXT,
    created_by                  UUID        NOT NULL REFERENCES users (id),
    rollback_source_version_id  UUID,               -- self-referential, nullable
    search_vector               TSVECTOR,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (item_id, version_number)
);

-- Deferred FK: item_versions.rollback_source_version_id -> item_versions
ALTER TABLE item_versions
    ADD CONSTRAINT fk_item_versions_rollback_source
    FOREIGN KEY (rollback_source_version_id) REFERENCES item_versions (id);

-- Deferred FKs on items for version columns
ALTER TABLE items
    ADD CONSTRAINT fk_items_current_version
    FOREIGN KEY (current_version_id) REFERENCES item_versions (id);

ALTER TABLE items
    ADD CONSTRAINT fk_items_published_version
    FOREIGN KEY (published_version_id) REFERENCES item_versions (id);

ALTER TABLE items
    ADD CONSTRAINT fk_items_published_template_version
    FOREIGN KEY (published_template_version_id) REFERENCES template_versions (id);

-- ============================================================
-- ITEM VERSION TAGS
-- ============================================================
CREATE TABLE item_version_tags (
    id               UUID    PRIMARY KEY DEFAULT uuid_generate_v4(),
    item_version_id  UUID    NOT NULL REFERENCES item_versions (id),
    tag_id           UUID    NOT NULL REFERENCES tags (id),
    UNIQUE (item_version_id, tag_id)
);

-- ============================================================
-- AUDITS  (7-year retention; actor_id nullable for system actions)
-- ============================================================
CREATE TABLE audits (
    id            UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    actor_id      UUID,
    actor_username VARCHAR,
    action        VARCHAR     NOT NULL,
    object_type   VARCHAR     NOT NULL,
    object_id     UUID,
    before_state  JSONB,
    after_state   JSONB,
    reason        TEXT,
    request_id    VARCHAR,
    ip_address    VARCHAR,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SEARCHES
-- ============================================================
CREATE TABLE searches (
    id               UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id          UUID        REFERENCES users (id),
    query_raw        TEXT        NOT NULL,
    query_normalized TEXT        NOT NULL,
    channel_filter   UUID,
    tag_filter       TEXT,
    result_count     INTEGER,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SEARCH HISTORY  (per-user max 200 enforced at application layer)
-- ============================================================
CREATE TABLE search_history (
    id         UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID        NOT NULL REFERENCES users (id),
    query      TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SEARCH TRENDING DAILY
-- ============================================================
CREATE TABLE search_trending_daily (
    id            UUID    PRIMARY KEY DEFAULT uuid_generate_v4(),
    term          VARCHAR NOT NULL,
    frequency     INTEGER NOT NULL,
    computed_date DATE    NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (term, computed_date)
);

-- ============================================================
-- IMPORTS
-- ============================================================
CREATE TABLE imports (
    id                   UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id              UUID        NOT NULL REFERENCES users (id),
    template_version_id  UUID        NOT NULL REFERENCES template_versions (id),
    channel_id           UUID        NOT NULL REFERENCES channels (id),
    filename             VARCHAR     NOT NULL,
    file_size            BIGINT,
    status               VARCHAR     NOT NULL DEFAULT 'queued',
    total_rows           INTEGER,
    accepted_rows        INTEGER,
    rejected_rows        INTEGER,
    options              JSONB,
    idempotency_key      VARCHAR     UNIQUE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- IMPORT ROWS
-- ============================================================
CREATE TABLE import_rows (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    import_id   UUID        NOT NULL REFERENCES imports (id),
    row_number  INTEGER     NOT NULL,
    status      VARCHAR     NOT NULL,
    item_id     UUID,
    errors      JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- EXPORTS
-- ============================================================
CREATE TABLE exports (
    id                  UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID        NOT NULL REFERENCES users (id),
    scope_filters       JSONB       NOT NULL,
    format              VARCHAR     NOT NULL,
    include_explanations BOOLEAN    NOT NULL DEFAULT false,
    mask_sensitive      BOOLEAN     NOT NULL DEFAULT false,
    status              VARCHAR     NOT NULL DEFAULT 'queued',
    artifact_path       TEXT,
    artifact_checksum   VARCHAR,
    artifact_size       BIGINT,
    idempotency_key     VARCHAR     UNIQUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- EXPORT ARTIFACTS
-- ============================================================
CREATE TABLE export_artifacts (
    id                     UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    export_id              UUID        NOT NULL REFERENCES exports (id),
    file_path              TEXT        NOT NULL,
    checksum               VARCHAR     NOT NULL,
    size_bytes             BIGINT      NOT NULL,
    masking_applied        BOOLEAN     NOT NULL DEFAULT false,
    explanations_included  BOOLEAN     NOT NULL DEFAULT false,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- LOGIN ATTEMPTS
-- ============================================================
CREATE TABLE login_attempts (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    username    VARCHAR     NOT NULL,
    ip_address  VARCHAR,
    success     BOOLEAN     NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- CAPTCHA CHALLENGES
-- ============================================================
CREATE TABLE captcha_challenges (
    id               UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    username         VARCHAR     NOT NULL,
    challenge_type   VARCHAR     NOT NULL DEFAULT 'arithmetic',
    challenge_prompt TEXT        NOT NULL,
    expected_answer  VARCHAR     NOT NULL,
    expires_at       TIMESTAMPTZ NOT NULL,
    used             BOOLEAN     NOT NULL DEFAULT false,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- DAILY COUNTERS
-- ============================================================
CREATE TABLE daily_counters (
    id             UUID    PRIMARY KEY DEFAULT uuid_generate_v4(),
    counter_date   DATE    UNIQUE NOT NULL,
    last_sequence  INTEGER NOT NULL DEFAULT 0,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SCHEMA MAPPINGS
-- ============================================================
CREATE TABLE schema_mappings (
    id           UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name         VARCHAR     UNIQUE NOT NULL,
    source_scope TEXT,
    description  TEXT,
    created_by   UUID        NOT NULL REFERENCES users (id),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SCHEMA MAPPING VERSIONS  (immutable – no updated_at)
-- ============================================================
CREATE TABLE schema_mapping_versions (
    id                  UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    mapping_id          UUID        NOT NULL REFERENCES schema_mappings (id),
    version_number      INTEGER     NOT NULL,
    mapping_rules       JSONB       NOT NULL,
    explicit_defaults   JSONB,
    unit_rules          JSONB,
    timezone_rules      JSONB,
    fingerprint_keys    JSONB,
    pii_fields          JSONB,
    change_note         TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (mapping_id, version_number)
);

-- ============================================================
-- STANDARDIZATION JOBS
-- ============================================================
CREATE TABLE standardization_jobs (
    id                  UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    mapping_version_id  UUID        NOT NULL REFERENCES schema_mapping_versions (id),
    source_filters      JSONB,
    run_label           VARCHAR,
    status              VARCHAR     NOT NULL DEFAULT 'queued',
    total_records       INTEGER,
    processed_records   INTEGER,
    failed_records      INTEGER,
    retry_count         INTEGER     NOT NULL DEFAULT 0,
    error_info          TEXT,
    idempotency_key     VARCHAR     UNIQUE,
    started_at          TIMESTAMPTZ,
    completed_at        TIMESTAMPTZ,
    created_by          UUID        NOT NULL REFERENCES users (id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- STANDARDIZED MODELS  (immutable – no updated_at)
-- ============================================================
CREATE TABLE standardized_models (
    id                  UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id              UUID        NOT NULL REFERENCES standardization_jobs (id),
    mapping_version_id  UUID        NOT NULL REFERENCES schema_mapping_versions (id),
    version_number      INTEGER     NOT NULL,
    source_window       JSONB,
    quality_stats       JSONB,
    record_count        INTEGER     NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- STANDARDIZED RECORDS  (immutable – no updated_at)
-- ============================================================
CREATE TABLE standardized_records (
    id                       UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    model_id                 UUID        NOT NULL REFERENCES standardized_models (id),
    source_item_id           UUID        REFERENCES items (id),
    fingerprint              VARCHAR     NOT NULL,
    raw_values               JSONB       NOT NULL,
    standardized_values      JSONB       NOT NULL,
    transformations_applied  JSONB,
    outlier_flags            JSONB,
    is_duplicate             BOOLEAN     NOT NULL DEFAULT false,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- METRICS SNAPSHOTS
-- ============================================================
CREATE TABLE metrics_snapshots (
    id             UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    snapshot_type  VARCHAR     NOT NULL,
    time_range     JSONB       NOT NULL,
    dimensions     JSONB,
    metrics        JSONB       NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- EVENTS
-- ============================================================
CREATE TABLE events (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type  VARCHAR     NOT NULL,
    actor_id    UUID,
    payload     JSONB       NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- FEATURE FLAGS
-- ============================================================
CREATE TABLE feature_flags (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    key         VARCHAR     UNIQUE NOT NULL,
    enabled     BOOLEAN     NOT NULL DEFAULT false,
    variants    JSONB,
    allocation  JSONB,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- RATE LIMITS
-- ============================================================
CREATE TABLE rate_limits (
    id            UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id       UUID        REFERENCES users (id),
    ip_address    VARCHAR,
    window_start  TIMESTAMPTZ NOT NULL,
    request_count INTEGER     NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- INDEXES
-- ============================================================

-- items: common query columns
CREATE INDEX idx_items_auto_number   ON items (auto_number);
CREATE INDEX idx_items_status        ON items (status);
CREATE INDEX idx_items_channel_id    ON items (channel_id);
CREATE INDEX idx_items_published_at  ON items (published_at);

-- item_versions: lookup by item + descending version
CREATE INDEX idx_item_versions_item_version
    ON item_versions (item_id, version_number DESC);

-- GIN full-text search on item_versions.search_vector
CREATE INDEX idx_item_versions_search_vector
    ON item_versions USING GIN (search_vector);

-- audits: object-centric queries + time range
CREATE INDEX idx_audits_object
    ON audits (object_type, object_id, created_at);

-- sessions: fast token lookup
CREATE INDEX idx_sessions_token_hash
    ON sessions (token_hash);

-- searches: time-based queries
CREATE INDEX idx_searches_created_at
    ON searches (created_at);

-- search_history: per-user history queries
CREATE INDEX idx_search_history_user_created
    ON search_history (user_id, created_at DESC);

-- login_attempts: brute-force detection
CREATE INDEX idx_login_attempts_username_created
    ON login_attempts (username, created_at);

-- rate_limits: window queries
CREATE INDEX idx_rate_limits_user_window
    ON rate_limits (user_id, window_start);

-- ============================================================
-- TRIGGER: auto-update search_vector on item_versions INSERT
-- ============================================================
CREATE OR REPLACE FUNCTION trg_item_versions_search_vector()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.body,  '')), 'B');
    RETURN NEW;
END;
$$;

CREATE TRIGGER trig_item_versions_search_vector_insert
BEFORE INSERT ON item_versions
FOR EACH ROW
EXECUTE FUNCTION trg_item_versions_search_vector();
