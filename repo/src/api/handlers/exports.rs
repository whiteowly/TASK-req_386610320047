use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::{AuthContext, PaginationParams};
use crate::services::exports as export_service;
use crate::api::dto;
use crate::errors::AppError;

fn require_export_role(req: &HttpRequest) -> Result<AuthContext, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("Not authenticated".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" {
        return Err(AppError::Forbidden("Export access restricted to Analyst and Administrator".to_string()));
    }
    Ok(ctx)
}

pub async fn create_export(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Json<export_service::CreateExportRequest>) -> Result<HttpResponse, AppError> {
    let ctx = require_export_role(&req)?;
    let idem = req.headers().get("X-Idempotency-Key").and_then(|v| v.to_str().ok());
    let export = export_service::create_export(&pool, &config, &body, ctx.user_id, &ctx.username, idem)?;
    Ok(dto::created_response(export))
}

pub async fn list_exports(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_export_role(&req)?;
    let (exports, total) = export_service::list_exports(&pool, &query)?;
    Ok(dto::paginated_response(exports, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_export(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>) -> Result<HttpResponse, AppError> {
    let _ctx = require_export_role(&req)?;
    let export = export_service::get_export(&pool, *path)?;
    Ok(dto::success_response(export))
}

pub async fn download(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>) -> Result<HttpResponse, AppError> {
    let _ctx = require_export_role(&req)?;
    let (data, ct) = export_service::download_export(&pool, *path)?;
    Ok(HttpResponse::Ok().content_type(ct).body(data))
}
