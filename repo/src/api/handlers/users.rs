use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::{AuthContext, PaginationParams};
use crate::services::users as user_service;
use crate::api::dto;
use crate::errors::AppError;

fn require_admin(req: &HttpRequest) -> Result<AuthContext, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("Not authenticated".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator access required".to_string())); }
    Ok(ctx)
}

pub async fn list_users(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = require_admin(&req)?;
    let (users, total) = user_service::list_users(&pool, &query, None)?;
    Ok(dto::paginated_response(users, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn create_user(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Json<user_service::CreateUserRequest>) -> Result<HttpResponse, AppError> {
    let ctx = require_admin(&req)?;
    let user = user_service::create_user(&pool, &body, &config.encryption_key, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(user))
}

pub async fn update_user(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, path: web::Path<uuid::Uuid>, body: web::Json<user_service::UpdateUserRequest>) -> Result<HttpResponse, AppError> {
    let ctx = require_admin(&req)?;
    let user = user_service::update_user(&pool, *path, &body, &config.encryption_key, ctx.user_id, &ctx.username)?;
    Ok(dto::success_response(user))
}

pub async fn reset_password(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<user_service::ResetPasswordRequest>) -> Result<HttpResponse, AppError> {
    let ctx = require_admin(&req)?;
    user_service::reset_password(&pool, *path, &body.new_password, ctx.user_id, &ctx.username)?;
    Ok(dto::success_response(serde_json::json!({"message": "Password reset successful"})))
}
