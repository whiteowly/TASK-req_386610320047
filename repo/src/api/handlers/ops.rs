use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use crate::config::AppConfig;
use crate::models::AuthContext;
use crate::api::dto;
use crate::errors::AppError;

pub async fn list_alerts(req: HttpRequest, config: web::Data<AppConfig>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    let alerts = crate::alerts::list_alerts(&config.alerts_spool_dir);
    Ok(dto::success_response(alerts))
}

pub async fn ack_alert(req: HttpRequest, config: web::Data<AppConfig>, pool: web::Data<crate::config::DbPool>, path: web::Path<String>, body: web::Json<serde_json::Value>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    crate::alerts::ack_alert(&config.alerts_spool_dir, &path).map_err(|e| AppError::NotFound(e))?;
    let note = body.get("note").and_then(|n| n.as_str());
    crate::audit::log_audit(&pool, Some(ctx.user_id), &ctx.username, "ACK_ALERT", "alert", None, None, Some(serde_json::json!({"alert_id": path.as_str()})), note, None);
    Ok(dto::success_response(serde_json::json!({"message": "Alert acknowledged"})))
}

/// Admin-only diagnostic: triggers a real Internal error through the standard
/// 5xx → alert-spool code path.  The response is a normal 500 envelope; the
/// side-effect is a spool record of type INTERNAL_ERROR (subject to the
/// existing 60-second throttle).
pub async fn diagnostic_trigger_error(req: HttpRequest, pool: web::Data<crate::config::DbPool>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    crate::audit::log_audit(&pool, Some(ctx.user_id), &ctx.username, "DIAGNOSTIC_TRIGGER", "alert", None, None, Some(serde_json::json!({"kind": "INTERNAL_ERROR"})), Some("Admin-triggered diagnostic error"), None);
    Err(AppError::Internal("diagnostic_test_trigger".to_string()))
}

/// Admin-only diagnostic: writes a JOB_FAILURE alert to the spool using the
/// same `alerts::write_alert` function the background scheduler uses.  This
/// lets integration tests verify the spool pipeline deterministically without
/// waiting for scheduler timing.
pub async fn diagnostic_trigger_job_failure(req: HttpRequest, config: web::Data<AppConfig>, pool: web::Data<crate::config::DbPool>) -> Result<HttpResponse, AppError> {
    let ctx = req.extensions().get::<AuthContext>().cloned().ok_or(AppError::Unauthorized("".to_string()))?;
    if ctx.role != "Administrator" { return Err(AppError::Forbidden("Administrator required".to_string())); }
    crate::alerts::write_alert(
        &config.alerts_spool_dir,
        "JOB_FAILURE",
        "Diagnostic job failure trigger",
        &serde_json::json!({"source": "diagnostic", "triggered_by": ctx.username}),
    );
    crate::audit::log_audit(&pool, Some(ctx.user_id), &ctx.username, "DIAGNOSTIC_TRIGGER", "alert", None, None, Some(serde_json::json!({"kind": "JOB_FAILURE"})), Some("Admin-triggered diagnostic job failure"), None);
    Ok(dto::created_response(serde_json::json!({"message": "JOB_FAILURE alert written to spool"})))
}
