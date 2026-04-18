// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tsvector", schema = "pg_catalog"))]
    pub struct Tsvector;
}

diesel::table! {
    audits (id) {
        id -> Uuid,
        actor_id -> Nullable<Uuid>,
        actor_username -> Nullable<Varchar>,
        action -> Varchar,
        object_type -> Varchar,
        object_id -> Nullable<Uuid>,
        before_state -> Nullable<Jsonb>,
        after_state -> Nullable<Jsonb>,
        reason -> Nullable<Text>,
        request_id -> Nullable<Varchar>,
        ip_address -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    captcha_challenges (id) {
        id -> Uuid,
        username -> Varchar,
        challenge_type -> Varchar,
        challenge_prompt -> Text,
        expected_answer -> Varchar,
        expires_at -> Timestamp,
        used -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    channels (id) {
        id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    daily_counters (id) {
        id -> Uuid,
        counter_date -> Date,
        last_sequence -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    events (id) {
        id -> Uuid,
        event_type -> Varchar,
        actor_id -> Nullable<Uuid>,
        payload -> Jsonb,
        occurred_at -> Timestamp,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    export_artifacts (id) {
        id -> Uuid,
        export_id -> Uuid,
        file_path -> Text,
        checksum -> Varchar,
        size_bytes -> Int8,
        masking_applied -> Bool,
        explanations_included -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    exports (id) {
        id -> Uuid,
        user_id -> Uuid,
        scope_filters -> Jsonb,
        format -> Varchar,
        include_explanations -> Bool,
        mask_sensitive -> Bool,
        status -> Varchar,
        artifact_path -> Nullable<Text>,
        artifact_checksum -> Nullable<Varchar>,
        artifact_size -> Nullable<Int8>,
        idempotency_key -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    feature_flags (id) {
        id -> Uuid,
        key -> Varchar,
        enabled -> Bool,
        variants -> Nullable<Jsonb>,
        allocation -> Nullable<Jsonb>,
        description -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    import_rows (id) {
        id -> Uuid,
        import_id -> Uuid,
        row_number -> Int4,
        status -> Varchar,
        item_id -> Nullable<Uuid>,
        errors -> Nullable<Jsonb>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    imports (id) {
        id -> Uuid,
        user_id -> Uuid,
        template_version_id -> Uuid,
        channel_id -> Uuid,
        filename -> Varchar,
        file_size -> Nullable<Int8>,
        status -> Varchar,
        total_rows -> Nullable<Int4>,
        accepted_rows -> Nullable<Int4>,
        rejected_rows -> Nullable<Int4>,
        options -> Nullable<Jsonb>,
        idempotency_key -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    item_version_tags (id) {
        id -> Uuid,
        item_version_id -> Uuid,
        tag_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Tsvector;

    item_versions (id) {
        id -> Uuid,
        item_id -> Uuid,
        version_number -> Int4,
        template_version_id -> Uuid,
        title -> Text,
        body -> Nullable<Text>,
        fields -> Jsonb,
        sensitive_fields_encrypted -> Nullable<Text>,
        change_note -> Nullable<Text>,
        created_by -> Uuid,
        rollback_source_version_id -> Nullable<Uuid>,
        search_vector -> Nullable<Tsvector>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    items (id) {
        id -> Uuid,
        template_id -> Uuid,
        channel_id -> Uuid,
        owner_user_id -> Uuid,
        auto_number -> Varchar,
        status -> Varchar,
        current_version_id -> Nullable<Uuid>,
        published_at -> Nullable<Timestamp>,
        published_version_id -> Nullable<Uuid>,
        published_template_version_id -> Nullable<Uuid>,
        entered_in_review_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    login_attempts (id) {
        id -> Uuid,
        username -> Varchar,
        ip_address -> Nullable<Varchar>,
        success -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    metrics_snapshots (id) {
        id -> Uuid,
        snapshot_type -> Varchar,
        time_range -> Jsonb,
        dimensions -> Nullable<Jsonb>,
        metrics -> Jsonb,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    rate_limits (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        ip_address -> Nullable<Varchar>,
        window_start -> Timestamp,
        request_count -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    roles (id) {
        id -> Uuid,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    schema_mapping_versions (id) {
        id -> Uuid,
        mapping_id -> Uuid,
        version_number -> Int4,
        mapping_rules -> Jsonb,
        explicit_defaults -> Nullable<Jsonb>,
        unit_rules -> Nullable<Jsonb>,
        timezone_rules -> Nullable<Jsonb>,
        fingerprint_keys -> Nullable<Jsonb>,
        pii_fields -> Nullable<Jsonb>,
        change_note -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    schema_mappings (id) {
        id -> Uuid,
        name -> Varchar,
        source_scope -> Nullable<Text>,
        description -> Nullable<Text>,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    search_history (id) {
        id -> Uuid,
        user_id -> Uuid,
        query -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    search_trending_daily (id) {
        id -> Uuid,
        term -> Varchar,
        frequency -> Int4,
        computed_date -> Date,
        created_at -> Timestamp,
    }
}

diesel::table! {
    searches (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        query_raw -> Text,
        query_normalized -> Text,
        channel_filter -> Nullable<Uuid>,
        tag_filter -> Nullable<Text>,
        result_count -> Nullable<Int4>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        token_hash -> Varchar,
        last_activity_at -> Timestamp,
        expires_at -> Timestamp,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    standardization_jobs (id) {
        id -> Uuid,
        mapping_version_id -> Uuid,
        source_filters -> Nullable<Jsonb>,
        run_label -> Nullable<Varchar>,
        status -> Varchar,
        total_records -> Nullable<Int4>,
        processed_records -> Nullable<Int4>,
        failed_records -> Nullable<Int4>,
        retry_count -> Int4,
        error_info -> Nullable<Text>,
        idempotency_key -> Nullable<Varchar>,
        started_at -> Nullable<Timestamp>,
        completed_at -> Nullable<Timestamp>,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    standardized_models (id) {
        id -> Uuid,
        job_id -> Uuid,
        mapping_version_id -> Uuid,
        version_number -> Int4,
        source_window -> Nullable<Jsonb>,
        quality_stats -> Nullable<Jsonb>,
        record_count -> Int4,
        created_at -> Timestamp,
    }
}

diesel::table! {
    standardized_records (id) {
        id -> Uuid,
        model_id -> Uuid,
        source_item_id -> Nullable<Uuid>,
        fingerprint -> Varchar,
        raw_values -> Jsonb,
        standardized_values -> Jsonb,
        transformations_applied -> Nullable<Jsonb>,
        outlier_flags -> Nullable<Jsonb>,
        is_duplicate -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tags (id) {
        id -> Uuid,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    template_versions (id) {
        id -> Uuid,
        template_id -> Uuid,
        version_number -> Int4,
        field_schema -> Jsonb,
        constraints_schema -> Nullable<Jsonb>,
        cross_field_rules -> Nullable<Jsonb>,
        change_note -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    templates (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        channel_scope -> Nullable<Uuid>,
        active_version_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Citext,
        password_hash -> Text,
        email_encrypted -> Nullable<Text>,
        phone_encrypted -> Nullable<Text>,
        role_id -> Uuid,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(export_artifacts -> exports (export_id));
diesel::joinable!(exports -> users (user_id));
diesel::joinable!(import_rows -> imports (import_id));
diesel::joinable!(imports -> channels (channel_id));
diesel::joinable!(imports -> template_versions (template_version_id));
diesel::joinable!(imports -> users (user_id));
diesel::joinable!(item_version_tags -> item_versions (item_version_id));
diesel::joinable!(item_version_tags -> tags (tag_id));
diesel::joinable!(item_versions -> template_versions (template_version_id));
diesel::joinable!(item_versions -> users (created_by));
diesel::joinable!(items -> channels (channel_id));
diesel::joinable!(items -> template_versions (published_template_version_id));
diesel::joinable!(items -> templates (template_id));
diesel::joinable!(items -> users (owner_user_id));
diesel::joinable!(rate_limits -> users (user_id));
diesel::joinable!(schema_mapping_versions -> schema_mappings (mapping_id));
diesel::joinable!(schema_mappings -> users (created_by));
diesel::joinable!(search_history -> users (user_id));
diesel::joinable!(searches -> users (user_id));
diesel::joinable!(sessions -> users (user_id));
diesel::joinable!(standardization_jobs -> schema_mapping_versions (mapping_version_id));
diesel::joinable!(standardization_jobs -> users (created_by));
diesel::joinable!(standardized_models -> schema_mapping_versions (mapping_version_id));
diesel::joinable!(standardized_models -> standardization_jobs (job_id));
diesel::joinable!(standardized_records -> items (source_item_id));
diesel::joinable!(standardized_records -> standardized_models (model_id));
diesel::joinable!(templates -> channels (channel_scope));
diesel::joinable!(users -> roles (role_id));

diesel::allow_tables_to_appear_in_same_query!(
    audits,captcha_challenges,channels,daily_counters,events,export_artifacts,exports,feature_flags,import_rows,imports,item_version_tags,item_versions,items,login_attempts,metrics_snapshots,rate_limits,roles,schema_mapping_versions,schema_mappings,search_history,search_trending_daily,searches,sessions,standardization_jobs,standardized_models,standardized_records,tags,template_versions,templates,users,);
