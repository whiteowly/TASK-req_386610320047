use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::{AuthContext, PaginationParams};
use crate::services::items as item_service;
use crate::api::dto;
use crate::errors::AppError;

pub async fn create_item(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Json<item_service::CreateItemRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Author" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Author or Administrator required".to_string())); }
    let item = item_service::create_item(&pool, &config, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(item))
}

pub async fn list_items(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, query: web::Query<item_service::ItemListParams>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (items, total) = item_service::list_items(&pool, &config, &query, &ctx)?;
    Ok(dto::paginated_response(items, total, query.page.unwrap_or(1), query.page_size.unwrap_or(20)))
}

pub async fn get_item(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let item = item_service::get_item(&pool, *path, &ctx)?;
    Ok(dto::success_response(item))
}

pub async fn update_item(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<item_service::UpdateItemRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Author" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Author or Administrator required".to_string())); }
    let version = item_service::update_item(&pool, *path, &body, &ctx)?;
    Ok(dto::success_response(version))
}

pub async fn list_versions(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    // Enforce object-level auth: Authors can only see their own item versions
    item_service::get_item(&pool, *path, &ctx)?;
    let (versions, total) = item_service::list_item_versions(&pool, *path, &query)?;
    Ok(dto::paginated_response(versions, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_version(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<(uuid::Uuid, uuid::Uuid)>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (item_id, version_id) = path.into_inner();
    // Enforce object-level auth: Authors can only see their own item versions
    item_service::get_item(&pool, item_id, &ctx)?;
    let version = item_service::get_item_version(&pool, item_id, version_id)?;
    Ok(dto::success_response(version))
}

pub async fn rollback(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<item_service::RollbackRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Author" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Author or Administrator required".to_string())); }
    let version = item_service::rollback_item(&pool, *path, &body, &ctx)?;
    Ok(dto::success_response(version))
}

pub async fn transition(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<item_service::TransitionRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let item = item_service::transition_item(&pool, *path, &body, &ctx)?;
    Ok(dto::success_response(item))
}

pub async fn publish(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<item_service::PublishRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Author" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Author or Administrator required".to_string())); }
    let item = item_service::publish_item(&pool, *path, &body, &ctx)?;
    Ok(dto::success_response(item))
}
