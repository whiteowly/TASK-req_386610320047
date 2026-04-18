use crate::config::DbPool;
use crate::errors::{AppError, FieldError};
use crate::models::{Template, TemplateVersion, PaginationParams};
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub channel_scope: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct CreateTemplateVersionRequest {
    pub field_schema: serde_json::Value,
    pub constraints_schema: Option<serde_json::Value>,
    pub cross_field_rules: Option<serde_json::Value>,
    pub change_note: Option<String>,
}

pub fn create_template(pool: &DbPool, req: &CreateTemplateRequest, actor_id: Uuid, actor_username: &str) -> Result<Template, AppError> {
    let conn = &mut pool.get()?;
    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    if req.slug.trim().is_empty() || req.name.trim().is_empty() {
        return Err(AppError::ValidationError(vec![
            FieldError { field: "slug".to_string(), reason: "Slug and name are required".to_string() }
        ]));
    }

    diesel::insert_into(templates::table)
        .values((
            templates::id.eq(id),
            templates::name.eq(req.name.trim()),
            templates::slug.eq(req.slug.trim()),
            templates::description.eq(req.description.as_deref()),
            templates::channel_scope.eq(req.channel_scope),
            templates::created_at.eq(now),
            templates::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "CREATE_TEMPLATE", "template", Some(id), None, Some(serde_json::json!({"name": req.name, "slug": req.slug})), None, None);

    Ok(Template { id, name: req.name.trim().to_string(), slug: req.slug.trim().to_string(), description: req.description.clone(), channel_scope: req.channel_scope, active_version_id: None, created_at: now, updated_at: now })
}

pub fn list_templates(pool: &DbPool, pagination: &PaginationParams) -> Result<(Vec<Template>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = templates::table.count().get_result(conn)?;
    let results = templates::table
        .order(templates::created_at.desc())
        .offset(pagination.offset())
        .limit(pagination.page_size())
        .load(conn)?;
    Ok((results, total))
}

pub fn get_template(pool: &DbPool, template_id: Uuid) -> Result<Template, AppError> {
    let conn = &mut pool.get()?;
    templates::table.filter(templates::id.eq(template_id)).first(conn).map_err(|_| AppError::NotFound("Template not found".to_string()))
}

pub fn create_template_version(pool: &DbPool, template_id: Uuid, req: &CreateTemplateVersionRequest, actor_id: Uuid, actor_username: &str) -> Result<TemplateVersion, AppError> {
    let conn = &mut pool.get()?;

    // Verify template exists
    let _: Template = templates::table.filter(templates::id.eq(template_id)).first(conn)
        .map_err(|_| AppError::NotFound("Template not found".to_string()))?;

    // Validate field_schema
    validate_field_schema(&req.field_schema)?;

    // Get next version number
    let max_version: Option<i32> = template_versions::table
        .filter(template_versions::template_id.eq(template_id))
        .select(diesel::dsl::max(template_versions::version_number))
        .first(conn)?;
    let version_number = max_version.unwrap_or(0) + 1;

    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    diesel::insert_into(template_versions::table)
        .values((
            template_versions::id.eq(id),
            template_versions::template_id.eq(template_id),
            template_versions::version_number.eq(version_number),
            template_versions::field_schema.eq(&req.field_schema),
            template_versions::constraints_schema.eq(&req.constraints_schema),
            template_versions::cross_field_rules.eq(&req.cross_field_rules),
            template_versions::change_note.eq(req.change_note.as_deref()),
            template_versions::created_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "CREATE_TEMPLATE_VERSION", "template_version", Some(id), None, Some(serde_json::json!({"template_id": template_id, "version": version_number})), None, None);

    Ok(TemplateVersion { id, template_id, version_number, field_schema: req.field_schema.clone(), constraints_schema: req.constraints_schema.clone(), cross_field_rules: req.cross_field_rules.clone(), change_note: req.change_note.clone(), created_at: now, updated_at: now })
}

pub fn list_template_versions(pool: &DbPool, template_id: Uuid, pagination: &PaginationParams) -> Result<(Vec<TemplateVersion>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = template_versions::table.filter(template_versions::template_id.eq(template_id)).count().get_result(conn)?;
    let results = template_versions::table
        .filter(template_versions::template_id.eq(template_id))
        .order(template_versions::version_number.desc())
        .offset(pagination.offset())
        .limit(pagination.page_size())
        .load(conn)?;
    Ok((results, total))
}

pub fn get_template_version(pool: &DbPool, template_id: Uuid, version_id: Uuid) -> Result<TemplateVersion, AppError> {
    let conn = &mut pool.get()?;
    template_versions::table
        .filter(template_versions::id.eq(version_id))
        .filter(template_versions::template_id.eq(template_id))
        .first(conn)
        .map_err(|_| AppError::NotFound("Template version not found".to_string()))
}

pub fn activate_template_version(pool: &DbPool, template_id: Uuid, version_id: Uuid, actor_id: Uuid, actor_username: &str) -> Result<(), AppError> {
    let conn = &mut pool.get()?;

    // Verify version exists and belongs to template
    let _: TemplateVersion = template_versions::table
        .filter(template_versions::id.eq(version_id))
        .filter(template_versions::template_id.eq(template_id))
        .first(conn)
        .map_err(|_| AppError::NotFound("Template version not found".to_string()))?;

    let now = Utc::now().naive_utc();
    diesel::update(templates::table.filter(templates::id.eq(template_id)))
        .set((
            templates::active_version_id.eq(Some(version_id)),
            templates::updated_at.eq(now),
        ))
        .execute(conn)?;

    crate::audit::log_audit(pool, Some(actor_id), actor_username, "ACTIVATE_TEMPLATE_VERSION", "template", Some(template_id), None, Some(serde_json::json!({"active_version_id": version_id})), None, None);

    Ok(())
}

fn validate_field_schema(schema: &serde_json::Value) -> Result<(), AppError> {
    let fields = schema.as_array().ok_or_else(|| AppError::ValidationError(vec![
        FieldError { field: "field_schema".to_string(), reason: "field_schema must be an array of field definitions".to_string() }
    ]))?;

    let valid_types = ["string", "number", "date", "enum", "text"];

    for (i, field) in fields.iter().enumerate() {
        let _name = field.get("name").and_then(|n| n.as_str()).ok_or_else(|| AppError::ValidationError(vec![
            FieldError { field: format!("field_schema[{}].name", i), reason: "Field name is required".to_string() }
        ]))?;

        let field_type = field.get("type").and_then(|t| t.as_str()).ok_or_else(|| AppError::ValidationError(vec![
            FieldError { field: format!("field_schema[{}].type", i), reason: "Field type is required".to_string() }
        ]))?;

        if !valid_types.contains(&field_type) {
            return Err(AppError::ValidationError(vec![
                FieldError { field: format!("field_schema[{}].type", i), reason: format!("Field type must be one of: {:?}", valid_types) }
            ]));
        }

        // Text max length check
        if field_type == "text" {
            if let Some(max_len) = field.get("max_length").and_then(|m| m.as_u64()) {
                if max_len > 2000 {
                    return Err(AppError::ValidationError(vec![
                        FieldError { field: format!("field_schema[{}].max_length", i), reason: "Text field max_length cannot exceed 2000".to_string() }
                    ]));
                }
            }
        }

        // Regex must be compilable
        if let Some(pattern) = field.get("regex").and_then(|r| r.as_str()) {
            if regex::Regex::new(pattern).is_err() {
                return Err(AppError::ValidationError(vec![
                    FieldError { field: format!("field_schema[{}].regex", i), reason: "Regex pattern is not valid".to_string() }
                ]));
            }
        }

        // Enum options must be non-empty
        if field_type == "enum" {
            let options = field.get("options").and_then(|o| o.as_array());
            if options.map_or(true, |o| o.is_empty()) {
                return Err(AppError::ValidationError(vec![
                    FieldError { field: format!("field_schema[{}].options", i), reason: "Enum field must have non-empty options list".to_string() }
                ]));
            }
        }
    }

    Ok(())
}

pub fn validate_item_fields(field_schema: &serde_json::Value, fields: &serde_json::Value, cross_field_rules: &Option<serde_json::Value>) -> Result<(), AppError> {
    let empty_vec = vec![];
    let schema_fields = field_schema.as_array().unwrap_or(&empty_vec);
    let field_values = fields.as_object().ok_or_else(|| AppError::ValidationError(vec![
        FieldError { field: "fields".to_string(), reason: "fields must be a JSON object".to_string() }
    ]))?;

    let mut errors = Vec::new();

    for field_def in schema_fields {
        let name = field_def.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let field_type = field_def.get("type").and_then(|t| t.as_str()).unwrap_or("string");
        let required = field_def.get("required").and_then(|r| r.as_bool()).unwrap_or(false);

        let value = field_values.get(name);

        // Required check
        if required && (value.is_none() || value.unwrap().is_null()) {
            errors.push(FieldError { field: name.to_string(), reason: "This field is required".to_string() });
            continue;
        }

        if let Some(val) = value {
            if val.is_null() { continue; }

            match field_type {
                "string" => {
                    if !val.is_string() {
                        errors.push(FieldError { field: name.to_string(), reason: "Expected string value".to_string() });
                    }
                    if let Some(pattern) = field_def.get("regex").and_then(|r| r.as_str()) {
                        if let (Ok(re), Some(s)) = (regex::Regex::new(pattern), val.as_str()) {
                            if !re.is_match(s) {
                                errors.push(FieldError { field: name.to_string(), reason: format!("Value does not match pattern: {}", pattern) });
                            }
                        }
                    }
                }
                "number" => {
                    if !val.is_number() {
                        errors.push(FieldError { field: name.to_string(), reason: "Expected numeric value".to_string() });
                    }
                }
                "date" => {
                    if let Some(s) = val.as_str() {
                        if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_err() {
                            errors.push(FieldError { field: name.to_string(), reason: "Expected date in YYYY-MM-DD format".to_string() });
                        }
                    } else {
                        errors.push(FieldError { field: name.to_string(), reason: "Expected date string".to_string() });
                    }
                }
                "enum" => {
                    if let Some(options) = field_def.get("options").and_then(|o| o.as_array()) {
                        if !options.contains(val) {
                            errors.push(FieldError { field: name.to_string(), reason: format!("Value must be one of: {:?}", options) });
                        }
                    }
                }
                "text" => {
                    if let Some(s) = val.as_str() {
                        let max_len = field_def.get("max_length").and_then(|m| m.as_u64()).unwrap_or(2000);
                        if s.len() as u64 > max_len {
                            errors.push(FieldError { field: name.to_string(), reason: format!("Text exceeds maximum length of {}", max_len) });
                        }
                    } else {
                        errors.push(FieldError { field: name.to_string(), reason: "Expected text string".to_string() });
                    }
                }
                _ => {}
            }
        }
    }

    // Cross-field rules
    if let Some(rules) = cross_field_rules {
        if let Some(rules_arr) = rules.as_array() {
            for rule in rules_arr {
                let condition_field = rule.get("if_field").and_then(|f| f.as_str()).unwrap_or("");
                let condition_value = rule.get("if_value");
                let then_field = rule.get("then_field").and_then(|f| f.as_str()).unwrap_or("");
                let then_min = rule.get("then_min").and_then(|m| m.as_f64());
                let then_max = rule.get("then_max").and_then(|m| m.as_f64());

                if let Some(cv) = condition_value {
                    if field_values.get(condition_field) == Some(cv) {
                        if let Some(then_val) = field_values.get(then_field) {
                            if let Some(num) = then_val.as_f64() {
                                if let Some(min) = then_min {
                                    if num < min {
                                        errors.push(FieldError { field: then_field.to_string(), reason: format!("must be >= {} when {} = {:?}", min, condition_field, cv) });
                                    }
                                }
                                if let Some(max) = then_max {
                                    if num > max {
                                        errors.push(FieldError { field: then_field.to_string(), reason: format!("must be <= {} when {} = {:?}", max, condition_field, cv) });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::ValidationError(errors))
    }
}
