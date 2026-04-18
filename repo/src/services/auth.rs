use crate::config::{AppConfig, DbPool};
use crate::crypto;
use crate::db_instrumentation::timed_query;
use crate::errors::AppError;
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub captcha_id: Option<Uuid>,
    pub captcha_answer: Option<String>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct CaptchaResponse {
    pub captcha_id: Uuid,
    pub challenge_prompt: String,
    pub expires_at: chrono::NaiveDateTime,
}

pub fn seed_roles(pool: &DbPool) {
    let conn = &mut pool.get().expect("Failed to get connection for role seeding");
    let role_names = vec!["Administrator", "Author", "Reviewer", "Analyst"];
    let now = Utc::now().naive_utc();

    for name in role_names {
        let exists: bool = diesel::select(diesel::dsl::exists(
            roles::table.filter(roles::name.eq(name))
        )).get_result(conn).unwrap_or(false);

        if !exists {
            diesel::insert_into(roles::table)
                .values((
                    roles::id.eq(Uuid::new_v4()),
                    roles::name.eq(name),
                    roles::created_at.eq(now),
                    roles::updated_at.eq(now),
                ))
                .execute(conn)
                .ok();
        }
    }
}

pub fn seed_demo_users(pool: &DbPool) {
    let conn = &mut pool.get().expect("Failed to get connection for user seeding");
    let now = Utc::now().naive_utc();

    let demo_users = vec![
        ("admin", "Administrator"),
        ("author", "Author"),
        ("reviewer", "Reviewer"),
        ("analyst", "Analyst"),
    ];

    for (username, role_name) in demo_users {
        let exists: bool = diesel::select(diesel::dsl::exists(
            users::table.filter(users::username.eq(username))
        )).get_result(conn).unwrap_or(false);

        if !exists {
            let role_id: Option<Uuid> = roles::table
                .filter(roles::name.eq(role_name))
                .select(roles::id)
                .first(conn)
                .ok();

            if let Some(role_id) = role_id {
                let password_hash = crypto::hash_password("changeme123!")
                    .unwrap_or_else(|_| "invalid".to_string());

                diesel::insert_into(users::table)
                    .values((
                        users::id.eq(Uuid::new_v4()),
                        users::username.eq(username),
                        users::password_hash.eq(&password_hash),
                        users::role_id.eq(role_id),
                        users::active.eq(true),
                        users::created_at.eq(now),
                        users::updated_at.eq(now),
                    ))
                    .execute(conn)
                    .ok();

                log::info!("Seeded demo user: {} ({})", username, role_name);
            }
        }
    }
}

pub fn login(pool: &DbPool, config: &AppConfig, req: &LoginRequest, ip: Option<&str>, captcha_threshold: u32, captcha_window_minutes: i64) -> Result<LoginResponse, AppError> {
    let conn = &mut pool.get()?;
    let threshold = config.slow_query_threshold_ms;

    // Check if CAPTCHA is required
    if is_captcha_required(conn, &req.username, ip, captcha_threshold, captcha_window_minutes) {
        match (&req.captcha_id, &req.captcha_answer) {
            (Some(cid), Some(answer)) => {
                if !verify_captcha(conn, *cid, answer, &req.username) {
                    return Err(AppError::CaptchaRequired("Invalid CAPTCHA answer".to_string()));
                }
            }
            _ => {
                return Err(AppError::CaptchaRequired("CAPTCHA verification required after repeated failed login attempts".to_string()));
            }
        }
    }

    // Find user (case-insensitive via CITEXT column - parameterized, no raw SQL)
    let username_lower = req.username.to_lowercase();
    let user_result: Result<(Uuid, String, String, Uuid, bool), _> = timed_query("login_user_lookup", threshold, || {
        users::table
            .filter(users::username.eq(&username_lower))
            .select((users::id, users::username, users::password_hash, users::role_id, users::active))
            .first(conn)
    });

    let (user_id, username, password_hash, role_id, active) = match user_result {
        Ok(u) => u,
        Err(_) => {
            record_login_attempt(conn, &req.username, ip, false);
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }
    };

    if !active {
        record_login_attempt(conn, &req.username, ip, false);
        return Err(AppError::Unauthorized("Account is deactivated".to_string()));
    }

    if !crypto::verify_password(&req.password, &password_hash) {
        record_login_attempt(conn, &req.username, ip, false);
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Record success
    record_login_attempt(conn, &username, ip, true);

    // Get role name
    let role_name: String = roles::table
        .filter(roles::id.eq(role_id))
        .select(roles::name)
        .first(conn)
        .map_err(|_| AppError::Internal("Role lookup failed".to_string()))?;

    // Create session
    let token = crypto::generate_session_token();
    let token_hash = crypto::hash_token(&token);
    let now = Utc::now().naive_utc();
    let expires = now + chrono::Duration::hours(12);

    timed_query("login_session_insert", threshold, || {
        diesel::insert_into(sessions::table)
            .values((
                sessions::id.eq(Uuid::new_v4()),
                sessions::user_id.eq(user_id),
                sessions::token_hash.eq(&token_hash),
                sessions::last_activity_at.eq(now),
                sessions::expires_at.eq(expires),
                sessions::created_at.eq(now),
                sessions::updated_at.eq(now),
            ))
            .execute(conn)
    })
    .map_err(|e| AppError::Internal(format!("Session creation failed: {}", e)))?;

    // Audit
    crate::audit::log_audit(
        pool, Some(user_id), &username, "LOGIN", "session", None,
        None, Some(serde_json::json!({"role": &role_name})), None, ip,
    );

    Ok(LoginResponse {
        token,
        user: UserInfo { id: user_id, username, role: role_name },
    })
}

pub fn logout(pool: &DbPool, session_id: Uuid, user_id: Uuid, username: &str) -> Result<(), AppError> {
    let conn = &mut pool.get()?;
    diesel::delete(sessions::table.filter(sessions::id.eq(session_id)))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(user_id), username, "LOGOUT", "session", Some(session_id), None, None, None, None);
    Ok(())
}

pub fn create_captcha(pool: &DbPool, username: &str) -> Result<CaptchaResponse, AppError> {
    let conn = &mut pool.get()?;

    let a = rand::random::<u8>() % 50 + 1;
    let b = rand::random::<u8>() % 50 + 1;
    let answer = (a as u32 + b as u32).to_string();
    let prompt = format!("What is {} + {}?", a, b);

    let captcha_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    let expires = now + chrono::Duration::minutes(5);

    diesel::insert_into(captcha_challenges::table)
        .values((
            captcha_challenges::id.eq(captcha_id),
            captcha_challenges::username.eq(username),
            captcha_challenges::challenge_type.eq("arithmetic"),
            captcha_challenges::challenge_prompt.eq(&prompt),
            captcha_challenges::expected_answer.eq(&answer),
            captcha_challenges::expires_at.eq(expires),
            captcha_challenges::used.eq(false),
            captcha_challenges::created_at.eq(now),
        ))
        .execute(conn)?;

    Ok(CaptchaResponse {
        captcha_id,
        challenge_prompt: prompt,
        expires_at: expires,
    })
}

fn verify_captcha(conn: &mut PgConnection, captcha_id: Uuid, answer: &str, username: &str) -> bool {
    let result: Option<(String, String, chrono::NaiveDateTime, bool)> = captcha_challenges::table
        .filter(captcha_challenges::id.eq(captcha_id))
        .select((
            captcha_challenges::expected_answer,
            captcha_challenges::username,
            captcha_challenges::expires_at,
            captcha_challenges::used,
        ))
        .first(conn)
        .ok();

    match result {
        Some((expected, challenge_username, expires, used)) => {
            // Must not be used, not expired, and bound to the same username
            if used || Utc::now().naive_utc() > expires {
                return false;
            }
            if challenge_username.to_lowercase() != username.to_lowercase() {
                return false;
            }
            diesel::update(captcha_challenges::table.filter(captcha_challenges::id.eq(captcha_id)))
                .set(captcha_challenges::used.eq(true))
                .execute(conn)
                .ok();
            expected == answer
        }
        None => false,
    }
}

fn is_captcha_required(conn: &mut PgConnection, username: &str, _ip: Option<&str>, threshold: u32, window_minutes: i64) -> bool {
    let since = Utc::now().naive_utc() - chrono::Duration::minutes(window_minutes);

    let failure_count: i64 = login_attempts::table
        .filter(login_attempts::username.eq(username.to_lowercase()))
        .filter(login_attempts::success.eq(false))
        .filter(login_attempts::created_at.ge(since))
        .count()
        .get_result(conn)
        .unwrap_or(0);

    failure_count >= threshold as i64
}

fn record_login_attempt(conn: &mut PgConnection, username: &str, ip: Option<&str>, success: bool) {
    diesel::insert_into(login_attempts::table)
        .values((
            login_attempts::id.eq(Uuid::new_v4()),
            login_attempts::username.eq(username.to_lowercase()),
            login_attempts::ip_address.eq(ip),
            login_attempts::success.eq(success),
            login_attempts::created_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)
        .ok();
}
