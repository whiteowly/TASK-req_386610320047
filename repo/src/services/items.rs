use crate::config::{AppConfig, DbPool};
use crate::db_instrumentation::timed_query;
use crate::errors::{AppError, FieldError};
use crate::models::*;
use crate::schema::*;
use diesel::prelude::*;
use chrono::{NaiveDateTime, NaiveDate, Datelike, Utc};
use chrono_tz::America::New_York;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateItemRequest {
    pub template_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub channel_id: Uuid,
    pub title: String,
    pub body: Option<String>,
    pub fields: serde_json::Value,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub fields: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
    pub change_note: Option<String>,
}

#[derive(Deserialize)]
pub struct RollbackRequest {
    pub source_version_id: Uuid,
    pub reason: String,
}

#[derive(Deserialize)]
pub struct TransitionRequest {
    pub to_status: String,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct PublishRequest {
    pub item_version_id: Uuid,
    pub publish_note: Option<String>,
}

#[derive(Deserialize)]
pub struct ItemListParams {
    pub status: Option<String>,
    pub channel_id: Option<Uuid>,
    pub tag: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

fn generate_auto_number(conn: &mut PgConnection) -> Result<String, AppError> {
    use chrono::TimeZone;

    let now_utc = Utc::now();
    let now_ny = now_utc.with_timezone(&New_York);
    let today_ny = now_ny.date_naive();
    let date_str = today_ny.format("%Y%m%d").to_string();

    // Upsert daily counter with row lock
    let result = diesel::sql_query(
        "INSERT INTO daily_counters (id, counter_date, last_sequence, created_at, updated_at) \
         VALUES ($1, $2, 1, NOW(), NOW()) \
         ON CONFLICT (counter_date) DO UPDATE SET last_sequence = daily_counters.last_sequence + 1, updated_at = NOW() \
         RETURNING last_sequence"
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Date, _>(today_ny)
    .get_result::<CounterResult>(conn)
    .map_err(|e| AppError::Internal(format!("Auto-number generation failed: {}", e)))?;

    if result.last_sequence > 99999 {
        return Err(AppError::Conflict("AUTO_NUMBER_DAILY_LIMIT_REACHED".to_string()));
    }

    Ok(format!("KO-{}-{:05}", date_str, result.last_sequence))
}

#[derive(QueryableByName)]
struct CounterResult {
    #[diesel(sql_type = diesel::sql_types::Int4)]
    pub last_sequence: i32,
}

pub fn create_item(pool: &DbPool, config: &AppConfig, req: &CreateItemRequest, user_id: Uuid, username: &str) -> Result<Item, AppError> {
    let conn = &mut pool.get()?;
    let threshold = config.slow_query_threshold_ms;

    // Get template version for validation
    let tv_id = match req.template_version_id {
        Some(id) => id,
        None => {
            // Use active version
            let t: crate::models::Template = templates::table.filter(templates::id.eq(req.template_id)).first(conn)
                .map_err(|_| AppError::NotFound("Template not found".to_string()))?;
            t.active_version_id.ok_or_else(|| AppError::Conflict("Template has no active version".to_string()))?
        }
    };

    let tv: crate::models::TemplateVersion = template_versions::table.filter(template_versions::id.eq(tv_id)).first(conn)
        .map_err(|_| AppError::NotFound("Template version not found".to_string()))?;

    // Validate fields against template schema
    crate::services::templates::validate_item_fields(&tv.field_schema, &req.fields, &tv.cross_field_rules)?;

    // Verify channel exists and is active
    let channel: crate::models::Channel = channels::table.filter(channels::id.eq(req.channel_id)).first(conn)
        .map_err(|_| AppError::NotFound("Channel not found".to_string()))?;
    if !channel.active {
        return Err(AppError::Conflict("Channel is not active".to_string()));
    }

    // Generate auto-number
    let auto_number = generate_auto_number(conn)?;

    let item_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    // Create item first with NULL current_version_id (FK constraint)
    timed_query("create_item_insert", threshold, || {
        diesel::insert_into(items::table)
            .values((
                items::id.eq(item_id),
                items::template_id.eq(req.template_id),
                items::channel_id.eq(req.channel_id),
                items::owner_user_id.eq(user_id),
                items::auto_number.eq(&auto_number),
                items::status.eq("Draft"),
                items::created_at.eq(now),
                items::updated_at.eq(now),
            ))
            .execute(conn)
    })?;

    // Create first version
    timed_query("create_item_version_insert", threshold, || {
        diesel::insert_into(item_versions::table)
            .values((
                item_versions::id.eq(version_id),
                item_versions::item_id.eq(item_id),
                item_versions::version_number.eq(1),
                item_versions::template_version_id.eq(tv_id),
                item_versions::title.eq(&req.title),
                item_versions::body.eq(req.body.as_deref()),
                item_versions::fields.eq(&req.fields),
                item_versions::created_by.eq(user_id),
                item_versions::created_at.eq(now),
            ))
            .execute(conn)
    })?;

    // Now set current_version_id on item
    diesel::update(items::table.filter(items::id.eq(item_id)))
        .set(items::current_version_id.eq(Some(version_id)))
        .execute(conn)?;

    // Add tags
    if let Some(tag_names) = &req.tags {
        for tag_name in tag_names {
            let normalized = tag_name.trim().to_lowercase();
            // Find or create tag
            let tag_id: Uuid = tags::table
                .filter(tags::name.eq(&normalized))
                .select(tags::id)
                .first(conn)
                .unwrap_or_else(|_| {
                    let tid = Uuid::new_v4();
                    diesel::insert_into(tags::table)
                        .values((tags::id.eq(tid), tags::name.eq(&normalized), tags::created_at.eq(now), tags::updated_at.eq(now)))
                        .execute(conn).ok();
                    tid
                });

            diesel::insert_into(item_version_tags::table)
                .values((
                    item_version_tags::id.eq(Uuid::new_v4()),
                    item_version_tags::item_version_id.eq(version_id),
                    item_version_tags::tag_id.eq(tag_id),
                ))
                .execute(conn)
                .ok();
        }
    }

    crate::audit::log_audit(pool, Some(user_id), username, "CREATE_ITEM", "item", Some(item_id), None, Some(serde_json::json!({"auto_number": &auto_number, "status": "Draft"})), None, None);

    let item = items::table.filter(items::id.eq(item_id)).first(conn)?;
    Ok(item)
}

pub fn list_items(pool: &DbPool, config: &AppConfig, params: &ItemListParams, auth: &AuthContext) -> Result<(Vec<Item>, i64), AppError> {
    let conn = &mut pool.get()?;
    let threshold = config.slow_query_threshold_ms;
    let mut query = items::table.into_boxed();

    // Object-scope filtering by role
    if auth.role == "Author" {
        query = query.filter(items::owner_user_id.eq(auth.user_id));
    }

    if let Some(ref status) = params.status {
        query = query.filter(items::status.eq(status));
    }
    if let Some(channel_id) = params.channel_id {
        query = query.filter(items::channel_id.eq(channel_id));
    }
    if let Some(ref from) = params.from {
        if let Ok(d) = chrono::NaiveDateTime::parse_from_str(&format!("{} 00:00:00", from), "%Y-%m-%d %H:%M:%S") {
            query = query.filter(items::created_at.ge(d));
        }
    }
    if let Some(ref to) = params.to {
        if let Ok(d) = chrono::NaiveDateTime::parse_from_str(&format!("{} 23:59:59", to), "%Y-%m-%d %H:%M:%S") {
            query = query.filter(items::created_at.le(d));
        }
    }

    let page = params.page.unwrap_or(1).max(1) - 1;
    let page_size = params.page_size.unwrap_or(20).min(100).max(1);

    // Build a parallel count query with the same filters
    let mut count_query = items::table.into_boxed();
    if auth.role == "Author" { count_query = count_query.filter(items::owner_user_id.eq(auth.user_id)); }
    if let Some(ref status) = params.status { count_query = count_query.filter(items::status.eq(status)); }
    if let Some(channel_id) = params.channel_id { count_query = count_query.filter(items::channel_id.eq(channel_id)); }
    if let Some(ref from) = params.from {
        if let Ok(d) = chrono::NaiveDateTime::parse_from_str(&format!("{} 00:00:00", from), "%Y-%m-%d %H:%M:%S") {
            count_query = count_query.filter(items::created_at.ge(d));
        }
    }
    if let Some(ref to) = params.to {
        if let Ok(d) = chrono::NaiveDateTime::parse_from_str(&format!("{} 23:59:59", to), "%Y-%m-%d %H:%M:%S") {
            count_query = count_query.filter(items::created_at.le(d));
        }
    }
    let total: i64 = timed_query("list_items_count", threshold, || {
        count_query.count().get_result(conn)
    })?;

    let results = timed_query("list_items_fetch", threshold, || {
        query
            .order(items::created_at.desc())
            .offset(page * page_size)
            .limit(page_size)
            .load(conn)
    })?;

    Ok((results, total))
}

pub fn get_item(pool: &DbPool, item_id: Uuid, auth: &AuthContext) -> Result<Item, AppError> {
    let conn = &mut pool.get()?;
    let item: Item = items::table.filter(items::id.eq(item_id)).first(conn)
        .map_err(|_| AppError::NotFound("Item not found".to_string()))?;

    // Object-level access: Authors can only see their own items
    if auth.role == "Author" && item.owner_user_id != auth.user_id {
        return Err(AppError::Forbidden("You can only access your own items".to_string()));
    }

    Ok(item)
}

pub fn update_item(pool: &DbPool, item_id: Uuid, req: &UpdateItemRequest, auth: &AuthContext) -> Result<ItemVersion, AppError> {
    let conn = &mut pool.get()?;

    let item: Item = items::table.filter(items::id.eq(item_id)).first(conn)
        .map_err(|_| AppError::NotFound("Item not found".to_string()))?;

    // Object-level ownership check
    if auth.role == "Author" && item.owner_user_id != auth.user_id {
        return Err(AppError::Forbidden("You can only edit your own items".to_string()));
    }

    // Draft-only editing
    if item.status != "Draft" {
        return Err(AppError::Conflict("Items can only be edited in Draft status".to_string()));
    }

    // Get current version
    let current_version: ItemVersion = item_versions::table
        .filter(item_versions::item_id.eq(item_id))
        .order(item_versions::version_number.desc())
        .select((
            item_versions::id, item_versions::item_id, item_versions::version_number,
            item_versions::template_version_id, item_versions::title, item_versions::body,
            item_versions::fields, item_versions::sensitive_fields_encrypted,
            item_versions::change_note, item_versions::created_by,
            item_versions::rollback_source_version_id, item_versions::created_at,
            item_versions::updated_at,
        ))
        .first(conn)
        .map_err(|_| AppError::Internal("No current version found".to_string()))?;

    let new_title = req.title.as_deref().unwrap_or(&current_version.title);
    let new_body = req.body.clone().or(current_version.body.clone());
    let new_fields = req.fields.clone().unwrap_or(current_version.fields.clone());

    // Validate if fields changed
    if req.fields.is_some() {
        let tv: crate::models::TemplateVersion = template_versions::table
            .filter(template_versions::id.eq(current_version.template_version_id))
            .first(conn)?;
        crate::services::templates::validate_item_fields(&tv.field_schema, &new_fields, &tv.cross_field_rules)?;
    }

    let new_version_number = current_version.version_number + 1;
    let new_version_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(item_versions::table)
        .values((
            item_versions::id.eq(new_version_id),
            item_versions::item_id.eq(item_id),
            item_versions::version_number.eq(new_version_number),
            item_versions::template_version_id.eq(current_version.template_version_id),
            item_versions::title.eq(new_title),
            item_versions::body.eq(new_body.as_deref()),
            item_versions::fields.eq(&new_fields),
            item_versions::change_note.eq(req.change_note.as_deref()),
            item_versions::created_by.eq(auth.user_id),
            item_versions::created_at.eq(now),
        ))
        .execute(conn)?;

    // Update tags if provided
    if let Some(tag_names) = &req.tags {
        for tag_name in tag_names {
            let normalized = tag_name.trim().to_lowercase();
            let tag_id: Uuid = tags::table.filter(tags::name.eq(&normalized)).select(tags::id).first(conn)
                .unwrap_or_else(|_| {
                    let tid = Uuid::new_v4();
                    diesel::insert_into(tags::table).values((tags::id.eq(tid), tags::name.eq(&normalized), tags::created_at.eq(now), tags::updated_at.eq(now))).execute(conn).ok();
                    tid
                });
            diesel::insert_into(item_version_tags::table)
                .values((item_version_tags::id.eq(Uuid::new_v4()), item_version_tags::item_version_id.eq(new_version_id), item_version_tags::tag_id.eq(tag_id)))
                .execute(conn).ok();
        }
    }

    // Update current_version_id on item
    diesel::update(items::table.filter(items::id.eq(item_id)))
        .set((items::current_version_id.eq(Some(new_version_id)), items::updated_at.eq(now)))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(auth.user_id), &auth.username, "UPDATE_ITEM", "item", Some(item_id), None, Some(serde_json::json!({"version": new_version_number})), req.change_note.as_deref(), None);

    // Return the new version (without search_vector which is auto-generated)
    Ok(ItemVersion {
        id: new_version_id, item_id, version_number: new_version_number,
        template_version_id: current_version.template_version_id,
        title: new_title.to_string(), body: new_body, fields: new_fields,
        sensitive_fields_encrypted: None, change_note: req.change_note.clone(),
        created_by: auth.user_id, rollback_source_version_id: None, created_at: now, updated_at: now,
    })
}

pub fn list_item_versions(pool: &DbPool, item_id: Uuid, pagination: &PaginationParams) -> Result<(Vec<ItemVersionResponse>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = item_versions::table.filter(item_versions::item_id.eq(item_id)).count().get_result(conn)?;

    let versions: Vec<ItemVersion> = item_versions::table
        .filter(item_versions::item_id.eq(item_id))
        .order(item_versions::version_number.desc())
        .offset(pagination.offset())
        .limit(pagination.page_size())
        .select((
            item_versions::id, item_versions::item_id, item_versions::version_number,
            item_versions::template_version_id, item_versions::title, item_versions::body,
            item_versions::fields, item_versions::sensitive_fields_encrypted,
            item_versions::change_note, item_versions::created_by,
            item_versions::rollback_source_version_id, item_versions::created_at,
            item_versions::updated_at,
        ))
        .load(conn)?;

    let responses: Vec<ItemVersionResponse> = versions.into_iter().map(|v| {
        let tag_names: Vec<String> = item_version_tags::table
            .inner_join(tags::table)
            .filter(item_version_tags::item_version_id.eq(v.id))
            .select(tags::name)
            .load(conn)
            .unwrap_or_default();

        ItemVersionResponse {
            id: v.id, item_id: v.item_id, version_number: v.version_number,
            template_version_id: v.template_version_id, title: v.title, body: v.body,
            fields: v.fields, change_note: v.change_note, created_by: v.created_by,
            rollback_source_version_id: v.rollback_source_version_id, created_at: v.created_at,
            tags: tag_names,
        }
    }).collect();

    Ok((responses, total))
}

pub fn get_item_version(pool: &DbPool, item_id: Uuid, version_id: Uuid) -> Result<ItemVersionResponse, AppError> {
    let conn = &mut pool.get()?;

    let v: ItemVersion = item_versions::table
        .filter(item_versions::id.eq(version_id))
        .filter(item_versions::item_id.eq(item_id))
        .select((
            item_versions::id, item_versions::item_id, item_versions::version_number,
            item_versions::template_version_id, item_versions::title, item_versions::body,
            item_versions::fields, item_versions::sensitive_fields_encrypted,
            item_versions::change_note, item_versions::created_by,
            item_versions::rollback_source_version_id, item_versions::created_at,
            item_versions::updated_at,
        ))
        .first(conn)
        .map_err(|_| AppError::NotFound("Item version not found".to_string()))?;

    let tag_names: Vec<String> = item_version_tags::table
        .inner_join(tags::table)
        .filter(item_version_tags::item_version_id.eq(v.id))
        .select(tags::name)
        .load(conn)
        .unwrap_or_default();

    Ok(ItemVersionResponse {
        id: v.id, item_id: v.item_id, version_number: v.version_number,
        template_version_id: v.template_version_id, title: v.title, body: v.body,
        fields: v.fields, change_note: v.change_note, created_by: v.created_by,
        rollback_source_version_id: v.rollback_source_version_id, created_at: v.created_at,
        tags: tag_names,
    })
}

pub fn rollback_item(pool: &DbPool, item_id: Uuid, req: &RollbackRequest, auth: &AuthContext) -> Result<ItemVersionResponse, AppError> {
    let conn = &mut pool.get()?;

    let item: Item = items::table.filter(items::id.eq(item_id)).first(conn)
        .map_err(|_| AppError::NotFound("Item not found".to_string()))?;

    if auth.role == "Author" && item.owner_user_id != auth.user_id {
        return Err(AppError::Forbidden("You can only rollback your own items".to_string()));
    }

    if item.status != "Draft" {
        return Err(AppError::Conflict("Rollback is only allowed in Draft status".to_string()));
    }

    // Get previous versions (up to 10)
    let recent_versions: Vec<(Uuid, i32)> = item_versions::table
        .filter(item_versions::item_id.eq(item_id))
        .order(item_versions::version_number.desc())
        .select((item_versions::id, item_versions::version_number))
        .limit(11) // current + 10 previous
        .load(conn)?;

    // Source must be among the previous 10 (excluding current latest)
    let eligible_ids: Vec<Uuid> = recent_versions.iter().skip(1).take(10).map(|(id, _)| *id).collect();

    if !eligible_ids.contains(&req.source_version_id) {
        return Err(AppError::Conflict("Source version must be among the previous 10 versions".to_string()));
    }

    // Get source version data
    let source: ItemVersion = item_versions::table
        .filter(item_versions::id.eq(req.source_version_id))
        .select((
            item_versions::id, item_versions::item_id, item_versions::version_number,
            item_versions::template_version_id, item_versions::title, item_versions::body,
            item_versions::fields, item_versions::sensitive_fields_encrypted,
            item_versions::change_note, item_versions::created_by,
            item_versions::rollback_source_version_id, item_versions::created_at,
            item_versions::updated_at,
        ))
        .first(conn)
        .map_err(|_| AppError::NotFound("Source version not found".to_string()))?;

    // Clone-forward: create new version from source
    let new_version_number = recent_versions[0].1 + 1;
    let new_version_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(item_versions::table)
        .values((
            item_versions::id.eq(new_version_id),
            item_versions::item_id.eq(item_id),
            item_versions::version_number.eq(new_version_number),
            item_versions::template_version_id.eq(source.template_version_id),
            item_versions::title.eq(&source.title),
            item_versions::body.eq(source.body.as_deref()),
            item_versions::fields.eq(&source.fields),
            item_versions::change_note.eq(Some(format!("Rollback from version {}", source.version_number))),
            item_versions::created_by.eq(auth.user_id),
            item_versions::rollback_source_version_id.eq(Some(req.source_version_id)),
            item_versions::created_at.eq(now),
        ))
        .execute(conn)?;

    // Clone tags from source version
    let source_tags: Vec<Uuid> = item_version_tags::table
        .filter(item_version_tags::item_version_id.eq(source.id))
        .select(item_version_tags::tag_id)
        .load(conn)?;
    for tag_id in &source_tags {
        diesel::insert_into(item_version_tags::table)
            .values((
                item_version_tags::id.eq(Uuid::new_v4()),
                item_version_tags::item_version_id.eq(new_version_id),
                item_version_tags::tag_id.eq(tag_id),
            ))
            .execute(conn).ok();
    }

    // Update item pointer
    diesel::update(items::table.filter(items::id.eq(item_id)))
        .set((items::current_version_id.eq(Some(new_version_id)), items::updated_at.eq(now)))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(auth.user_id), &auth.username, "ROLLBACK_ITEM", "item", Some(item_id), None, Some(serde_json::json!({"source_version": source.version_number, "new_version": new_version_number})), Some(&req.reason), None);

    let tag_names: Vec<String> = item_version_tags::table
        .inner_join(tags::table)
        .filter(item_version_tags::item_version_id.eq(new_version_id))
        .select(tags::name)
        .load(conn)?;

    Ok(ItemVersionResponse {
        id: new_version_id, item_id, version_number: new_version_number,
        template_version_id: source.template_version_id, title: source.title, body: source.body,
        fields: source.fields, change_note: Some(format!("Rollback from version {}", source.version_number)),
        created_by: auth.user_id, rollback_source_version_id: Some(req.source_version_id), created_at: now,
        tags: tag_names,
    })
}

pub fn transition_item(pool: &DbPool, item_id: Uuid, req: &TransitionRequest, auth: &AuthContext) -> Result<Item, AppError> {
    let conn = &mut pool.get()?;

    let item: Item = items::table.filter(items::id.eq(item_id)).first(conn)
        .map_err(|_| AppError::NotFound("Item not found".to_string()))?;

    let current = &item.status;
    let target = &req.to_status;

    // State machine enforcement
    // Approved -> Published is blocked here; publishing must use the dedicated
    // publish_item endpoint which binds published_version_id, published_template_version_id,
    // and published_at.
    let allowed = match (current.as_str(), target.as_str()) {
        ("Draft", "InReview") => auth.role == "Author" || auth.role == "Administrator",
        ("InReview", "Draft") => auth.role == "Reviewer" || auth.role == "Administrator",
        ("InReview", "Approved") => auth.role == "Reviewer",
        ("Approved", "Published") => {
            return Err(AppError::Conflict(
                "INVALID_TRANSITION: Use the POST /items/{id}/publish endpoint to publish. \
                 Direct transition to Published is not allowed because publishing requires \
                 binding a specific item_version_id and template_version_id.".to_string()
            ));
        },
        ("Published", "Archived") => auth.role == "Administrator",
        _ => false,
    };

    if !allowed {
        return Err(AppError::Conflict(format!("INVALID_TRANSITION: {} -> {} not allowed for role {}", current, target, auth.role)));
    }

    // For Author submitting to review, check ownership
    if current == "Draft" && target == "InReview" && auth.role == "Author" && item.owner_user_id != auth.user_id {
        return Err(AppError::Forbidden("You can only submit your own items for review".to_string()));
    }

    let now = Utc::now().naive_utc();
    let mut update_entered_review = false;

    if target == "InReview" {
        update_entered_review = true;
    }

    if update_entered_review {
        diesel::update(items::table.filter(items::id.eq(item_id)))
            .set((
                items::status.eq(target),
                items::entered_in_review_at.eq(Some(now)),
                items::updated_at.eq(now),
            ))
            .execute(conn)?;
    } else {
        diesel::update(items::table.filter(items::id.eq(item_id)))
            .set((
                items::status.eq(target),
                items::entered_in_review_at.eq(None::<NaiveDateTime>),
                items::updated_at.eq(now),
            ))
            .execute(conn)?;
    }

    crate::audit::log_audit(pool, Some(auth.user_id), &auth.username, "TRANSITION_ITEM", "item", Some(item_id), Some(serde_json::json!({"from_status": current})), Some(serde_json::json!({"to_status": target})), req.reason.as_deref(), None);

    let updated_item: Item = items::table.filter(items::id.eq(item_id)).first(conn)?;
    Ok(updated_item)
}

pub fn publish_item(pool: &DbPool, item_id: Uuid, req: &PublishRequest, auth: &AuthContext) -> Result<Item, AppError> {
    let conn = &mut pool.get()?;

    let item: Item = items::table.filter(items::id.eq(item_id)).first(conn)
        .map_err(|_| AppError::NotFound("Item not found".to_string()))?;

    if item.status != "Approved" {
        return Err(AppError::Conflict("Item must be in Approved status to publish".to_string()));
    }

    // Ownership check for Author
    if auth.role == "Author" && item.owner_user_id != auth.user_id {
        return Err(AppError::Forbidden("You can only publish your own items".to_string()));
    }

    // Verify the version exists and belongs to this item
    let version: ItemVersion = item_versions::table
        .filter(item_versions::id.eq(req.item_version_id))
        .filter(item_versions::item_id.eq(item_id))
        .select((
            item_versions::id, item_versions::item_id, item_versions::version_number,
            item_versions::template_version_id, item_versions::title, item_versions::body,
            item_versions::fields, item_versions::sensitive_fields_encrypted,
            item_versions::change_note, item_versions::created_by,
            item_versions::rollback_source_version_id, item_versions::created_at,
            item_versions::updated_at,
        ))
        .first(conn)
        .map_err(|_| AppError::NotFound("Item version not found".to_string()))?;

    let now = Utc::now().naive_utc();

    diesel::update(items::table.filter(items::id.eq(item_id)))
        .set((
            items::status.eq("Published"),
            items::published_at.eq(Some(now)),
            items::published_version_id.eq(Some(req.item_version_id)),
            items::published_template_version_id.eq(Some(version.template_version_id)),
            items::entered_in_review_at.eq(None::<NaiveDateTime>),
            items::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(auth.user_id), &auth.username, "PUBLISH_ITEM", "item", Some(item_id), None, Some(serde_json::json!({"published_version_id": req.item_version_id, "template_version_id": version.template_version_id})), req.publish_note.as_deref(), None);

    let updated_item: Item = items::table.filter(items::id.eq(item_id)).first(conn)?;
    Ok(updated_item)
}
