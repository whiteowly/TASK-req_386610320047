use crate::config::DbPool;
use crate::crypto;
use crate::errors::{AppError, FieldError};
use crate::models::{UserResponse, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub active: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

pub fn list_users(pool: &DbPool, pagination: &PaginationParams, role_filter: Option<&str>) -> Result<(Vec<UserResponse>, i64), AppError> {
    let conn = &mut pool.get()?;

    let mut query = users::table.inner_join(roles::table).into_boxed();
    if let Some(role) = role_filter {
        query = query.filter(roles::name.eq(role));
    }

    let total: i64 = {
        let mut count_query = users::table.inner_join(roles::table).into_boxed();
        if let Some(role) = role_filter {
            count_query = count_query.filter(roles::name.eq(role));
        }
        count_query.count().get_result(conn)?
    };

    let results: Vec<(Uuid, String, Uuid, bool, chrono::NaiveDateTime, String)> = query
        .select((users::id, users::username, users::role_id, users::active, users::created_at, roles::name))
        .order(users::created_at.desc())
        .offset(pagination.offset())
        .limit(pagination.page_size())
        .load(conn)?;

    let responses: Vec<UserResponse> = results.into_iter().map(|(id, username, _, active, created_at, role)| {
        UserResponse { id, username, role, active, created_at }
    }).collect();

    Ok((responses, total))
}

pub fn create_user(pool: &DbPool, req: &CreateUserRequest, encryption_key: &[u8], actor_id: Uuid, actor_username: &str) -> Result<UserResponse, AppError> {
    let conn = &mut pool.get()?;

    if req.username.trim().is_empty() || req.password.len() < 8 {
        return Err(AppError::ValidationError(vec![
            FieldError { field: "password".to_string(), reason: "Password must be at least 8 characters".to_string() }
        ]));
    }

    let valid_roles = ["Administrator", "Author", "Reviewer", "Analyst"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Err(AppError::ValidationError(vec![
            FieldError { field: "role".to_string(), reason: format!("Role must be one of: {:?}", valid_roles) }
        ]));
    }

    // Get role ID
    let role_id: Uuid = roles::table
        .filter(roles::name.eq(&req.role))
        .select(roles::id)
        .first(conn)
        .map_err(|_| AppError::NotFound("Role not found".to_string()))?;

    let password_hash = crypto::hash_password(&req.password)
        .map_err(|e| AppError::Internal(e))?;

    let email_enc = req.email.as_ref().map(|e| crypto::encrypt_value(e, encryption_key)).transpose()
        .map_err(|e| AppError::Internal(e))?;
    let phone_enc = req.phone.as_ref().map(|p| crypto::encrypt_value(p, encryption_key)).transpose()
        .map_err(|e| AppError::Internal(e))?;

    let user_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(users::table)
        .values((
            users::id.eq(user_id),
            users::username.eq(req.username.trim()),
            users::password_hash.eq(&password_hash),
            users::email_encrypted.eq(email_enc),
            users::phone_encrypted.eq(phone_enc),
            users::role_id.eq(role_id),
            users::active.eq(req.active.unwrap_or(true)),
            users::created_at.eq(now),
            users::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "CREATE_USER", "user", Some(user_id), None, Some(serde_json::json!({"username": req.username, "role": req.role})), None, None);

    Ok(UserResponse { id: user_id, username: req.username.trim().to_string(), role: req.role.clone(), active: req.active.unwrap_or(true), created_at: now })
}

pub fn update_user(pool: &DbPool, user_id: Uuid, req: &UpdateUserRequest, encryption_key: &[u8], actor_id: Uuid, actor_username: &str) -> Result<UserResponse, AppError> {
    let conn = &mut pool.get()?;
    let now = Utc::now().naive_utc();

    // Check user exists
    let (username, old_role_id, old_active): (String, Uuid, bool) = users::table
        .filter(users::id.eq(user_id))
        .select((users::username, users::role_id, users::active))
        .first(conn)
        .map_err(|_| AppError::NotFound("User not found".to_string()))?;

    let mut new_role_id = old_role_id;
    let mut role_name = String::new();

    if let Some(ref role) = req.role {
        new_role_id = roles::table
            .filter(roles::name.eq(role))
            .select(roles::id)
            .first(conn)
            .map_err(|_| AppError::NotFound("Role not found".to_string()))?;
        role_name = role.clone();
    } else {
        role_name = roles::table.filter(roles::id.eq(old_role_id)).select(roles::name).first(conn)?;
    }

    // Build update
    diesel::update(users::table.filter(users::id.eq(user_id)))
        .set((
            users::role_id.eq(new_role_id),
            users::active.eq(req.active.unwrap_or(old_active)),
            users::updated_at.eq(now),
        ))
        .execute(conn)?;

    if let Some(ref email) = req.email {
        let enc = crypto::encrypt_value(email, encryption_key).map_err(|e| AppError::Internal(e))?;
        diesel::update(users::table.filter(users::id.eq(user_id)))
            .set(users::email_encrypted.eq(Some(enc)))
            .execute(conn)?;
    }
    if let Some(ref phone) = req.phone {
        let enc = crypto::encrypt_value(phone, encryption_key).map_err(|e| AppError::Internal(e))?;
        diesel::update(users::table.filter(users::id.eq(user_id)))
            .set(users::phone_encrypted.eq(Some(enc)))
            .execute(conn)?;
    }

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "UPDATE_USER", "user", Some(user_id), None, Some(serde_json::json!({"role": &role_name, "active": req.active})), None, None);

    Ok(UserResponse { id: user_id, username, role: role_name, active: req.active.unwrap_or(old_active), created_at: now })
}

pub fn reset_password(pool: &DbPool, user_id: Uuid, new_password: &str, actor_id: Uuid, actor_username: &str) -> Result<(), AppError> {
    if new_password.len() < 8 {
        return Err(AppError::ValidationError(vec![
            FieldError { field: "new_password".to_string(), reason: "Password must be at least 8 characters".to_string() }
        ]));
    }

    let conn = &mut pool.get()?;
    let hash = crypto::hash_password(new_password).map_err(|e| AppError::Internal(e))?;
    let now = Utc::now().naive_utc();

    let updated = diesel::update(users::table.filter(users::id.eq(user_id)))
        .set((users::password_hash.eq(&hash), users::updated_at.eq(now)))
        .execute(conn)?;

    if updated == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "RESET_PASSWORD", "user", Some(user_id), None, None, None, None);
    Ok(())
}
