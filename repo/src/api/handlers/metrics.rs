use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::{AuthContext, PaginationParams};
use crate::services::analytics;
use crate::api::dto;
use crate::errors::AppError;

pub async fn create_snapshot(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Json<analytics::SnapshotRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let snapshot = analytics::create_snapshot(&pool, &config, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(snapshot))
}

pub async fn list_snapshots(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let (snapshots, total) = analytics::list_snapshots(&pool, &query)?;
    Ok(dto::paginated_response(snapshots, total, query.page.unwrap_or(1), query.page_size()))
}
