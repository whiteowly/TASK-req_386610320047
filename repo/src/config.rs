use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub session_secret: String,
    pub encryption_key: Vec<u8>,
    pub server_host: String,
    pub server_port: u16,
    pub session_inactivity_hours: i64,
    pub rate_limit_per_minute: u32,
    pub captcha_failure_threshold: u32,
    pub captcha_window_minutes: i64,
    pub auto_revert_idle_days: i64,
    pub search_history_max: i64,
    pub trending_window_days: i64,
    pub import_max_size_bytes: usize,
    pub audit_retention_years: i64,
    pub slow_query_threshold_ms: u64,
    pub alerts_spool_dir: String,
    pub data_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            session_secret: std::env::var("SESSION_SECRET").expect("SESSION_SECRET must be set"),
            encryption_key: {
                let key_b64 = std::env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY must be set");
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &key_b64)
                    .expect("ENCRYPTION_KEY must be valid base64")
            },
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            session_inactivity_hours: std::env::var("SESSION_INACTIVITY_HOURS")
                .unwrap_or_else(|_| "12".to_string())
                .parse()
                .unwrap_or(12),
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            captcha_failure_threshold: std::env::var("CAPTCHA_FAILURE_THRESHOLD")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            captcha_window_minutes: std::env::var("CAPTCHA_WINDOW_MINUTES")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .unwrap_or(15),
            auto_revert_idle_days: std::env::var("AUTO_REVERT_IDLE_DAYS")
                .unwrap_or_else(|_| "14".to_string())
                .parse()
                .unwrap_or(14),
            search_history_max: std::env::var("SEARCH_HISTORY_MAX")
                .unwrap_or_else(|_| "200".to_string())
                .parse()
                .unwrap_or(200),
            trending_window_days: std::env::var("TRENDING_WINDOW_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            import_max_size_bytes: std::env::var("IMPORT_MAX_SIZE_BYTES")
                .unwrap_or_else(|_| "10485760".to_string())
                .parse()
                .unwrap_or(10_485_760),
            audit_retention_years: std::env::var("AUDIT_RETENTION_YEARS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .unwrap_or(7),
            slow_query_threshold_ms: std::env::var("SLOW_QUERY_THRESHOLD_MS")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
            alerts_spool_dir: std::env::var("ALERTS_SPOOL_DIR")
                .unwrap_or_else(|_| "/app/alerts_spool".to_string()),
            data_dir: std::env::var("DATA_DIR").unwrap_or_else(|_| "/app/data".to_string()),
        }
    }

    pub fn create_db_pool(&self) -> DbPool {
        let manager = ConnectionManager::<PgConnection>::new(&self.database_url);
        Pool::builder()
            .max_size(10)
            .build(manager)
            .expect("Failed to create DB pool")
    }
}
