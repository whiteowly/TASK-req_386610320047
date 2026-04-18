use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::OnceLock;

// Global error-rate counters (lightweight, lock-free)
static ERROR_COUNT_4XX: AtomicU64 = AtomicU64::new(0);
static ERROR_COUNT_5XX: AtomicU64 = AtomicU64::new(0);
static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);

// Alert spool directory (set once at startup)
static ALERTS_SPOOL_DIR: OnceLock<String> = OnceLock::new();
// Rate-limit: unix timestamp of last 5xx alert to avoid spam (max one per 60s)
static LAST_5XX_ALERT_TS: AtomicI64 = AtomicI64::new(0);

pub fn set_alerts_spool_dir(dir: String) {
    ALERTS_SPOOL_DIR.set(dir).ok();
}

pub fn increment_request_count() {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn get_error_rate_stats() -> (u64, u64, u64) {
    (
        REQUEST_COUNT.load(Ordering::Relaxed),
        ERROR_COUNT_4XX.load(Ordering::Relaxed),
        ERROR_COUNT_5XX.load(Ordering::Relaxed),
    )
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
    pub request_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FieldError {
    pub field: String,
    pub reason: String,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    ValidationError(Vec<FieldError>),
    PayloadTooLarge(String),
    UnsupportedMedia(String),
    RateLimited(String),
    CaptchaRequired(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::ValidationError(_) => write!(f, "Validation error"),
            AppError::PayloadTooLarge(msg) => write!(f, "Payload too large: {}", msg),
            AppError::UnsupportedMedia(msg) => write!(f, "Unsupported media: {}", msg),
            AppError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            AppError::CaptchaRequired(msg) => write!(f, "CAPTCHA required: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

// Thread-local request_id mechanism
thread_local! {
    static REQUEST_ID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub fn set_request_id(id: String) {
    REQUEST_ID.with(|r| *r.borrow_mut() = id);
}

pub fn get_request_id() -> String {
    REQUEST_ID.with(|r| r.borrow().clone())
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let request_id = get_request_id();
        let (status, code, message, details) = match self {
            AppError::BadRequest(msg) => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                msg.clone(),
                None,
            ),
            AppError::Unauthorized(msg) => (
                actix_web::http::StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                msg.clone(),
                None,
            ),
            AppError::Forbidden(msg) => (
                actix_web::http::StatusCode::FORBIDDEN,
                "FORBIDDEN",
                msg.clone(),
                None,
            ),
            AppError::NotFound(msg) => (
                actix_web::http::StatusCode::NOT_FOUND,
                "NOT_FOUND",
                msg.clone(),
                None,
            ),
            AppError::Conflict(msg) => (
                actix_web::http::StatusCode::CONFLICT,
                "CONFLICT",
                msg.clone(),
                None,
            ),
            AppError::ValidationError(errs) => (
                actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                "One or more fields failed validation".to_string(),
                Some(errs.clone()),
            ),
            AppError::PayloadTooLarge(msg) => (
                actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                msg.clone(),
                None,
            ),
            AppError::UnsupportedMedia(msg) => (
                actix_web::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "UNSUPPORTED_MEDIA_TYPE",
                msg.clone(),
                None,
            ),
            AppError::RateLimited(msg) => (
                actix_web::http::StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                msg.clone(),
                None,
            ),
            AppError::CaptchaRequired(msg) => (
                actix_web::http::StatusCode::TOO_MANY_REQUESTS,
                "CAPTCHA_REQUIRED",
                msg.clone(),
                None,
            ),
            AppError::Internal(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An internal error occurred".to_string(),
                None,
            ),
        };

        // Log internal errors but don't expose details
        if let AppError::Internal(msg) = self {
            if !msg.is_empty() {
                log::error!("Internal error [{}]: {}", request_id, self);
            }
        }

        // Track error rates
        if status.is_client_error() {
            ERROR_COUNT_4XX.fetch_add(1, Ordering::Relaxed);
        } else if status.is_server_error() {
            ERROR_COUNT_5XX.fetch_add(1, Ordering::Relaxed);
            // Write to alert spool (rate-limited: max one alert per 60 seconds)
            if let Some(spool_dir) = ALERTS_SPOOL_DIR.get() {
                let now_ts = chrono::Utc::now().timestamp();
                let last = LAST_5XX_ALERT_TS.load(Ordering::Relaxed);
                if now_ts - last >= 60 {
                    if LAST_5XX_ALERT_TS.compare_exchange(last, now_ts, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                        let detail = match self {
                            AppError::Internal(msg) => msg.clone(),
                            _ => "Server error".to_string(),
                        };
                        crate::alerts::write_alert(
                            spool_dir,
                            "INTERNAL_ERROR",
                            "Internal server error occurred",
                            &serde_json::json!({"request_id": &request_id, "detail": detail}),
                        );
                    }
                }
            }
        }

        HttpResponse::build(status).json(ApiError {
            error: ApiErrorBody {
                code: code.to_string(),
                message,
                details,
                request_id,
            },
        })
    }
}

impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => {
                AppError::NotFound("Resource not found".to_string())
            }
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                info,
            ) => AppError::Conflict(format!("Duplicate: {}", info.message())),
            _ => {
                log::error!("Database error: {:?}", err);
                AppError::Internal("Database error".to_string())
            }
        }
    }
}

impl From<r2d2::Error> for AppError {
    fn from(err: r2d2::Error) -> Self {
        log::error!("Connection pool error: {:?}", err);
        AppError::Internal("Database connection error".to_string())
    }
}
