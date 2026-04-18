use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use crate::config::DbPool;
use crate::models::{AuthContext, PaginationParams};
use crate::api::dto;
use crate::errors::AppError;
use crate::schema::*;
use diesel::prelude::*;
use uuid::Uuid;
use chrono::Utc;

fn require_role(req: &HttpRequest) -> Result<AuthContext, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    Ok(ctx)
}

pub async fn create_job(req: HttpRequest, pool: web::Data<DbPool>, body: web::Json<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let mv_id = body.get("mapping_version_id").and_then(|m| m.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(AppError::BadRequest("mapping_version_id required".to_string()))?;
    if let Some(key) = body.get("idempotency_key").and_then(|k| k.as_str()) {
        let existing_id: Option<Uuid> = standardization_jobs::table
            .filter(standardization_jobs::idempotency_key.eq(key))
            .select(standardization_jobs::id)
            .first(conn).ok();
        if let Some(eid) = existing_id {
            return Ok(dto::success_response(serde_json::json!({"id": eid, "idempotency_key": key})));
        }
    }
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    diesel::insert_into(standardization_jobs::table).values((
        standardization_jobs::id.eq(id), standardization_jobs::mapping_version_id.eq(mv_id),
        standardization_jobs::source_filters.eq(body.get("source_filters").cloned()),
        standardization_jobs::run_label.eq(body.get("run_label").and_then(|r| r.as_str())),
        standardization_jobs::status.eq("queued"),
        standardization_jobs::idempotency_key.eq(body.get("idempotency_key").and_then(|k| k.as_str())),
        standardization_jobs::created_by.eq(ctx.user_id),
        standardization_jobs::created_at.eq(now), standardization_jobs::updated_at.eq(now),
    )).execute(conn)?;

    // Return a subset of fields to avoid the 16-column Diesel tuple limit
    let (job_id, status, mv, created): (Uuid, String, Uuid, chrono::NaiveDateTime) = standardization_jobs::table
        .filter(standardization_jobs::id.eq(id))
        .select((standardization_jobs::id, standardization_jobs::status, standardization_jobs::mapping_version_id, standardization_jobs::created_at))
        .first(conn)?;
    Ok(dto::created_response(serde_json::json!({"id": job_id, "status": status, "mapping_version_id": mv, "created_at": created.to_string()})))
}

pub async fn list_jobs(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let total: i64 = standardization_jobs::table.count().get_result(conn)?;
    let results: Vec<(Uuid, String, Uuid, Option<String>, chrono::NaiveDateTime)> = standardization_jobs::table
        .select((standardization_jobs::id, standardization_jobs::status, standardization_jobs::mapping_version_id, standardization_jobs::run_label, standardization_jobs::created_at))
        .order(standardization_jobs::created_at.desc())
        .offset(query.offset()).limit(query.page_size()).load(conn)?;
    let items: Vec<serde_json::Value> = results.into_iter().map(|(id, status, mv, label, created)| {
        serde_json::json!({"id": id, "status": status, "mapping_version_id": mv, "run_label": label, "created_at": created.to_string()})
    }).collect();
    Ok(dto::paginated_response(items, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_job(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<Uuid>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let (id, status, mv, label, total_r, proc_r, fail_r, err_info, created): (Uuid, String, Uuid, Option<String>, Option<i32>, Option<i32>, Option<i32>, Option<String>, chrono::NaiveDateTime) =
        standardization_jobs::table
            .filter(standardization_jobs::id.eq(*path))
            .select((standardization_jobs::id, standardization_jobs::status, standardization_jobs::mapping_version_id,
                standardization_jobs::run_label, standardization_jobs::total_records, standardization_jobs::processed_records,
                standardization_jobs::failed_records, standardization_jobs::error_info, standardization_jobs::created_at))
            .first(conn).map_err(|_| AppError::NotFound("Not found".to_string()))?;
    Ok(dto::success_response(serde_json::json!({
        "id": id, "status": status, "mapping_version_id": mv, "run_label": label,
        "total_records": total_r, "processed_records": proc_r, "failed_records": fail_r,
        "error_info": err_info, "created_at": created.to_string()
    })))
}

pub async fn list_models(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let total: i64 = standardized_models::table.count().get_result(conn)?;
    let results: Vec<crate::models::StandardizedModel> = standardized_models::table.order(standardized_models::created_at.desc()).offset(query.offset()).limit(query.page_size()).load(conn)?;
    Ok(dto::paginated_response(results, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_model(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<Uuid>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let model: crate::models::StandardizedModel = standardized_models::table.filter(standardized_models::id.eq(*path)).first(conn).map_err(|_| AppError::NotFound("Not found".to_string()))?;
    Ok(dto::success_response(model))
}

pub async fn get_records(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<Uuid>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let total: i64 = standardized_records::table.filter(standardized_records::model_id.eq(*path)).count().get_result(conn)?;
    let results: Vec<crate::models::StandardizedRecord> = standardized_records::table.filter(standardized_records::model_id.eq(*path)).offset(query.offset()).limit(query.page_size()).load(conn)?;
    Ok(dto::paginated_response(results, total, query.page.unwrap_or(1), query.page_size()))
}
