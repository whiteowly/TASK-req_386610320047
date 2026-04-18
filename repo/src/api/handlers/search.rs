use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::{AuthContext, PaginationParams};
use crate::services::search as search_service;
use crate::api::dto;
use crate::errors::AppError;

pub async fn search(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, query: web::Query<search_service::SearchParams>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (results, total) = search_service::search_items(&pool, &config, &query, ctx.user_id)?;
    Ok(dto::paginated_response(results, total, query.page.unwrap_or(1), query.page_size.unwrap_or(20)))
}

pub async fn suggestions(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let prefix = query.get("prefix").and_then(|p| p.as_str()).unwrap_or("");
    let limit = query.get("limit").and_then(|l| l.as_i64()).unwrap_or(10);
    let results = search_service::get_suggestions(&pool, prefix, limit)?;
    Ok(dto::success_response(results))
}

pub async fn trending(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let window = query.get("window_days").and_then(|w| w.as_i64()).unwrap_or(30);
    let results = search_service::get_trending(&pool, window)?;
    Ok(dto::success_response(results))
}

pub async fn history(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (entries, total) = search_service::get_history(&pool, ctx.user_id, &query)?;
    Ok(dto::paginated_response(entries, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn clear_history(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let before = query.get("before").and_then(|b| b.as_str()).and_then(|s| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok());
    let deleted = search_service::clear_history(&pool, ctx.user_id, before)?;
    Ok(dto::success_response(serde_json::json!({"deleted": deleted})))
}
