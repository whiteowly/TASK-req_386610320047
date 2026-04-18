-- KnowledgeOps initial schema rollback
-- Drop tables in reverse dependency order

-- Drop trigger and function first
DROP TRIGGER IF EXISTS trig_item_versions_search_vector_insert ON item_versions;
DROP FUNCTION IF EXISTS trg_item_versions_search_vector();

-- Leaf / analytics tables (no dependents)
DROP TABLE IF EXISTS rate_limits;
DROP TABLE IF EXISTS feature_flags;
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS metrics_snapshots;
DROP TABLE IF EXISTS standardized_records;
DROP TABLE IF EXISTS standardized_models;
DROP TABLE IF EXISTS standardization_jobs;
DROP TABLE IF EXISTS schema_mapping_versions;
DROP TABLE IF EXISTS schema_mappings;
DROP TABLE IF EXISTS daily_counters;
DROP TABLE IF EXISTS captcha_challenges;
DROP TABLE IF EXISTS login_attempts;
DROP TABLE IF EXISTS export_artifacts;
DROP TABLE IF EXISTS exports;
DROP TABLE IF EXISTS import_rows;
DROP TABLE IF EXISTS imports;
DROP TABLE IF EXISTS search_trending_daily;
DROP TABLE IF EXISTS search_history;
DROP TABLE IF EXISTS searches;
DROP TABLE IF EXISTS audits;
DROP TABLE IF EXISTS item_version_tags;

-- Drop deferred FK constraints before dropping tables that reference each other
ALTER TABLE IF EXISTS items DROP CONSTRAINT IF EXISTS fk_items_published_template_version;
ALTER TABLE IF EXISTS items DROP CONSTRAINT IF EXISTS fk_items_published_version;
ALTER TABLE IF EXISTS items DROP CONSTRAINT IF EXISTS fk_items_current_version;
ALTER TABLE IF EXISTS item_versions DROP CONSTRAINT IF EXISTS fk_item_versions_rollback_source;

DROP TABLE IF EXISTS item_versions;
DROP TABLE IF EXISTS items;

-- Drop deferred FK on templates before dropping template_versions
ALTER TABLE IF EXISTS templates DROP CONSTRAINT IF EXISTS fk_templates_active_version;

DROP TABLE IF EXISTS template_versions;
DROP TABLE IF EXISTS templates;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS channels;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS roles;

-- Drop extensions
DROP EXTENSION IF EXISTS "pgcrypto";
DROP EXTENSION IF EXISTS "citext";
DROP EXTENSION IF EXISTS "uuid-ossp";
