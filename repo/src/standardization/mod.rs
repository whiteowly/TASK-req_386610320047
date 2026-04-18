use crate::config::{AppConfig, DbPool};
use chrono::Utc;
use chrono_tz::America::New_York;
use diesel::prelude::*;
use uuid::Uuid;

pub fn execute_job(
    pool: &DbPool,
    _config: &AppConfig,
    job_id: Uuid,
    mapping_version_id: Uuid,
) -> Result<(), String> {
    use crate::schema::*;

    let conn = &mut pool.get().map_err(|e| e.to_string())?;

    // Mark job as running
    diesel::update(
        standardization_jobs::table.filter(standardization_jobs::id.eq(job_id)),
    )
    .set((
        standardization_jobs::status.eq("running"),
        standardization_jobs::started_at.eq(Some(Utc::now().naive_utc())),
        standardization_jobs::updated_at.eq(Utc::now().naive_utc()),
    ))
    .execute(conn)
    .map_err(|e| e.to_string())?;

    // Get mapping version rules
    let mapping_version: (
        Uuid,
        serde_json::Value,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
    ) = schema_mapping_versions::table
        .filter(schema_mapping_versions::id.eq(mapping_version_id))
        .select((
            schema_mapping_versions::id,
            schema_mapping_versions::mapping_rules,
            schema_mapping_versions::explicit_defaults,
            schema_mapping_versions::unit_rules,
            schema_mapping_versions::timezone_rules,
            schema_mapping_versions::fingerprint_keys,
            schema_mapping_versions::pii_fields,
        ))
        .first(conn)
        .map_err(|e| e.to_string())?;

    let (
        _mv_id,
        _mapping_rules,
        explicit_defaults,
        unit_rules,
        tz_rules,
        fingerprint_keys,
        pii_fields,
    ) = mapping_version;

    // Get source items
    let source_items: Vec<(Uuid, String, Option<serde_json::Value>)> = {
        let source_filters: Option<serde_json::Value> = standardization_jobs::table
            .filter(standardization_jobs::id.eq(job_id))
            .select(standardization_jobs::source_filters)
            .first(conn)
            .map_err(|e| e.to_string())?;

        let mut query = item_versions::table
            .inner_join(
                items::table
                    .on(items::current_version_id.eq(item_versions::id.nullable())),
            )
            .select((
                items::id,  // items.id, not item_versions.id — FK references items table
                item_versions::title,
                item_versions::fields.nullable(),
            ))
            .into_boxed();

        if let Some(filters) = &source_filters {
            if let Some(status) = filters.get("status").and_then(|s| s.as_str()) {
                query = query.filter(items::status.eq(status));
            }
        }

        query.limit(10000).load(conn).map_err(|e| e.to_string())?
    };

    // Create standardized model
    let model_id = Uuid::new_v4();
    let model_version = 1;

    diesel::insert_into(standardized_models::table)
        .values((
            standardized_models::id.eq(model_id),
            standardized_models::job_id.eq(job_id),
            standardized_models::mapping_version_id.eq(mapping_version_id),
            standardized_models::version_number.eq(model_version),
            standardized_models::record_count.eq(source_items.len() as i32),
            standardized_models::created_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)
        .map_err(|e| e.to_string())?;

    let pii_field_names: Vec<String> = pii_fields
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let fp_keys: Vec<String> = fingerprint_keys
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    // Collect all numeric values per field for z-score computation
    let mut numeric_collections: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
    for (_, _, fields_opt) in &source_items {
        if let Some(obj) = fields_opt.as_ref().and_then(|f| f.as_object()) {
            for (key, value) in obj {
                if let Some(num) = value.as_f64() {
                    numeric_collections.entry(key.clone()).or_default().push(num);
                }
            }
        }
    }

    // Compute mean and stddev per numeric field
    let field_stats: std::collections::HashMap<String, (f64, f64)> = numeric_collections
        .iter()
        .filter_map(|(key, values)| {
            if values.len() < 2 { return None; }
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
            let stddev = variance.sqrt();
            if stddev > 0.0 { Some((key.clone(), (mean, stddev))) } else { None }
        })
        .collect();

    // Collect fingerprints for dedup detection within this batch
    let mut seen_fingerprints: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Also load existing fingerprints from DB for cross-batch dedup
    let existing_fps: Vec<String> = standardized_records::table
        .select(standardized_records::fingerprint)
        .load(conn)
        .unwrap_or_default();
    for fp in &existing_fps {
        seen_fingerprints.insert(fp.clone());
    }

    let mut processed = 0i32;
    let mut failed = 0i32;

    for (item_ver_id, title, fields_opt) in &source_items {
        let fields = fields_opt.clone().unwrap_or(serde_json::json!({}));

        match standardize_record(
            title,
            &fields,
            &explicit_defaults,
            &unit_rules,
            &tz_rules,
            &pii_field_names,
            &fp_keys,
            &field_stats,
            &mut seen_fingerprints,
        ) {
            Ok((standardized, fingerprint, outlier_flags, is_dup, raw_values)) => {
                // source_item_id is items.id (not item_versions.id)
                match diesel::insert_into(standardized_records::table)
                    .values((
                        standardized_records::id.eq(Uuid::new_v4()),
                        standardized_records::model_id.eq(model_id),
                        standardized_records::source_item_id.eq(Some(*item_ver_id)),
                        standardized_records::fingerprint.eq(&fingerprint),
                        standardized_records::raw_values.eq(&raw_values),
                        standardized_records::standardized_values.eq(&standardized),
                        standardized_records::outlier_flags.eq(&outlier_flags),
                        standardized_records::is_duplicate.eq(is_dup),
                        standardized_records::created_at.eq(Utc::now().naive_utc()),
                    ))
                    .execute(conn)
                {
                    Ok(_) => processed += 1,
                    Err(e) => {
                        log::error!("Failed to insert standardized record for item {}: {}", item_ver_id, e);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to standardize record {}: {}", item_ver_id, e);
                failed += 1;
            }
        }
    }

    let final_status = if failed == 0 {
        "succeeded"
    } else if processed > 0 {
        "partial_succeeded"
    } else {
        "failed"
    };

    diesel::update(
        standardization_jobs::table.filter(standardization_jobs::id.eq(job_id)),
    )
    .set((
        standardization_jobs::status.eq(final_status),
        standardization_jobs::processed_records.eq(Some(processed)),
        standardization_jobs::failed_records.eq(Some(failed)),
        standardization_jobs::total_records.eq(Some(source_items.len() as i32)),
        standardization_jobs::completed_at.eq(Some(Utc::now().naive_utc())),
        standardization_jobs::updated_at.eq(Utc::now().naive_utc()),
    ))
    .execute(conn)
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn standardize_record(
    title: &str,
    fields: &serde_json::Value,
    explicit_defaults: &Option<serde_json::Value>,
    unit_rules: &Option<serde_json::Value>,
    tz_rules: &Option<serde_json::Value>,
    pii_fields: &[String],
    fingerprint_keys: &[String],
    field_stats: &std::collections::HashMap<String, (f64, f64)>,
    seen_fingerprints: &mut std::collections::HashSet<String>,
) -> Result<(serde_json::Value, String, serde_json::Value, bool, serde_json::Value), String> {
    let mut standardized = serde_json::Map::new();
    let raw_values = fields.clone();
    let mut outlier_map = serde_json::Map::new();

    // Normalize title
    standardized.insert(
        "title".to_string(),
        serde_json::Value::String(title.trim().to_string()),
    );

    // Process fields
    if let Some(obj) = fields.as_object() {
        for (key, value) in obj {
            // Never impute PII fields
            if pii_fields.contains(key) {
                standardized.insert(key.clone(), value.clone());
                continue;
            }

            // Apply explicit defaults for missing values (null only)
            if value.is_null() {
                if let Some(defaults) = explicit_defaults {
                    if let Some(default_val) = defaults.get(key) {
                        standardized.insert(key.clone(), default_val.clone());
                        continue;
                    }
                }
                standardized.insert(key.clone(), serde_json::Value::Null);
                continue;
            }

            // Timezone normalization for timestamp-like string fields
            let tz_normalized = normalize_timestamp(value, key, tz_rules);

            // Unit normalization
            let normalized_value = if let Some(rules) = unit_rules {
                if let Some(rule) = rules.get(key) {
                    normalize_unit(&tz_normalized, rule)
                } else {
                    tz_normalized
                }
            } else {
                tz_normalized
            };

            standardized.insert(key.clone(), normalized_value);
        }
    }

    // Compute deterministic fingerprint from normalized text + key fields
    let fingerprint = compute_fingerprint(title, fields, fingerprint_keys);

    // Dedup: check if this fingerprint was already seen
    let is_dup = !seen_fingerprints.insert(fingerprint.clone());

    // Z-score outlier detection for numeric fields using |z| >= 3
    if let Some(obj) = fields.as_object() {
        for (key, value) in obj {
            if let Some(num) = value.as_f64() {
                if let Some((mean, stddev)) = field_stats.get(key) {
                    let z_score = (num - mean) / stddev;
                    if z_score.abs() >= 3.0 {
                        outlier_map.insert(
                            key.clone(),
                            serde_json::json!({
                                "flagged": true,
                                "value": num,
                                "z_score": (z_score * 100.0).round() / 100.0,
                                "mean": (mean * 100.0).round() / 100.0,
                                "stddev": (stddev * 100.0).round() / 100.0,
                                "reason": "z_score_ge_3"
                            }),
                        );
                    }
                }
            }
        }
    }

    Ok((
        serde_json::Value::Object(standardized),
        fingerprint,
        serde_json::Value::Object(outlier_map),
        is_dup,
        raw_values,
    ))
}

/// Normalize timestamps to America/New_York while preserving raw value.
/// If tz_rules specifies a field as a timestamp field, parse and re-format in NY timezone.
fn normalize_timestamp(
    value: &serde_json::Value,
    key: &str,
    tz_rules: &Option<serde_json::Value>,
) -> serde_json::Value {
    let is_ts_field = tz_rules
        .as_ref()
        .and_then(|r| r.get(key))
        .and_then(|r| r.as_str())
        .map(|t| t == "timestamp" || t == "datetime")
        .unwrap_or(false);

    if !is_ts_field {
        return value.clone();
    }

    if let Some(ts_str) = value.as_str() {
        // Try parsing as ISO-8601 / RFC3339
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str) {
            let ny_time = dt.with_timezone(&New_York);
            return serde_json::json!({
                "normalized": ny_time.to_rfc3339(),
                "timezone": "America/New_York",
                "raw": ts_str
            });
        }
        // Try parsing as NaiveDateTime
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%dT%H:%M:%S") {
            use chrono::TimeZone;
            if let Some(utc_dt) = Utc.from_local_datetime(&ndt).single() {
                let ny_time = utc_dt.with_timezone(&New_York);
                return serde_json::json!({
                    "normalized": ny_time.to_rfc3339(),
                    "timezone": "America/New_York",
                    "raw": ts_str
                });
            }
        }
    }
    value.clone()
}

fn normalize_unit(value: &serde_json::Value, rule: &serde_json::Value) -> serde_json::Value {
    if let (Some(num), Some(from), Some(to)) = (
        value.as_f64(),
        rule.get("from").and_then(|f| f.as_str()),
        rule.get("to").and_then(|t| t.as_str()),
    ) {
        let converted = match (from, to) {
            ("kg", "lb") => num * 2.20462,
            ("km", "mi") => num * 0.621371,
            ("celsius", "fahrenheit") => num * 9.0 / 5.0 + 32.0,
            ("liters", "gallons") => num * 0.264172,
            ("meters", "feet") => num * 3.28084,
            ("cm", "inches") => num * 0.393701,
            _ => num,
        };
        serde_json::json!(converted)
    } else {
        value.clone()
    }
}

fn compute_fingerprint(title: &str, fields: &serde_json::Value, keys: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();

    let normalized_title = title.trim().to_lowercase();
    hasher.update(normalized_title.as_bytes());

    if let Some(obj) = fields.as_object() {
        // Sort keys for determinism
        let mut sorted_keys: Vec<&String> = keys.iter().filter(|k| obj.contains_key(k.as_str())).collect();
        sorted_keys.sort();
        for key in sorted_keys {
            if let Some(val) = obj.get(key.as_str()) {
                hasher.update(key.as_bytes());
                hasher.update(val.to_string().as_bytes());
            }
        }
    }

    hex::encode(hasher.finalize())
}

// Publicly exposed for unit testing
pub fn compute_fingerprint_pub(title: &str, fields: &serde_json::Value, keys: &[String]) -> String {
    compute_fingerprint(title, fields, keys)
}

pub fn compute_z_score(value: f64, mean: f64, stddev: f64) -> f64 {
    if stddev == 0.0 { return 0.0; }
    (value - mean) / stddev
}
