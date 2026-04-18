use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::AuthContext;
use crate::services::analytics;
use crate::api::dto;
use crate::errors::AppError;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EventListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub event_type: Option<String>,
}

pub async fn create_event(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Json<analytics::CreateEventRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let event = analytics::create_event(&pool, &config, &body, Some(ctx.user_id))?;
    Ok(dto::created_response(event))
}

pub async fn list_events(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, query: web::Query<EventListQuery>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let pagination = crate::models::PaginationParams { page: query.page, page_size: query.page_size };
    let event_type = query.event_type.as_deref();
    let (events, total) = analytics::list_events(&pool, &config, &pagination, event_type)?;
    Ok(dto::paginated_response(events, total, pagination.page.unwrap_or(1), pagination.page_size()))
}
