use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::DbPool;
use crate::models::{AuthContext, PaginationParams};
use crate::services::channels as channel_service;
use crate::api::dto;
use crate::errors::AppError;

pub async fn list_channels(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (channels, total) = channel_service::list_channels(&pool, &query, None)?;
    Ok(dto::paginated_response(channels, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn create_channel(req: HttpRequest, pool: web::Data<DbPool>, body: web::Json<channel_service::CreateChannelRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let channel = channel_service::create_channel(&pool, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(channel))
}

pub async fn update_channel(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<channel_service::UpdateChannelRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let channel = channel_service::update_channel(&pool, *path, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::success_response(channel))
}
