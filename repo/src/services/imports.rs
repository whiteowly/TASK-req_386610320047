use crate::config::{AppConfig, DbPool};
use crate::db_instrumentation::timed_query;
use crate::errors::{AppError, FieldError};
use crate::models::{Import, ImportRow, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ImportOptions {
    pub template_version_id: Uuid,
    pub channel_id: Uuid,
}

#[derive(Serialize)]
pub struct ImportSummary {
    pub import_id: Uuid,
    pub status: String,
    pub total_rows: i32,
    pub accepted_rows: i32,
    pub rejected_rows: i32,
}

pub fn create_import(
    pool: &DbPool, config: &AppConfig, user_id: Uuid, username: &str,
    filename: &str, file_data: &[u8], options: &ImportOptions,
    idempotency_key: Option<&str>,
) -> Result<Import, AppError> {
    // Check size
    if file_data.len() > config.import_max_size_bytes {
        return Err(AppError::PayloadTooLarge("File exceeds 10 MB limit".to_string()));
    }

    // Check signature
    crate::import_export::check_file_signature(file_data, filename)
        .map_err(|e| AppError::UnsupportedMedia(e))?;

    // Check idempotency
    if let Some(key) = idempotency_key {
        let conn = &mut pool.get()?;
        let existing: Option<Import> = imports::table
            .filter(imports::idempotency_key.eq(key))
            .first(conn).ok();
        if let Some(existing_import) = existing {
            return Ok(existing_import);
        }
    }

    let conn = &mut pool.get()?;

    // Verify template version exists
    let tv: crate::models::TemplateVersion = template_versions::table
        .filter(template_versions::id.eq(options.template_version_id))
        .first(conn)
        .map_err(|_| AppError::NotFound("Template version not found".to_string()))?;

    let import_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    let threshold = config.slow_query_threshold_ms;
    timed_query("create_import_insert", threshold, || {
        diesel::insert_into(imports::table)
            .values((
                imports::id.eq(import_id),
                imports::user_id.eq(user_id),
                imports::template_version_id.eq(options.template_version_id),
                imports::channel_id.eq(options.channel_id),
                imports::filename.eq(filename),
                imports::file_size.eq(Some(file_data.len() as i64)),
                imports::status.eq("processing"),
                imports::idempotency_key.eq(idempotency_key),
                imports::created_at.eq(now),
                imports::updated_at.eq(now),
            ))
            .execute(conn)
    })?;

    // Parse rows — use XLSX parser when filename ends with .xlsx, CSV otherwise
    let rows = if filename.to_lowercase().ends_with(".xlsx") {
        parse_xlsx_rows(file_data)
    } else {
        parse_csv_rows(file_data)
    };

    let mut accepted = 0i32;
    let mut rejected = 0i32;
    let total = rows.len() as i32;

    for (row_num, row_data) in rows.iter().enumerate() {
        let row_number = (row_num + 1) as i32;
        let validation_errors = validate_import_row(conn, row_data, &tv.field_schema, &tv.cross_field_rules, options.channel_id);

        if validation_errors.is_empty() {
            // Create item from row
            match create_item_from_import(conn, pool, row_data, &tv, options.channel_id, user_id, username) {
                Ok(item_id) => {
                    diesel::insert_into(import_rows::table)
                        .values((
                            import_rows::id.eq(Uuid::new_v4()),
                            import_rows::import_id.eq(import_id),
                            import_rows::row_number.eq(row_number),
                            import_rows::status.eq("accepted"),
                            import_rows::item_id.eq(Some(item_id)),
                            import_rows::created_at.eq(now),
                        ))
                        .execute(conn).ok();
                    accepted += 1;
                }
                Err(e) => {
                    diesel::insert_into(import_rows::table)
                        .values((
                            import_rows::id.eq(Uuid::new_v4()),
                            import_rows::import_id.eq(import_id),
                            import_rows::row_number.eq(row_number),
                            import_rows::status.eq("rejected"),
                            import_rows::errors.eq(Some(serde_json::json!([{"reason": e.to_string()}]))),
                            import_rows::created_at.eq(now),
                        ))
                        .execute(conn).ok();
                    rejected += 1;
                }
            }
        } else {
            diesel::insert_into(import_rows::table)
                .values((
                    import_rows::id.eq(Uuid::new_v4()),
                    import_rows::import_id.eq(import_id),
                    import_rows::row_number.eq(row_number),
                    import_rows::status.eq("rejected"),
                    import_rows::errors.eq(Some(serde_json::to_value(&validation_errors).unwrap_or_default())),
                    import_rows::created_at.eq(now),
                ))
                .execute(conn).ok();
            rejected += 1;
        }
    }

    let final_status = if rejected == 0 { "succeeded" } else if accepted > 0 { "partial_succeeded" } else { "failed" };

    diesel::update(imports::table.filter(imports::id.eq(import_id)))
        .set((
            imports::status.eq(final_status),
            imports::total_rows.eq(Some(total)),
            imports::accepted_rows.eq(Some(accepted)),
            imports::rejected_rows.eq(Some(rejected)),
            imports::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(user_id), username, "CREATE_IMPORT", "import", Some(import_id), None, Some(serde_json::json!({"total": total, "accepted": accepted, "rejected": rejected})), None, None);

    imports::table.filter(imports::id.eq(import_id)).first(conn).map_err(|e| AppError::Internal(e.to_string()))
}

fn parse_csv_rows(data: &[u8]) -> Vec<std::collections::HashMap<String, String>> {
    let mut rows = Vec::new();
    let mut reader = csv::ReaderBuilder::new().has_headers(true).from_reader(data);
    let headers: Vec<String> = reader.headers().ok().map(|h| h.iter().map(String::from).collect()).unwrap_or_default();

    for result in reader.records() {
        if let Ok(record) = result {
            let mut row = std::collections::HashMap::new();
            for (i, val) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    row.insert(header.clone(), val.to_string());
                }
            }
            rows.push(row);
        }
    }
    rows
}

fn parse_xlsx_rows(data: &[u8]) -> Vec<std::collections::HashMap<String, String>> {
    use calamine::{Reader, Xlsx, open_workbook_from_rs};
    use std::io::Cursor;

    let mut rows = Vec::new();
    let cursor = Cursor::new(data);
    let mut workbook: Xlsx<_> = match open_workbook_from_rs(cursor) {
        Ok(wb) => wb,
        Err(_) => return rows,
    };

    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = match sheet_names.first() {
        Some(name) => name.clone(),
        None => return rows,
    };

    let range = match workbook.worksheet_range(&sheet_name) {
        Ok(r) => r,
        Err(_) => return rows,
    };

    let mut iter = range.rows();

    // First row is headers
    let headers: Vec<String> = match iter.next() {
        Some(header_row) => header_row.iter().map(|cell| cell.to_string().trim().to_string()).collect(),
        None => return rows,
    };

    for row_cells in iter {
        let mut row = std::collections::HashMap::new();
        for (i, cell) in row_cells.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                if header.is_empty() { continue; }
                let s = cell.to_string();
                let value = if s == "" || s == "empty" { String::new() } else { s };
                row.insert(header.clone(), value);
            }
        }
        rows.push(row);
    }

    rows
}

fn validate_import_row(
    conn: &mut PgConnection,
    row: &std::collections::HashMap<String, String>,
    field_schema: &serde_json::Value,
    cross_field_rules: &Option<serde_json::Value>,
    channel_id: Uuid,
) -> Vec<serde_json::Value> {
    let mut errors = Vec::new();

    // Check for duplicate by auto-number
    if let Some(auto_num) = row.get("auto_number") {
        if !auto_num.is_empty() {
            let exists: bool = diesel::select(diesel::dsl::exists(
                items::table.filter(items::auto_number.eq(auto_num))
            )).get_result(conn).unwrap_or(false);
            if exists {
                errors.push(serde_json::json!({"field": "auto_number", "reason": "Duplicate auto-number"}));
            }
        }
    }

    // Check for duplicate by normalized title + channel within 90 days
    // Compare normalized input against normalized stored title (both sides)
    if let Some(title) = row.get("title") {
        let normalized = crate::import_export::normalize_title(title);
        let cutoff = Utc::now().naive_utc() - chrono::Duration::days(90);

        #[derive(diesel::QueryableByName)]
        struct BoolResult {
            #[diesel(sql_type = diesel::sql_types::Bool)]
            result: bool,
        }

        let dup_exists: bool = diesel::sql_query(
            "SELECT EXISTS(\
                SELECT 1 FROM items \
                INNER JOIN item_versions ON items.current_version_id = item_versions.id \
                WHERE items.channel_id = $1 \
                AND items.created_at >= $2 \
                AND LOWER(TRIM(REGEXP_REPLACE(item_versions.title, '\\s+', ' ', 'g'))) = $3\
            ) AS result"
        )
        .bind::<diesel::sql_types::Uuid, _>(channel_id)
        .bind::<diesel::sql_types::Timestamp, _>(cutoff)
        .bind::<diesel::sql_types::Text, _>(&normalized)
        .get_result::<BoolResult>(conn)
        .map(|r| r.result)
        .unwrap_or(false);
        if dup_exists {
            errors.push(serde_json::json!({"field": "title", "reason": "Duplicate title+channel within 90 days"}));
        }
    }

    // Validate fields from schema: required check + type/constraint checks
    if let Some(fields) = field_schema.as_array() {
        for field_def in fields {
            let name = field_def.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let field_type = field_def.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let required = field_def.get("required").and_then(|r| r.as_bool()).unwrap_or(false);

            let raw_value = row.get(name).map(|s| s.as_str()).unwrap_or("");

            // Required field check
            if required && raw_value.trim().is_empty() {
                errors.push(serde_json::json!({"field": name, "reason": "Required field is missing"}));
                // Skip further checks for this field — value is absent
                continue;
            }

            // Skip type/constraint checks for empty optional fields
            if raw_value.trim().is_empty() {
                continue;
            }

            // Enum option validation
            if field_type == "enum" {
                if let Some(options) = field_def.get("options").and_then(|o| o.as_array()) {
                    let allowed: Vec<&str> = options.iter()
                        .filter_map(|o| o.as_str())
                        .collect();
                    if !allowed.contains(&raw_value) {
                        errors.push(serde_json::json!({
                            "field": name,
                            "reason": format!("Value '{}' is not a valid option; allowed: [{}]", raw_value, allowed.join(", "))
                        }));
                    }
                }
            }

            // Regex constraint validation
            if let Some(pattern) = field_def.get("regex").and_then(|r| r.as_str()) {
                match regex::Regex::new(pattern) {
                    Ok(re) => {
                        if !re.is_match(raw_value) {
                            errors.push(serde_json::json!({
                                "field": name,
                                "reason": format!("Value '{}' does not match required pattern", raw_value)
                            }));
                        }
                    }
                    Err(_) => {
                        // Malformed regex in schema — skip silently to avoid false rejections
                    }
                }
            }

            // Number format validation
            if field_type == "number" {
                if raw_value.parse::<f64>().is_err() {
                    errors.push(serde_json::json!({
                        "field": name,
                        "reason": format!("Value '{}' is not a valid number", raw_value)
                    }));
                }
            }

            // Date format validation (YYYY-MM-DD)
            if field_type == "date" {
                if chrono::NaiveDate::parse_from_str(raw_value, "%Y-%m-%d").is_err() {
                    errors.push(serde_json::json!({
                        "field": name,
                        "reason": format!("Value '{}' is not a valid date (expected YYYY-MM-DD)", raw_value)
                    }));
                }
            }

            // Text length validation (max_length capped at 2000)
            if field_type == "text" {
                if let Some(max_len_val) = field_def.get("max_length").and_then(|m| m.as_u64()) {
                    let max_len = (max_len_val as usize).min(2000);
                    if raw_value.len() > max_len {
                        errors.push(serde_json::json!({
                            "field": name,
                            "reason": format!("Value exceeds maximum length of {} characters", max_len)
                        }));
                    }
                }
            }
        }
    }

    // Cross-field rule validation
    if let Some(rules_val) = cross_field_rules {
        if let Some(rules) = rules_val.as_array() {
            for rule in rules {
                let if_field = rule.get("if_field").and_then(|v| v.as_str());
                let if_value = rule.get("if_value").and_then(|v| v.as_str());
                let then_field = rule.get("then_field").and_then(|v| v.as_str());
                let then_min = rule.get("then_min").and_then(|v| v.as_f64());
                let then_max = rule.get("then_max").and_then(|v| v.as_f64());

                if let (Some(if_f), Some(if_v), Some(then_f)) = (if_field, if_value, then_field) {
                    // Check whether the condition is satisfied
                    let condition_met = row.get(if_f)
                        .map(|v| v.as_str() == if_v)
                        .unwrap_or(false);

                    if condition_met {
                        let then_raw = row.get(then_f).map(|s| s.as_str()).unwrap_or("");
                        match then_raw.parse::<f64>() {
                            Ok(num) => {
                                if let Some(min) = then_min {
                                    if num < min {
                                        errors.push(serde_json::json!({
                                            "field": then_f,
                                            "reason": format!(
                                                "When {}={}, {} must be >= {} (got {})",
                                                if_f, if_v, then_f, min, num
                                            )
                                        }));
                                    }
                                }
                                if let Some(max) = then_max {
                                    if num > max {
                                        errors.push(serde_json::json!({
                                            "field": then_f,
                                            "reason": format!(
                                                "When {}={}, {} must be <= {} (got {})",
                                                if_f, if_v, then_f, max, num
                                            )
                                        }));
                                    }
                                }
                            }
                            Err(_) => {
                                // then_field is required by the rule but not a valid number
                                if then_min.is_some() || then_max.is_some() {
                                    errors.push(serde_json::json!({
                                        "field": then_f,
                                        "reason": format!(
                                            "When {}={}, {} must be a numeric value",
                                            if_f, if_v, then_f
                                        )
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}

fn create_item_from_import(
    conn: &mut PgConnection, pool: &DbPool,
    row: &std::collections::HashMap<String, String>,
    tv: &crate::models::TemplateVersion,
    channel_id: Uuid, user_id: Uuid, username: &str,
) -> Result<Uuid, AppError> {
    let title = row.get("title").cloned().unwrap_or_default();
    let body = row.get("body").cloned();

    // Build fields JSON from row data
    let mut fields_map = serde_json::Map::new();
    if let Some(schema_fields) = tv.field_schema.as_array() {
        for field_def in schema_fields {
            if let Some(name) = field_def.get("name").and_then(|n| n.as_str()) {
                if let Some(val) = row.get(name) {
                    let field_type = field_def.get("type").and_then(|t| t.as_str()).unwrap_or("string");
                    match field_type {
                        "number" => {
                            if let Ok(n) = val.parse::<f64>() {
                                fields_map.insert(name.to_string(), serde_json::json!(n));
                            } else {
                                fields_map.insert(name.to_string(), serde_json::Value::String(val.clone()));
                            }
                        }
                        _ => {
                            fields_map.insert(name.to_string(), serde_json::Value::String(val.clone()));
                        }
                    }
                }
            }
        }
    }

    // Generate auto-number
    use chrono::TimeZone;
    use chrono_tz::America::New_York;
    let now_ny = Utc::now().with_timezone(&New_York);
    let today_ny = now_ny.date_naive();
    let date_str = today_ny.format("%Y%m%d").to_string();

    #[derive(QueryableByName)]
    struct CounterResult {
        #[diesel(sql_type = diesel::sql_types::Int4)]
        pub last_sequence: i32,
    }

    let result = diesel::sql_query(
        "INSERT INTO daily_counters (id, counter_date, last_sequence, created_at, updated_at) \
         VALUES ($1, $2, 1, NOW(), NOW()) \
         ON CONFLICT (counter_date) DO UPDATE SET last_sequence = daily_counters.last_sequence + 1, updated_at = NOW() \
         RETURNING last_sequence"
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Date, _>(today_ny)
    .get_result::<CounterResult>(conn)
    .map_err(|e| AppError::Internal(format!("Auto-number generation failed: {}", e)))?;

    let auto_number = format!("KO-{}-{:05}", date_str, result.last_sequence);

    let item_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    // Insert item first with NULL current_version_id to satisfy FK constraint
    diesel::insert_into(items::table)
        .values((
            items::id.eq(item_id),
            items::template_id.eq(tv.template_id),
            items::channel_id.eq(channel_id),
            items::owner_user_id.eq(user_id),
            items::auto_number.eq(&auto_number),
            items::status.eq("Draft"),
            items::created_at.eq(now),
            items::updated_at.eq(now),
        ))
        .execute(conn)?;

    // Insert version row (item exists, so item_id FK is satisfied)
    diesel::insert_into(item_versions::table)
        .values((
            item_versions::id.eq(version_id),
            item_versions::item_id.eq(item_id),
            item_versions::version_number.eq(1),
            item_versions::template_version_id.eq(tv.id),
            item_versions::title.eq(&title),
            item_versions::body.eq(body.as_deref()),
            item_versions::fields.eq(serde_json::Value::Object(fields_map)),
            item_versions::created_by.eq(user_id),
            item_versions::created_at.eq(now),
        ))
        .execute(conn)?;

    // Now set current_version_id (version row exists, so version FK is satisfied)
    diesel::update(items::table.filter(items::id.eq(item_id)))
        .set(items::current_version_id.eq(Some(version_id)))
        .execute(conn)?;

    Ok(item_id)
}

pub fn list_imports(pool: &DbPool, user_id: Uuid, role: &str, pagination: &PaginationParams) -> Result<(Vec<Import>, i64), AppError> {
    let conn = &mut pool.get()?;
    let mut query = imports::table.into_boxed();
    let mut count_query = imports::table.into_boxed();
    if role == "Author" {
        query = query.filter(imports::user_id.eq(user_id));
        count_query = count_query.filter(imports::user_id.eq(user_id));
    }
    let total: i64 = count_query.count().get_result(conn)?;
    let results = query.order(imports::created_at.desc()).offset(pagination.offset()).limit(pagination.page_size()).load(conn)?;
    Ok((results, total))
}

pub fn get_import(pool: &DbPool, import_id: Uuid, user_id: Uuid, role: &str) -> Result<Import, AppError> {
    let conn = &mut pool.get()?;
    let imp: Import = imports::table.filter(imports::id.eq(import_id)).first(conn)
        .map_err(|_| AppError::NotFound("Import not found".to_string()))?;
    if role == "Author" && imp.user_id != user_id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }
    Ok(imp)
}

pub fn get_import_errors(pool: &DbPool, import_id: Uuid, pagination: &PaginationParams) -> Result<(Vec<ImportRow>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = import_rows::table.filter(import_rows::import_id.eq(import_id)).filter(import_rows::status.eq("rejected")).count().get_result(conn)?;
    let results = import_rows::table
        .filter(import_rows::import_id.eq(import_id))
        .filter(import_rows::status.eq("rejected"))
        .order(import_rows::row_number.asc())
        .offset(pagination.offset())
        .limit(pagination.page_size())
        .load(conn)?;
    Ok((results, total))
}

pub fn get_import_result(pool: &DbPool, import_id: Uuid) -> Result<(Vec<ImportRow>, Vec<ImportRow>), AppError> {
    let conn = &mut pool.get()?;
    let accepted = import_rows::table.filter(import_rows::import_id.eq(import_id)).filter(import_rows::status.eq("accepted")).load(conn)?;
    let rejected = import_rows::table.filter(import_rows::import_id.eq(import_id)).filter(import_rows::status.eq("rejected")).load(conn)?;
    Ok((accepted, rejected))
}

pub fn generate_import_template(pool: &DbPool, template_version_id: Uuid, format: &str) -> Result<(Vec<u8>, String), AppError> {
    let conn = &mut pool.get()?;
    let tv: crate::models::TemplateVersion = template_versions::table
        .filter(template_versions::id.eq(template_version_id))
        .first(conn)
        .map_err(|_| AppError::NotFound("Template version not found".to_string()))?;

    let fields = tv.field_schema.as_array().cloned().unwrap_or_default();
    let mut headers = vec!["auto_number".to_string(), "title".to_string(), "body".to_string()];
    for field in &fields {
        if let Some(name) = field.get("name").and_then(|n| n.as_str()) {
            headers.push(name.to_string());
        }
    }

    match format {
        "csv" => {
            let mut writer = csv::Writer::from_writer(Vec::new());
            writer.write_record(&headers).ok();
            let data = writer.into_inner().unwrap_or_default();
            Ok((data, "text/csv".to_string()))
        }
        "xlsx" => {
            use rust_xlsxwriter::{Workbook, Format};
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();
            let bold = Format::new().set_bold();
            for (col, header) in headers.iter().enumerate() {
                worksheet.write_string_with_format(0, col as u16, header, &bold).ok();
            }
            let buf = workbook.save_to_buffer()
                .map_err(|e| AppError::Internal(format!("XLSX generation failed: {}", e)))?;
            Ok((buf, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()))
        }
        _ => {
            Err(AppError::BadRequest(format!("Unsupported template format: '{}'. Supported formats: csv, xlsx", format)))
        }
    }
}
