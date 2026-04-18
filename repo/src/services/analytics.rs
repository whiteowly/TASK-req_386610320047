use crate::config::{AppConfig, DbPool};
use crate::db_instrumentation::timed_query;
use crate::errors::AppError;
use crate::models::{Event, MetricsSnapshot, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateEventRequest {
    pub event_type: String,
    pub occurred_at: Option<chrono::NaiveDateTime>,
    pub payload: serde_json::Value,
}

#[derive(Deserialize)]
pub struct SnapshotRequest {
    pub range: serde_json::Value,
    pub dimensions: Option<serde_json::Value>,
    pub force: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct AnalyticsFilter {
    pub channel_id: Option<Uuid>,
    pub status: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub owner_user_id: Option<Uuid>,
}

const VALID_STATUSES: &[&str] = &["Draft", "InReview", "Approved", "Published", "Archived"];

impl AnalyticsFilter {
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(ref s) = self.status {
            if !VALID_STATUSES.contains(&s.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid status filter '{}'. Allowed: {}", s, VALID_STATUSES.join(", ")
                )));
            }
        }
        if let Some(ref d) = self.from {
            if chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
                return Err(AppError::BadRequest("Invalid 'from' date format, expected YYYY-MM-DD".to_string()));
            }
        }
        if let Some(ref d) = self.to {
            if chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
                return Err(AppError::BadRequest("Invalid 'to' date format, expected YYYY-MM-DD".to_string()));
            }
        }
        Ok(())
    }

    fn from_ts(&self) -> Option<chrono::NaiveDateTime> {
        self.from.as_ref().and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .and_then(|d| d.and_hms_opt(0, 0, 0))
    }

    fn to_ts(&self) -> Option<chrono::NaiveDateTime> {
        self.to.as_ref().and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .and_then(|d| d.and_hms_opt(23, 59, 59))
    }
}

#[derive(Serialize)]
pub struct KpiResult {
    pub total_items: i64,
    pub total_published: i64,
    pub total_draft: i64,
    pub total_archived: i64,
    pub total_users: i64,
}

#[derive(Serialize)]
pub struct OperationalResult {
    pub total_imports: i64,
    pub total_exports: i64,
    pub total_searches: i64,
    pub error_count: i64,
}

pub fn create_event(pool: &DbPool, config: &AppConfig, req: &CreateEventRequest, actor_id: Option<Uuid>) -> Result<Event, AppError> {
    let conn = &mut pool.get()?;
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    let occurred = req.occurred_at.unwrap_or(now);
    let threshold = config.slow_query_threshold_ms;

    // Redact sensitive fields from payload
    let payload = crate::audit::redact_event_payload(&req.payload);

    timed_query("create_event_insert", threshold, || {
        diesel::insert_into(events::table)
            .values((
                events::id.eq(id),
                events::event_type.eq(&req.event_type),
                events::actor_id.eq(actor_id),
                events::payload.eq(&payload),
                events::occurred_at.eq(occurred),
                events::created_at.eq(now),
                events::updated_at.eq(now),
            ))
            .execute(conn)
    })?;

    events::table.filter(events::id.eq(id)).first(conn).map_err(|e| AppError::Internal(e.to_string()))
}

pub fn list_events(pool: &DbPool, config: &AppConfig, pagination: &PaginationParams, event_type: Option<&str>) -> Result<(Vec<Event>, i64), AppError> {
    let conn = &mut pool.get()?;
    let threshold = config.slow_query_threshold_ms;
    let mut query = events::table.into_boxed();
    let mut count_query = events::table.into_boxed();
    if let Some(et) = event_type {
        query = query.filter(events::event_type.eq(et.to_string()));
        count_query = count_query.filter(events::event_type.eq(et.to_string()));
    }
    let total: i64 = timed_query("list_events_count", threshold, || {
        count_query.count().get_result(conn)
    })?;
    let results = timed_query("list_events_fetch", threshold, || {
        query.order(events::created_at.desc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)
    })?;
    Ok((results, total))
}

pub fn create_snapshot(pool: &DbPool, config: &AppConfig, req: &SnapshotRequest, user_id: Uuid, username: &str) -> Result<MetricsSnapshot, AppError> {
    let conn = &mut pool.get()?;
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    let threshold = config.slow_query_threshold_ms;

    let item_count: i64 = timed_query("snapshot_items_count", threshold, || items::table.count().get_result(conn))?;
    let user_count: i64 = timed_query("snapshot_users_count", threshold, || users::table.count().get_result(conn))?;
    let import_count: i64 = timed_query("snapshot_imports_count", threshold, || imports::table.count().get_result(conn))?;
    let export_count: i64 = timed_query("snapshot_exports_count", threshold, || exports::table.count().get_result(conn))?;

    let metrics_val = serde_json::json!({
        "total_items": item_count,
        "total_users": user_count,
        "total_imports": import_count,
        "total_exports": export_count,
        "snapshot_at": now.to_string(),
    });

    diesel::insert_into(metrics_snapshots::table)
        .values((
            metrics_snapshots::id.eq(id),
            metrics_snapshots::snapshot_type.eq("manual"),
            metrics_snapshots::time_range.eq(&req.range),
            metrics_snapshots::dimensions.eq(&req.dimensions),
            metrics_snapshots::metrics.eq(&metrics_val),
            metrics_snapshots::created_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(user_id), username, "CREATE_SNAPSHOT", "metrics_snapshot", Some(id), None, None, None, None);
    metrics_snapshots::table.filter(metrics_snapshots::id.eq(id)).first(conn).map_err(|e| AppError::Internal(e.to_string()))
}

pub fn list_snapshots(pool: &DbPool, pagination: &PaginationParams) -> Result<(Vec<MetricsSnapshot>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = metrics_snapshots::table.count().get_result(conn)?;
    let results = metrics_snapshots::table.order(metrics_snapshots::created_at.desc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)?;
    Ok((results, total))
}

pub fn get_kpis(pool: &DbPool, filters: &AnalyticsFilter) -> Result<KpiResult, AppError> {
    filters.validate()?;
    let conn = &mut pool.get()?;

    fn apply_item_filters<'a>(
        q: diesel::dsl::IntoBoxed<'a, items::table, diesel::pg::Pg>,
        f: &AnalyticsFilter,
    ) -> diesel::dsl::IntoBoxed<'a, items::table, diesel::pg::Pg> {
        let mut q = q;
        if let Some(ref cid) = f.channel_id { q = q.filter(items::channel_id.eq(*cid)); }
        if let Some(ref uid) = f.owner_user_id { q = q.filter(items::owner_user_id.eq(*uid)); }
        if let Some(from) = f.from_ts() { q = q.filter(items::created_at.ge(from)); }
        if let Some(to) = f.to_ts() { q = q.filter(items::created_at.le(to)); }
        // Note: status filter scoped per-count, not here
        q
    }

    let base = || apply_item_filters(items::table.into_boxed(), filters);

    let total_items: i64 = if let Some(ref s) = filters.status {
        base().filter(items::status.eq(s)).count().get_result(conn)?
    } else {
        base().count().get_result(conn)?
    };
    let total_published: i64 = base().filter(items::status.eq("Published")).count().get_result(conn)?;
    let total_draft: i64 = base().filter(items::status.eq("Draft")).count().get_result(conn)?;
    let total_archived: i64 = base().filter(items::status.eq("Archived")).count().get_result(conn)?;
    let total_users: i64 = users::table.count().get_result(conn)?;
    Ok(KpiResult { total_items, total_published, total_draft, total_archived, total_users })
}

pub fn get_operational(pool: &DbPool, filters: &AnalyticsFilter) -> Result<OperationalResult, AppError> {
    filters.validate()?;
    let conn = &mut pool.get()?;

    let mut imp_query = imports::table.into_boxed();
    let mut imp_err_query = imports::table.into_boxed();
    let mut exp_query = exports::table.into_boxed();
    let mut search_query = searches::table.into_boxed();

    if let Some(ref cid) = filters.channel_id {
        imp_query = imp_query.filter(imports::channel_id.eq(*cid));
        imp_err_query = imp_err_query.filter(imports::channel_id.eq(*cid));
    }
    if let Some(from) = filters.from_ts() {
        imp_query = imp_query.filter(imports::created_at.ge(from));
        imp_err_query = imp_err_query.filter(imports::created_at.ge(from));
        exp_query = exp_query.filter(exports::created_at.ge(from));
        search_query = search_query.filter(searches::created_at.ge(from));
    }
    if let Some(to) = filters.to_ts() {
        imp_query = imp_query.filter(imports::created_at.le(to));
        imp_err_query = imp_err_query.filter(imports::created_at.le(to));
        exp_query = exp_query.filter(exports::created_at.le(to));
        search_query = search_query.filter(searches::created_at.le(to));
    }

    let total_imports: i64 = imp_query.count().get_result(conn)?;
    let total_exports: i64 = exp_query.count().get_result(conn)?;
    let total_searches: i64 = search_query.count().get_result(conn)?;
    let error_count: i64 = imp_err_query.filter(imports::status.eq("failed")).count().get_result(conn)?;
    Ok(OperationalResult { total_imports, total_exports, total_searches, error_count })
}

pub fn create_analytics_export(pool: &DbPool, config: &crate::config::AppConfig, user_id: Uuid, username: &str) -> Result<crate::models::Export, AppError> {
    let req = super::exports::CreateExportRequest {
        scope_filters: serde_json::json!({"type": "analytics"}),
        format: "csv".to_string(),
        include_explanations: Some(false),
        mask_sensitive: Some(false),
    };
    super::exports::create_export(pool, config, &req, user_id, username, None)
}
