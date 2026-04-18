use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---- Roles ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Users ----
#[derive(Queryable, Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub email_encrypted: Option<String>,
    pub phone_encrypted: Option<String>,
    pub role_id: Uuid,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Clone, Debug)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub active: bool,
    pub created_at: NaiveDateTime,
}

// ---- Sessions ----
#[derive(Queryable, Clone, Debug)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub last_activity_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Channels ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Tags ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Templates ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Template {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub channel_scope: Option<Uuid>,
    pub active_version_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Template Versions ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct TemplateVersion {
    pub id: Uuid,
    pub template_id: Uuid,
    pub version_number: i32,
    pub field_schema: serde_json::Value,
    pub constraints_schema: Option<serde_json::Value>,
    pub cross_field_rules: Option<serde_json::Value>,
    pub change_note: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Items ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Item {
    pub id: Uuid,
    pub template_id: Uuid,
    pub channel_id: Uuid,
    pub owner_user_id: Uuid,
    pub auto_number: String,
    pub status: String,
    pub current_version_id: Option<Uuid>,
    pub published_at: Option<NaiveDateTime>,
    pub published_version_id: Option<Uuid>,
    pub published_template_version_id: Option<Uuid>,
    pub entered_in_review_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Item Versions ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct ItemVersion {
    pub id: Uuid,
    pub item_id: Uuid,
    pub version_number: i32,
    pub template_version_id: Uuid,
    pub title: String,
    pub body: Option<String>,
    pub fields: serde_json::Value,
    pub sensitive_fields_encrypted: Option<String>,
    pub change_note: Option<String>,
    pub created_by: Uuid,
    pub rollback_source_version_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Clone, Debug)]
pub struct ItemVersionResponse {
    pub id: Uuid,
    pub item_id: Uuid,
    pub version_number: i32,
    pub template_version_id: Uuid,
    pub title: String,
    pub body: Option<String>,
    pub fields: serde_json::Value,
    pub change_note: Option<String>,
    pub created_by: Uuid,
    pub rollback_source_version_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub tags: Vec<String>,
}

// ---- Audit ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Audit {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub action: String,
    pub object_type: String,
    pub object_id: Option<Uuid>,
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub request_id: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Imports ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Import {
    pub id: Uuid,
    pub user_id: Uuid,
    pub template_version_id: Uuid,
    pub channel_id: Uuid,
    pub filename: String,
    pub file_size: Option<i64>,
    pub status: String,
    pub total_rows: Option<i32>,
    pub accepted_rows: Option<i32>,
    pub rejected_rows: Option<i32>,
    pub options: Option<serde_json::Value>,
    pub idempotency_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Serialize, Clone, Debug)]
pub struct ImportRow {
    pub id: Uuid,
    pub import_id: Uuid,
    pub row_number: i32,
    pub status: String,
    pub item_id: Option<Uuid>,
    pub errors: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
}

// ---- Exports ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Export {
    pub id: Uuid,
    pub user_id: Uuid,
    pub scope_filters: serde_json::Value,
    pub format: String,
    pub include_explanations: bool,
    pub mask_sensitive: bool,
    pub status: String,
    pub artifact_path: Option<String>,
    pub artifact_checksum: Option<String>,
    pub artifact_size: Option<i64>,
    pub idempotency_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Feature Flags ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct FeatureFlag {
    pub id: Uuid,
    pub key: String,
    pub enabled: bool,
    pub variants: Option<serde_json::Value>,
    pub allocation: Option<serde_json::Value>,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Schema Mappings ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct SchemaMapping {
    pub id: Uuid,
    pub name: String,
    pub source_scope: Option<String>,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Serialize, Clone, Debug)]
pub struct SchemaMappingVersion {
    pub id: Uuid,
    pub mapping_id: Uuid,
    pub version_number: i32,
    pub mapping_rules: serde_json::Value,
    pub explicit_defaults: Option<serde_json::Value>,
    pub unit_rules: Option<serde_json::Value>,
    pub timezone_rules: Option<serde_json::Value>,
    pub fingerprint_keys: Option<serde_json::Value>,
    pub pii_fields: Option<serde_json::Value>,
    pub change_note: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Standardization ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct StandardizationJob {
    pub id: Uuid,
    pub mapping_version_id: Uuid,
    pub source_filters: Option<serde_json::Value>,
    pub run_label: Option<String>,
    pub status: String,
    pub total_records: Option<i32>,
    pub processed_records: Option<i32>,
    pub failed_records: Option<i32>,
    pub retry_count: Option<i32>,
    pub error_info: Option<String>,
    pub idempotency_key: Option<String>,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
    pub created_by: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Serialize, Clone, Debug)]
pub struct StandardizedModel {
    pub id: Uuid,
    pub job_id: Uuid,
    pub mapping_version_id: Uuid,
    pub version_number: i32,
    pub source_window: Option<serde_json::Value>,
    pub quality_stats: Option<serde_json::Value>,
    pub record_count: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Serialize, Clone, Debug)]
pub struct StandardizedRecord {
    pub id: Uuid,
    pub model_id: Uuid,
    pub source_item_id: Option<Uuid>,
    pub fingerprint: String,
    pub raw_values: serde_json::Value,
    pub standardized_values: serde_json::Value,
    pub transformations_applied: Option<serde_json::Value>,
    pub outlier_flags: Option<serde_json::Value>,
    pub is_duplicate: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Events ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub occurred_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Metrics Snapshots ----
#[derive(Queryable, Serialize, Clone, Debug)]
pub struct MetricsSnapshot {
    pub id: Uuid,
    pub snapshot_type: String,
    pub time_range: serde_json::Value,
    pub dimensions: Option<serde_json::Value>,
    pub metrics: serde_json::Value,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ---- Auth context passed through middleware ----
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
    pub session_id: Uuid,
}

// ---- Pagination ----
#[derive(Deserialize, Clone, Debug)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        let p = self.page.unwrap_or(1).max(1) - 1;
        let s = self.page_size();
        p * s
    }
    pub fn page_size(&self) -> i64 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}
