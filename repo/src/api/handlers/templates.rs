use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::DbPool;
use crate::models::{AuthContext, PaginationParams};
use crate::services::templates as template_service;
use crate::api::dto;
use crate::errors::AppError;

pub async fn create_template(req: HttpRequest, pool: web::Data<DbPool>, body: web::Json<template_service::CreateTemplateRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let tmpl = template_service::create_template(&pool, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(tmpl))
}

pub async fn list_templates(req: HttpRequest, pool: web::Data<DbPool>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (templates, total) = template_service::list_templates(&pool, &query)?;
    Ok(dto::paginated_response(templates, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_template(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let tmpl = template_service::get_template(&pool, *path)?;
    Ok(dto::success_response(tmpl))
}

pub async fn create_template_version(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, body: web::Json<template_service::CreateTemplateVersionRequest>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let version = template_service::create_template_version(&pool, *path, &body, ctx.user_id, &ctx.username)?;
    Ok(dto::created_response(version))
}

pub async fn list_template_versions(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<uuid::Uuid>, query: web::Query<PaginationParams>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (versions, total) = template_service::list_template_versions(&pool, *path, &query)?;
    Ok(dto::paginated_response(versions, total, query.page.unwrap_or(1), query.page_size()))
}

pub async fn get_template_version(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<(uuid::Uuid, uuid::Uuid)>) -> Result<HttpResponse, AppError> {
    let _ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    let (template_id, version_id) = path.into_inner();
    let version = template_service::get_template_version(&pool, template_id, version_id)?;
    Ok(dto::success_response(version))
}

pub async fn activate_version(req: HttpRequest, pool: web::Data<DbPool>, path: web::Path<(uuid::Uuid, uuid::Uuid)>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let (template_id, version_id) = path.into_inner();
    template_service::activate_template_version(&pool, template_id, version_id, ctx.user_id, &ctx.username)?;
    Ok(dto::success_response(serde_json::json!({"message": "Version activated"})))
}
