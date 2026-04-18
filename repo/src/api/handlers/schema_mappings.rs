use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
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

pub async fn create_mapping(req: HttpRequest, pool: web::Data<DbPool>, body: web::Json<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    let name = body.get("name").and_then(|n| n.as_str()).ok_or(AppError::BadRequest("name required".to_string()))?;
    diesel::insert_into(schema_mappings::table).values((
        schema_mappings::id.eq(id), schema_mappings::name.eq(name),
        schema_mappings::source_scope.eq(body.get("source_scope").and_then(|s| s.as_str())),
        schema_mappings::description.eq(body.get("description").and_then(|d| d.as_str())),
        schema_mappings::created_by.eq(ctx.user_id),
        schema_mappings::created_at.eq(now), schema_mappings::updated_at.eq(now),
    )).execute(conn)?;
    crate::audit::log_audit(&pool, Some(ctx.user_id), &ctx.username, "CREATE_SCHEMA_MAPPING", "schema_mapping", Some(id), None, Some(serde_json::json!({"name": name})), None, None);
    let mapping: crate::models::SchemaMapping = schema_mappings::table.filter(schema_mappings::id.eq(id)).first(conn)?;
    Ok(dto::created_response(mapping))
}

pub async fn list_mappings(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let total: i64 = schema_mappings::table.count().get_result(conn)?;
    let results: Vec<crate::models::SchemaMapping> = schema_mappings::table.order(schema_mappings::created_at.desc()).offset(query.offset()).limit(query.page_size()).load(conn)?;
    Ok(dto::paginated_response(results, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_mapping(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<Uuid>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let m: crate::models::SchemaMapping = schema_mappings::table.filter(schema_mappings::id.eq(*path)).first(conn).map_err(|_| AppError::NotFound("Not found".to_string()))?;
    Ok(dto::success_response(m))
}

pub async fn create_version(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<Uuid>, body: web::Json<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let mapping_id = *path;
    let max_ver: Option<i32> = schema_mapping_versions::table.filter(schema_mapping_versions::mapping_id.eq(mapping_id)).select(diesel::dsl::max(schema_mapping_versions::version_number)).first(conn)?;
    let ver = max_ver.unwrap_or(0) + 1;
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    diesel::insert_into(schema_mapping_versions::table).values((
        schema_mapping_versions::id.eq(id), schema_mapping_versions::mapping_id.eq(mapping_id),
        schema_mapping_versions::version_number.eq(ver),
        schema_mapping_versions::mapping_rules.eq(body.get("mapping_rules").cloned().unwrap_or(serde_json::json!({}))),
        schema_mapping_versions::explicit_defaults.eq(body.get("explicit_defaults").cloned()),
        schema_mapping_versions::unit_rules.eq(body.get("unit_rules").cloned()),
        schema_mapping_versions::timezone_rules.eq(body.get("timezone_rules").cloned()),
        schema_mapping_versions::fingerprint_keys.eq(body.get("fingerprint_keys").cloned()),
        schema_mapping_versions::pii_fields.eq(body.get("pii_fields").cloned()),
        schema_mapping_versions::change_note.eq(body.get("change_note").and_then(|c| c.as_str())),
        schema_mapping_versions::created_at.eq(now),
    )).execute(conn)?;
    crate::audit::log_audit(&pool, Some(ctx.user_id), &ctx.username, "CREATE_MAPPING_VERSION", "schema_mapping_version", Some(id), None, None, None, None);
    let v: crate::models::SchemaMappingVersion = schema_mapping_versions::table.filter(schema_mapping_versions::id.eq(id)).first(conn)?;
    Ok(dto::created_response(v))
}

pub async fn list_versions(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<Uuid>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_role(&req)?;
    let conn = &mut pool.get()?;
    let total: i64 = schema_mapping_versions::table.filter(schema_mapping_versions::mapping_id.eq(*path)).count().get_result(conn)?;
    let results: Vec<crate::models::SchemaMappingVersion> = schema_mapping_versions::table.filter(schema_mapping_versions::mapping_id.eq(*path)).order(schema_mapping_versions::version_number.desc()).offset(query.offset()).limit(query.page_size()).load(conn)?;
    Ok(dto::paginated_response(results, total, query.page.unwrap_or(1), query.page_size()))
}
