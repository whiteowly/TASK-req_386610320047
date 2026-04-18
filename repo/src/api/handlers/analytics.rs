use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::{AppConfig, DbPool};
use crate::models::AuthContext;
use crate::services::analytics::{self, AnalyticsFilter};
use crate::api::dto;
use crate::errors::AppError;

pub async fn get_kpis(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<AnalyticsFilter>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let kpis = analytics::get_kpis(&pool, &query)?;
    Ok(dto::success_response(kpis))
}

pub async fn get_operational(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<AnalyticsFilter>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let ops = analytics::get_operational(&pool, &query)?;
    Ok(dto::success_response(ops))
}

pub async fn create_export(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Analyst" && ctx.role != "Administrator" { return Err(AppError::Forbidden("Access denied".to_string())); }
    let export = analytics::create_analytics_export(&pool, &config, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(export))
}
