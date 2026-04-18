use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::{AuthContext, PaginationParams};
use crate::services::imports as import_service;
use crate::api::dto;
use crate::errors::AppError;

fn require_import_role(req: &HttpRequest) -> Result<AuthContext, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("Not authenticated".to_string()))?;
    if !["Author", "Administrator", "Analyst"].contains(&ctx.role.as_str()) {
        return Err(AppError::Forbidden("Import access denied".to_string()));
    }
    Ok(ctx)
}

pub async fn download_template(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, query: web::Query<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let _ctx = require_import_role(&req)?;
    let format = query.get("format").and_then(|f| f.as_str()).unwrap_or("csv");
    let (data, content_type) = import_service::generate_import_template(&pool, *path, format)?;
    Ok(HttpResponse::Ok().content_type(content_type).body(data))
}

pub async fn create_import(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Bytes) -> Result<HttpResponse, AppError> {
    let ctx = require_import_role(&req)?;
    let content_type = req.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
    let filename = if content_type.contains("xlsx") { "import.xlsx" } else { "import.csv" };
    let idempotency_key = req.headers().get("X-Idempotency-Key").and_then(|v| v.to_str().ok());
    let tv_id = req.headers().get("X-Template-Version-Id").and_then(|v| v.to_str().ok()).and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(AppError::BadRequest("X-Template-Version-Id header required".to_string()))?;
    let ch_id = req.headers().get("X-Channel-Id").and_then(|v| v.to_str().ok()).and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(AppError::BadRequest("X-Channel-Id header required".to_string()))?;
    let options = import_service::ImportOptions { template_version_id: tv_id, channel_id: ch_id };
    let import = import_service::create_import(&pool, &config, ctx.user_id, &ctx.username, filename, &body, &options, idempotency_key)?;
    Ok(dto::created_response(import))
}

pub async fn list_imports(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let ctx = require_import_role(&req)?;
    let (imports, total) = import_service::list_imports(&pool, ctx.user_id, &ctx.role, &query)?;
    Ok(dto::paginated_response(imports, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_import(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>) -> Result<HttpResponse, AppError> {
    let ctx = require_import_role(&req)?;
    let import = import_service::get_import(&pool, *path, ctx.user_id, &ctx.role)?;
    Ok(dto::success_response(import))
}

pub async fn get_errors(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let ctx = require_import_role(&req)?;
    // Enforce same ownership check as get_import
    import_service::get_import(&pool, *path, ctx.user_id, &ctx.role)?;
    let (errors, total) = import_service::get_import_errors(&pool, *path, &query)?;
    Ok(dto::paginated_response(errors, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_result(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>) -> Result<HttpResponse, AppError> {
    let ctx = require_import_role(&req)?;
    // Enforce same ownership check as get_import
    import_service::get_import(&pool, *path, ctx.user_id, &ctx.role)?;
    let (accepted, rejected) = import_service::get_import_result(&pool, *path)?;
    Ok(dto::success_response(serde_json::json!({"accepted": accepted, "rejected": rejected})))
}
