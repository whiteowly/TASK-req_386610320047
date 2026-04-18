use crate::config::DbPool;
use crate::errors::get_request_id;
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

pub fn log_audit(
    pool: &DbPool,
    actor_id: Option<Uuid>,
    actor_username: &str,
    action: &str,
    object_type: &str,
    object_id: Option<Uuid>,
    before_state: Option<Value>,
    after_state: Option<Value>,
    reason: Option<&str>,
    ip_address: Option<&str>,
) -> Uuid {
    use crate::schema::audits;

    let audit_id = Uuid::new_v4();
    let request_id = get_request_id();
    let now = chrono::Utc::now();

    // Redact sensitive fields from before/after state
    let before_state = before_state.map(|v| redact_sensitive(&v));
    let after_state = after_state.map(|v| redact_sensitive(&v));

    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to get DB connection for audit: {}", e);
            return audit_id;
        }
    };
    let conn = &mut { conn };

    diesel::insert_into(audits::table)
        .values((
            audits::id.eq(audit_id),
            audits::actor_id.eq(actor_id),
            audits::actor_username.eq(actor_username),
            audits::action.eq(action),
            audits::object_type.eq(object_type),
            audits::object_id.eq(object_id),
            audits::before_state.eq(before_state),
            audits::after_state.eq(after_state),
            audits::reason.eq(reason),
            audits::request_id.eq(&request_id),
            audits::ip_address.eq(ip_address),
            audits::created_at.eq(now.naive_utc()),
            audits::updated_at.eq(now.naive_utc()),
        ))
        .execute(conn)
        .unwrap_or_else(|e| {
            log::error!("Failed to write audit record: {}", e);
            0
        });

    audit_id
}

pub fn redact_event_payload(value: &Value) -> Value {
    redact_sensitive(value)
}

fn redact_sensitive(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                let lower_k = k.to_lowercase();
                if lower_k.contains("password")
                    || lower_k.contains("secret")
                    || lower_k.contains("token")
                    || lower_k.contains("encrypted")
                    || lower_k.contains("hash")
                {
                    new_map.insert(k.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    new_map.insert(k.clone(), redact_sensitive(v));
                }
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(redact_sensitive).collect()),
        other => other.clone(),
    }
}
