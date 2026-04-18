use crate::config::DbPool;
use crate::errors::AppError;
use crate::models::{Tag, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

pub fn list_tags(pool: &DbPool, pagination: &PaginationParams, prefix: Option<&str>) -> Result<(Vec<Tag>, i64), AppError> {
    let conn = &mut pool.get()?;
    let mut query = tags::table.into_boxed();
    if let Some(p) = prefix {
        query = query.filter(tags::name.ilike(format!("{}%", p.to_lowercase())));
    }
    let total: i64 = tags::table.count().get_result(conn)?;
    let results = query.order(tags::name.asc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)?;
    Ok((results, total))
}

pub fn create_tag(pool: &DbPool, req: &CreateTagRequest, actor_id: Uuid, actor_username: &str) -> Result<Tag, AppError> {
    let conn = &mut pool.get()?;
    let normalized = req.name.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(AppError::ValidationError(vec![crate::errors::FieldError { field: "name".to_string(), reason: "Tag name cannot be empty".to_string() }]));
    }

    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(tags::table)
        .values((
            tags::id.eq(id),
            tags::name.eq(&normalized),
            tags::created_at.eq(now),
            tags::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "CREATE_TAG", "tag", Some(id), None, None, None, None);

    Ok(Tag { id, name: normalized, created_at: now, updated_at: now })
}
