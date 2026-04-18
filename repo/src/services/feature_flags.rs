use crate::config::DbPool;
use crate::errors::AppError;
use crate::models::{FeatureFlag, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateFlagRequest {
    pub key: String,
    pub enabled: bool,
    pub variants: Option<serde_json::Value>,
    pub allocation: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateFlagRequest {
    pub enabled: Option<bool>,
    pub variants: Option<serde_json::Value>,
    pub allocation: Option<serde_json::Value>,
    pub description: Option<String>,
}

pub fn list_flags(pool: &DbPool, pagination: &PaginationParams) -> Result<(Vec<FeatureFlag>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = feature_flags::table.count().get_result(conn)?;
    let results = feature_flags::table.order(feature_flags::key.asc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)?;
    Ok((results, total))
}

pub fn create_flag(pool: &DbPool, req: &CreateFlagRequest, actor_id: Uuid, actor_username: &str) -> Result<FeatureFlag, AppError> {
    let conn = &mut pool.get()?;
    if req.key.trim().is_empty() {
        return Err(AppError::ValidationError(vec![crate::errors::FieldError { field: "key".to_string(), reason: "Key is required".to_string() }]));
    }

    // Validate allocation if provided
    if let Some(ref alloc) = req.allocation {
        if let Some(arr) = alloc.as_array() {
            let total: f64 = arr.iter().filter_map(|v| v.get("percentage").and_then(|p| p.as_f64())).sum();
            if total > 100.0 {
                return Err(AppError::ValidationError(vec![crate::errors::FieldError { field: "allocation".to_string(), reason: "Total allocation exceeds 100%".to_string() }]));
            }
        }
    }

    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(feature_flags::table)
        .values((
            feature_flags::id.eq(id),
            feature_flags::key.eq(req.key.trim()),
            feature_flags::enabled.eq(req.enabled),
            feature_flags::variants.eq(&req.variants),
            feature_flags::allocation.eq(&req.allocation),
            feature_flags::description.eq(req.description.as_deref()),
            feature_flags::created_at.eq(now),
            feature_flags::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "CREATE_FEATURE_FLAG", "feature_flag", Some(id), None, Some(serde_json::json!({"key": req.key, "enabled": req.enabled})), None, None);
    feature_flags::table.filter(feature_flags::id.eq(id)).first(conn).map_err(|e| AppError::Internal(e.to_string()))
}

pub fn update_flag(pool: &DbPool, key: &str, req: &UpdateFlagRequest, actor_id: Uuid, actor_username: &str) -> Result<FeatureFlag, AppError> {
    let conn = &mut pool.get()?;
    let existing: FeatureFlag = feature_flags::table.filter(feature_flags::key.eq(key)).first(conn)
        .map_err(|_| AppError::NotFound("Feature flag not found".to_string()))?;

    let now = Utc::now().naive_utc();
    diesel::update(feature_flags::table.filter(feature_flags::key.eq(key)))
        .set((
            feature_flags::enabled.eq(req.enabled.unwrap_or(existing.enabled)),
            feature_flags::variants.eq(req.variants.as_ref().or(existing.variants.as_ref())),
            feature_flags::allocation.eq(req.allocation.as_ref().or(existing.allocation.as_ref())),
            feature_flags::description.eq(req.description.as_deref().or(existing.description.as_deref())),
            feature_flags::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "UPDATE_FEATURE_FLAG", "feature_flag", Some(existing.id), None, Some(serde_json::json!({"key": key, "enabled": req.enabled})), None, None);
    feature_flags::table.filter(feature_flags::key.eq(key)).first(conn).map_err(|e| AppError::Internal(e.to_string()))
}
