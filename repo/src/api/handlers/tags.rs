use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::DbPool;
use crate::models::{AuthContext, PaginationParams};
use crate::services::tags as tag_service;
use crate::api::dto;
use crate::errors::AppError;

pub async fn list_tags(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (tags, total) = tag_service::list_tags(&pool, &query, None)?;
    Ok(dto::paginated_response(tags, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn create_tag(req: HttpRequest, pool: web::Data<DbPool>, body: web::Json<tag_service::CreateTagRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" && ctx.role != "Author" { return Err(AppError::Forbidden("Administrator or Author required".to_string())); }
    let tag = tag_service::create_tag(&pool, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(tag))
}
