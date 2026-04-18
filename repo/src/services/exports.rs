use crate::config::{AppConfig, DbPool};
use crate::db_instrumentation::timed_query;
use crate::errors::AppError;
use crate::models::{Export, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateExportRequest {
    pub scope_filters: serde_json::Value,
    pub format: String,
    pub include_explanations: Option<bool>,
    pub mask_sensitive: Option<bool>,
}

const SUPPORTED_FORMATS: &[&str] = &["csv", "xlsx"];

pub fn create_export(
    pool: &DbPool, config: &AppConfig, req: &CreateExportRequest,
    user_id: Uuid, username: &str, idempotency_key: Option<&str>,
) -> Result<Export, AppError> {
    if !SUPPORTED_FORMATS.contains(&req.format.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unsupported export format '{}'. Supported: csv, xlsx.", req.format
        )));
    }

    let conn = &mut pool.get()?;

    if let Some(key) = idempotency_key {
        let existing: Option<Export> = exports::table.filter(exports::idempotency_key.eq(key)).first(conn).ok();
        if let Some(e) = existing { return Ok(e); }
    }

    let export_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    let include_exp = req.include_explanations.unwrap_or(false);
    let mask = req.mask_sensitive.unwrap_or(false);

    let threshold = config.slow_query_threshold_ms;

    timed_query("create_export_insert", threshold, || {
        diesel::insert_into(exports::table)
            .values((
                exports::id.eq(export_id),
                exports::user_id.eq(user_id),
                exports::scope_filters.eq(&req.scope_filters),
                exports::format.eq(&req.format),
                exports::include_explanations.eq(include_exp),
                exports::mask_sensitive.eq(mask),
                exports::status.eq("processing"),
                exports::idempotency_key.eq(idempotency_key),
                exports::created_at.eq(now),
                exports::updated_at.eq(now),
            ))
            .execute(conn)
    })?;

    match generate_export_artifact(conn, config, export_id, &req.format, &req.scope_filters, include_exp, mask) {
        Ok((path, checksum, size)) => {
            diesel::update(exports::table.filter(exports::id.eq(export_id)))
                .set((
                    exports::status.eq("succeeded"),
                    exports::artifact_path.eq(Some(&path)),
                    exports::artifact_checksum.eq(Some(&checksum)),
                    exports::artifact_size.eq(Some(size)),
                    exports::updated_at.eq(Utc::now().naive_utc()),
                ))
                .execute(conn)?;

            diesel::insert_into(export_artifacts::table)
                .values((
                    export_artifacts::id.eq(Uuid::new_v4()),
                    export_artifacts::export_id.eq(export_id),
                    export_artifacts::file_path.eq(&path),
                    export_artifacts::checksum.eq(&checksum),
                    export_artifacts::size_bytes.eq(size),
                    export_artifacts::masking_applied.eq(mask),
                    export_artifacts::explanations_included.eq(include_exp),
                    export_artifacts::created_at.eq(Utc::now().naive_utc()),
                ))
                .execute(conn)?;
        }
        Err(e) => {
            diesel::update(exports::table.filter(exports::id.eq(export_id)))
                .set((exports::status.eq("failed"), exports::updated_at.eq(Utc::now().naive_utc())))
                .execute(conn)?;
            log::error!("Export {} failed: {}", export_id, e);
        }
    }

    crate::audit::log_audit(pool, Some(user_id), username, "CREATE_EXPORT", "export", Some(export_id), None, Some(serde_json::json!({"format": &req.format, "mask": mask})), None, None);
    exports::table.filter(exports::id.eq(export_id)).first(conn).map_err(|e| AppError::Internal(e.to_string()))
}

struct ExportData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn collect_export_data(
    conn: &mut PgConnection,
    slow_query_threshold_ms: u64,
    scope_filters: &serde_json::Value, include_explanations: bool, mask_sensitive: bool,
) -> Result<ExportData, String> {
    let mut query = items::table
        .inner_join(item_versions::table.on(items::current_version_id.eq(item_versions::id.nullable())))
        .into_boxed();

    if let Some(status) = scope_filters.get("status").and_then(|s| s.as_str()) {
        query = query.filter(items::status.eq(status));
    }
    if let Some(channel) = scope_filters.get("channel_id").and_then(|c| c.as_str()) {
        if let Ok(cid) = uuid::Uuid::parse_str(channel) {
            query = query.filter(items::channel_id.eq(cid));
        }
    }

    let rows: Vec<(Uuid, String, String, String, Option<String>, serde_json::Value, Uuid)> = timed_query("export_collect_data", slow_query_threshold_ms, || {
        query
            .select((items::id, items::auto_number, items::status, item_versions::title, item_versions::body, item_versions::fields, item_versions::template_version_id))
            .limit(10000)
            .load(conn)
            .map_err(|e| e.to_string())
    })?;

    let sensitive_field_cache: std::collections::HashMap<Uuid, Vec<String>> = {
        let mut cache = std::collections::HashMap::new();
        let tv_ids: Vec<Uuid> = rows.iter().map(|r| r.6).collect::<std::collections::HashSet<_>>().into_iter().collect();
        for tv_id in tv_ids {
            let tv_opt: Option<(serde_json::Value,)> = template_versions::table
                .filter(template_versions::id.eq(tv_id))
                .select((template_versions::field_schema,))
                .first(conn).ok();
            if let Some((schema,)) = tv_opt {
                let sensitive: Vec<String> = schema.as_array().map(|arr| {
                    arr.iter().filter_map(|f| {
                        let is_sensitive = f.get("sensitive").and_then(|s| s.as_bool()).unwrap_or(false);
                        if is_sensitive { f.get("name").and_then(|n| n.as_str()).map(String::from) } else { None }
                    }).collect()
                }).unwrap_or_default();
                cache.insert(tv_id, sensitive);
            }
        }
        cache
    };

    let mut all_field_keys: Vec<String> = Vec::new();
    for (_, _, _, _, _, fields, _) in &rows {
        if let Some(obj) = fields.as_object() {
            for key in obj.keys() {
                if !all_field_keys.contains(key) { all_field_keys.push(key.clone()); }
            }
        }
    }
    all_field_keys.sort();

    let mut headers: Vec<String> = vec!["auto_number".into(), "status".into(), "title".into(), "body".into()];
    for key in &all_field_keys { headers.push(format!("field_{}", key)); }
    if include_explanations { headers.push("explanation".into()); }

    let mut out_rows: Vec<Vec<String>> = Vec::new();
    for (_, auto_number, status, title, body, fields, tv_id) in &rows {
        let sensitive_fields = sensitive_field_cache.get(tv_id).cloned().unwrap_or_default();
        let body_str = body.as_deref().unwrap_or("");
        let mut record = vec![
            auto_number.clone(),
            status.clone(),
            if mask_sensitive { crate::import_export::mask_sensitive_value(title) } else { title.clone() },
            if mask_sensitive { crate::import_export::mask_sensitive_value(body_str) } else { body_str.to_string() },
        ];
        for key in &all_field_keys {
            let raw_val = fields.get(key).map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            }).unwrap_or_default();
            let val = if mask_sensitive && sensitive_fields.contains(key) {
                "[MASKED]".to_string()
            } else if mask_sensitive {
                crate::import_export::mask_sensitive_value(&raw_val)
            } else { raw_val };
            record.push(val);
        }
        if include_explanations { record.push("Exported from KnowledgeOps".into()); }
        out_rows.push(record);
    }

    Ok(ExportData { headers, rows: out_rows })
}

fn generate_export_artifact(
    conn: &mut PgConnection, config: &AppConfig, export_id: Uuid,
    format: &str, scope_filters: &serde_json::Value,
    include_explanations: bool, mask_sensitive: bool,
) -> Result<(String, String, i64), String> {
    let data = collect_export_data(
        conn,
        config.slow_query_threshold_ms,
        scope_filters,
        include_explanations,
        mask_sensitive,
    )?;

    let (file_bytes, extension) = match format {
        "xlsx" => (write_xlsx(&data)?, "xlsx"),
        _ => (write_csv(&data)?, "csv"),
    };

    let checksum = {
        use sha2::{Sha256, Digest};
        hex::encode(Sha256::digest(&file_bytes))
    };

    let filename = format!("export_{}.{}", export_id, extension);
    let path = format!("{}/{}", config.data_dir, filename);
    std::fs::create_dir_all(&config.data_dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, &file_bytes).map_err(|e| e.to_string())?;

    Ok((path, checksum, file_bytes.len() as i64))
}

fn write_csv(data: &ExportData) -> Result<Vec<u8>, String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    let header_refs: Vec<&str> = data.headers.iter().map(|s| s.as_str()).collect();
    writer.write_record(&header_refs).map_err(|e| e.to_string())?;
    for row in &data.rows {
        writer.write_record(row).map_err(|e| e.to_string())?;
    }
    writer.into_inner().map_err(|e| e.to_string())
}

fn write_xlsx(data: &ExportData) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Workbook, Format};

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    let bold = Format::new().set_bold();

    for (col, header) in data.headers.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, header, &bold)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, row) in data.rows.iter().enumerate() {
        for (col, val) in row.iter().enumerate() {
            sheet.write_string((row_idx + 1) as u32, col as u16, val)
                .map_err(|e| e.to_string())?;
        }
    }

    workbook.save_to_buffer().map_err(|e| e.to_string())
}

pub fn list_exports(pool: &DbPool, pagination: &PaginationParams) -> Result<(Vec<Export>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = exports::table.count().get_result(conn)?;
    let results = exports::table.order(exports::created_at.desc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)?;
    Ok((results, total))
}

pub fn get_export(pool: &DbPool, export_id: Uuid) -> Result<Export, AppError> {
    let conn = &mut pool.get()?;
    exports::table.filter(exports::id.eq(export_id)).first(conn).map_err(|_| AppError::NotFound("Export not found".to_string()))
}

pub fn download_export(pool: &DbPool, export_id: Uuid) -> Result<(Vec<u8>, String), AppError> {
    let exp = get_export(pool, export_id)?;
    let path = exp.artifact_path.ok_or_else(|| AppError::NotFound("Export artifact not ready".to_string()))?;
    let data = std::fs::read(&path).map_err(|_| AppError::NotFound("Export file not found".to_string()))?;
    let content_type = if exp.format == "xlsx" {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    } else {
        "text/csv"
    };
    Ok((data, content_type.to_string()))
}
