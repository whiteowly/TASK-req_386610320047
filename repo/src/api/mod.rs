pub mod handlers;
pub mod middleware;
pub mod dto;

use actix_web::web;
use middleware::{AuthMiddleware, RateLimitMiddleware};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(handlers::health::health_check))
            .route("/auth/login", web::post().to(handlers::auth::login))
            .route("/auth/captcha/challenge", web::post().to(handlers::auth::captcha_challenge))
            .service(
                web::scope("")
                    .wrap(RateLimitMiddleware)
                    .wrap(AuthMiddleware)
                    .route("/auth/logout", web::post().to(handlers::auth::logout))
                    .route("/auth/me", web::get().to(handlers::auth::me))
                    // Users
                    .route("/users", web::get().to(handlers::users::list_users))
                    .route("/users", web::post().to(handlers::users::create_user))
                    .route("/users/{user_id}", web::patch().to(handlers::users::update_user))
                    .route("/users/{user_id}/reset-password", web::post().to(handlers::users::reset_password))
                    // Channels & Tags
                    .route("/channels", web::get().to(handlers::channels::list_channels))
                    .route("/channels", web::post().to(handlers::channels::create_channel))
                    .route("/channels/{channel_id}", web::patch().to(handlers::channels::update_channel))
                    .route("/tags", web::get().to(handlers::tags::list_tags))
                    .route("/tags", web::post().to(handlers::tags::create_tag))
                    // Templates
                    .route("/templates", web::post().to(handlers::templates::create_template))
                    .route("/templates", web::get().to(handlers::templates::list_templates))
                    .route("/templates/{template_id}", web::get().to(handlers::templates::get_template))
                    .route("/templates/{template_id}/versions", web::post().to(handlers::templates::create_template_version))
                    .route("/templates/{template_id}/versions", web::get().to(handlers::templates::list_template_versions))
                    .route("/templates/{template_id}/versions/{version_id}", web::get().to(handlers::templates::get_template_version))
                    .route("/templates/{template_id}/versions/{version_id}/activate", web::post().to(handlers::templates::activate_version))
                    // Items
                    .route("/items", web::post().to(handlers::items::create_item))
                    .route("/items", web::get().to(handlers::items::list_items))
                    .route("/items/{item_id}", web::get().to(handlers::items::get_item))
                    .route("/items/{item_id}", web::patch().to(handlers::items::update_item))
                    .route("/items/{item_id}/versions", web::get().to(handlers::items::list_versions))
                    .route("/items/{item_id}/versions/{version_id}", web::get().to(handlers::items::get_version))
                    .route("/items/{item_id}/rollback", web::post().to(handlers::items::rollback))
                    .route("/items/{item_id}/transitions", web::post().to(handlers::items::transition))
                    .route("/items/{item_id}/publish", web::post().to(handlers::items::publish))
                    // Search
                    .route("/search", web::get().to(handlers::search::search))
                    .route("/search/suggestions", web::get().to(handlers::search::suggestions))
                    .route("/search/trending", web::get().to(handlers::search::trending))
                    .route("/search/history", web::get().to(handlers::search::history))
                    .route("/search/history", web::delete().to(handlers::search::clear_history))
                    // Imports
                    .route("/imports/templates/{template_version_id}", web::get().to(handlers::imports::download_template))
                    .route("/imports", web::post().to(handlers::imports::create_import))
                    .route("/imports", web::get().to(handlers::imports::list_imports))
                    .route("/imports/{import_id}", web::get().to(handlers::imports::get_import))
                    .route("/imports/{import_id}/errors", web::get().to(handlers::imports::get_errors))
                    .route("/imports/{import_id}/result", web::get().to(handlers::imports::get_result))
                    // Exports
                    .route("/exports", web::post().to(handlers::exports::create_export))
                    .route("/exports", web::get().to(handlers::exports::list_exports))
                    .route("/exports/{export_id}", web::get().to(handlers::exports::get_export))
                    .route("/exports/{export_id}/download", web::get().to(handlers::exports::download))
                    // Schema Mappings
                    .route("/schema-mappings", web::post().to(handlers::schema_mappings::create_mapping))
                    .route("/schema-mappings", web::get().to(handlers::schema_mappings::list_mappings))
                    .route("/schema-mappings/{mapping_id}", web::get().to(handlers::schema_mappings::get_mapping))
                    .route("/schema-mappings/{mapping_id}/versions", web::post().to(handlers::schema_mappings::create_version))
                    .route("/schema-mappings/{mapping_id}/versions", web::get().to(handlers::schema_mappings::list_versions))
                    // Standardization
                    .route("/standardization/jobs", web::post().to(handlers::standardization::create_job))
                    .route("/standardization/jobs", web::get().to(handlers::standardization::list_jobs))
                    .route("/standardization/jobs/{job_id}", web::get().to(handlers::standardization::get_job))
                    .route("/standardization/models", web::get().to(handlers::standardization::list_models))
                    .route("/standardization/models/{model_id}", web::get().to(handlers::standardization::get_model))
                    .route("/standardization/models/{model_id}/records", web::get().to(handlers::standardization::get_records))
                    // Events & Metrics
                    .route("/events", web::post().to(handlers::events::create_event))
                    .route("/events", web::get().to(handlers::events::list_events))
                    .route("/metrics/snapshots", web::post().to(handlers::metrics::create_snapshot))
                    .route("/metrics/snapshots", web::get().to(handlers::metrics::list_snapshots))
                    // Analytics
                    .route("/analytics/kpis", web::get().to(handlers::analytics::get_kpis))
                    .route("/analytics/operational", web::get().to(handlers::analytics::get_operational))
                    .route("/analytics/export", web::post().to(handlers::analytics::create_export))
                    // Feature Flags
                    .route("/feature-flags", web::get().to(handlers::feature_flags::list_flags))
                    .route("/feature-flags", web::post().to(handlers::feature_flags::create_flag))
                    .route("/feature-flags/{key}", web::patch().to(handlers::feature_flags::update_flag))
                    // Ops Alerts
                    .route("/ops/alerts", web::get().to(handlers::ops::list_alerts))
                    .route("/ops/alerts/{alert_id}/ack", web::post().to(handlers::ops::ack_alert))
                    // Ops Diagnostics (admin-only alert triggers for verification)
                    .route("/ops/diagnostic/error", web::post().to(handlers::ops::diagnostic_trigger_error))
                    .route("/ops/diagnostic/job-failure", web::post().to(handlers::ops::diagnostic_trigger_job_failure))
            )
    );
}
