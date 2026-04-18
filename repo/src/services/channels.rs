use crate::config::DbPool;
use crate::errors::AppError;
use crate::models::{Channel, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

pub fn list_channels(pool: &DbPool, pagination: &PaginationParams, search: Option<&str>) -> Result<(Vec<Channel>, i64), AppError> {
    let conn = &mut pool.get()?;
    let mut query = channels::table.into_boxed();
    if let Some(s) = search {
        query = query.filter(channels::name.ilike(format!("%{}%", s)));
    }
    let total: i64 = channels::table.count().get_result(conn)?;
    let results = query.order(channels::name.asc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)?;
    Ok((results, total))
}

pub fn create_channel(pool: &DbPool, req: &CreateChannelRequest, actor_id: Uuid, actor_username: &str) -> Result<Channel, AppError> {
    let conn = &mut pool.get()?;
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(channels::table)
        .values((
            channels::id.eq(id),
            channels::name.eq(req.name.trim()),
            channels::description.eq(req.description.as_deref()),
            channels::active.eq(true),
            channels::created_at.eq(now),
            channels::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "CREATE_CHANNEL", "channel", Some(id), None, Some(serde_json::json!({"name": req.name})), None, None);

    Ok(Channel { id, name: req.name.trim().to_string(), description: req.description.clone(), active: true, created_at: now, updated_at: now })
}

pub fn update_channel(pool: &DbPool, channel_id: Uuid, req: &UpdateChannelRequest, actor_id: Uuid, actor_username: &str) -> Result<Channel, AppError> {
    let conn = &mut pool.get()?;
    let now = Utc::now().naive_utc();

    let existing: Channel = channels::table.filter(channels::id.eq(channel_id)).first(conn)
        .map_err(|_| AppError::NotFound("Channel not found".to_string()))?;

    let new_name = req.name.as_deref().unwrap_or(&existing.name).trim().to_string();
    let new_desc = req.description.clone().or(existing.description);
    let new_active = req.active.unwrap_or(existing.active);

    diesel::update(channels::table.filter(channels::id.eq(channel_id)))
        .set((
            channels::name.eq(&new_name),
            channels::description.eq(&new_desc),
            channels::active.eq(new_active),
            channels::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "UPDATE_CHANNEL", "channel", Some(channel_id), None, Some(serde_json::json!({"name": &new_name, "active": new_active})), None, None);

    Ok(Channel { id: channel_id, name: new_name, description: new_desc, active: new_active, created_at: existing.created_at, updated_at: now })
}
