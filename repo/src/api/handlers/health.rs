use actix_web::{web, HttpResponse};
use crate::config::{AppConfig, DbPool};

pub async fn health_check(pool: web::Data<DbPool>, config: web::Data<AppConfig>) -> HttpResponse {
    let db_ok = pool.get().is_ok();
    let spool_ok = std::path::Path::new(&config.alerts_spool_dir).exists();
    let (total_requests, errors_4xx, errors_5xx) = crate::errors::get_error_rate_stats();

    let status = if db_ok { "healthy" } else { "degraded" };
    HttpResponse::Ok().json(serde_json::json!({
        "status": status,
        "components": {
            "database": if db_ok { "connected" } else { "disconnected" },
            "alert_spool": if spool_ok { "writable" } else { "unavailable" },
            "scheduler": "running"
        },
        "stats": {
            "total_requests": total_requests,
            "errors_4xx": errors_4xx,
            "errors_5xx": errors_5xx,
            "slow_request_threshold_ms": config.slow_query_threshold_ms
        }
    }))
}
