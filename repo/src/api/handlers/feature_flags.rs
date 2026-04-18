use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::DbPool;
use crate::models::{AuthContext, PaginationParams};
use crate::services::feature_flags;
use crate::api::dto;
use crate::errors::AppError;

pub async fn list_flags(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" && ctx.role != "Analyst" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let (flags, total) = feature_flags::list_flags(&pool, &query)?;
    Ok(dto::paginated_response(flags, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn create_flag(req: HttpRequest, pool: web::Data<DbPool>, body: web::Json<feature_flags::CreateFlagRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let flag = feature_flags::create_flag(&pool, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(flag))
}

pub async fn update_flag(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<String>, body: web::Json<feature_flags::UpdateFlagRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let flag = feature_flags::update_flag(&pool, &path, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::success_response(flag))
}
