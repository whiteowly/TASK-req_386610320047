use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use actix_web::cookie::{Cookie, SameSite};
use crate::config::{AppConfig, DbPool};
use crate::models::AuthContext;
use crate::services::auth as auth_service;
use crate::api::dto;

pub async fn login(req: HttpRequest, pool: web::Data<DbPool>, config: web::Data<AppConfig>, body: web::Json<auth_service::LoginRequest>) -> Result<HttpResponse, crate::errors::AppError> {
    let ip = req.peer_addr().map(|a| a.ip().to_string());
    let result = auth_service::login(&pool, &config, &body, ip.as_deref(), config.captcha_failure_threshold, config.captcha_window_minutes)?;

    let cookie = Cookie::build("ko_session", &result.token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(actix_web::cookie::time::Duration::hours(12))
        .finish();

    let mut response = HttpResponse::Ok().json(serde_json::json!({
        "data": {"token": result.token, "user": result.user},
        "meta": {"request_id": crate::errors::get_request_id()}
    }));
    response.add_cookie(&cookie).ok();
    Ok(response)
}

pub async fn captcha_challenge(pool: web::Data<DbPool>, body: web::Json<serde_json::Value>) -> Result<HttpResponse, crate::errors::AppError> {
    let username = body.get("username").and_then(|u| u.as_str()).unwrap_or("");
    let result = auth_service::create_captcha(&pool, username)?;
    Ok(dto::success_response(result))
}

pub async fn logout(req: HttpRequest, pool: web::Data<DbPool>) -> Result<HttpResponse, crate::errors::AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(crate::errors::AppError::Unauthorized("Not authenticated".to_string()))?;
    auth_service::logout(&pool, ctx.session_id, ctx.user_id, &ctx.username)?;
    Ok(dto::success_response(serde_json::json!({"message": "Logged out"})))
}

pub async fn me(req: HttpRequest) -> Result<HttpResponse, crate::errors::AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(crate::errors::AppError::Unauthorized("Not authenticated".to_string()))?;
    Ok(dto::success_response(serde_json::json!({
        "id": ctx.user_id, "username": ctx.username, "role": ctx.role
    })))
}
