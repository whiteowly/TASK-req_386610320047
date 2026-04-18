use crate::config::{AppConfig, DbPool};
use chrono::Utc;
use std::time::Duration;

pub async fn run_scheduler(pool: DbPool, config: AppConfig) {
    log::info!("Background job scheduler started");
    let mut last_trending = Utc::now();
    let mut last_metrics = Utc::now();
    let mut last_cleanup = Utc::now();
    let mut last_auto_revert = Utc::now();

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let now = Utc::now();

        // Auto-revert check every 15 minutes
        if (now - last_auto_revert).num_minutes() >= 15 {
            if let Err(e) = run_auto_revert(&pool, &config) {
                log::error!("Auto-revert job failed: {}", e);
                crate::alerts::write_alert(
                    &config.alerts_spool_dir,
                    "JOB_FAILURE",
                    "Auto-revert job failed",
                    &serde_json::json!({"error": e}),
                );
            }
            last_auto_revert = now;
        }

        // Trending recompute daily
        if (now - last_trending).num_hours() >= 24 {
            if let Err(e) = run_trending_recompute(&pool, &config) {
                log::error!("Trending recompute failed: {}", e);
                crate::alerts::write_alert(
                    &config.alerts_spool_dir,
                    "JOB_FAILURE",
                    "Trending recompute job failed",
                    &serde_json::json!({"error": e}),
                );
            }
            last_trending = now;
        }

        // Metrics snapshot every hour
        if (now - last_metrics).num_hours() >= 1 {
            if let Err(e) = run_metrics_snapshot(&pool) {
                log::error!("Metrics snapshot failed: {}", e);
                crate::alerts::write_alert(
                    &config.alerts_spool_dir,
                    "JOB_FAILURE",
                    "Metrics snapshot job failed",
                    &serde_json::json!({"error": e}),
                );
            }
            last_metrics = now;
        }

        // Retention cleanup daily
        if (now - last_cleanup).num_hours() >= 24 {
            if let Err(e) = run_retention_cleanup(&pool, &config) {
                log::error!("Retention cleanup failed: {}", e);
                crate::alerts::write_alert(
                    &config.alerts_spool_dir,
                    "JOB_FAILURE",
                    "Retention cleanup job failed",
                    &serde_json::json!({"error": e}),
                );
            }
            last_cleanup = now;
        }

        // Process pending standardization jobs
        if let Err(e) = process_pending_jobs(&pool, &config) {
            log::error!("Job processing failed: {}", e);
            crate::alerts::write_alert(
                &config.alerts_spool_dir,
                "JOB_FAILURE",
                "Standardization job processing failed",
                &serde_json::json!({"error": e}),
            );
        }
    }
}

fn run_auto_revert(pool: &DbPool, config: &AppConfig) -> Result<(), String> {
    use crate::schema::{audits as _audits, items};
    use diesel::prelude::*;

    let conn = &mut pool.get().map_err(|e| e.to_string())?;
    let cutoff = Utc::now() - chrono::Duration::days(config.auto_revert_idle_days);

    let stale_items: Vec<(uuid::Uuid, Option<chrono::NaiveDateTime>)> = items::table
        .filter(items::status.eq("InReview"))
        .filter(items::entered_in_review_at.le(Some(cutoff.naive_utc())))
        .select((items::id, items::entered_in_review_at))
        .load(conn)
        .map_err(|e| e.to_string())?;

    for (item_id, _) in &stale_items {
        diesel::update(items::table.filter(items::id.eq(item_id)))
            .set((
                items::status.eq("Draft"),
                items::entered_in_review_at.eq(None::<chrono::NaiveDateTime>),
                items::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn)
            .map_err(|e| e.to_string())?;

        crate::audit::log_audit(
            pool,
            None,
            "system",
            "AUTO_REVERT_IDLE_REVIEW",
            "item",
            Some(*item_id),
            None,
            Some(serde_json::json!({"new_status": "Draft"})),
            Some("Reverted after 14 days idle in review"),
            None,
        );
    }

    if !stale_items.is_empty() {
        log::info!(
            "Auto-reverted {} items from InReview to Draft",
            stale_items.len()
        );
    }
    Ok(())
}

fn run_trending_recompute(pool: &DbPool, config: &AppConfig) -> Result<(), String> {
    use crate::schema::{search_trending_daily, searches};
    use diesel::prelude::*;

    let conn = &mut pool.get().map_err(|e| e.to_string())?;
    let since = Utc::now() - chrono::Duration::days(config.trending_window_days);
    let today = Utc::now().date_naive();

    // Get normalized queries from last N days
    let queries: Vec<String> = searches::table
        .filter(searches::created_at.ge(since.naive_utc()))
        .select(searches::query_normalized)
        .load(conn)
        .map_err(|e| e.to_string())?;

    // Count term frequencies (word-level)
    let mut freq: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "it", "in", "on", "at", "to", "for", "of", "and", "or", "not",
    ]
    .iter()
    .cloned()
    .collect();

    for q in &queries {
        for word in q.split_whitespace() {
            let w = word.to_lowercase();
            if w.len() > 2 && !stop_words.contains(w.as_str()) {
                *freq.entry(w).or_insert(0) += 1;
            }
        }
    }

    // Delete today's entries and insert fresh
    diesel::delete(
        search_trending_daily::table
            .filter(search_trending_daily::computed_date.eq(today)),
    )
    .execute(conn)
    .map_err(|e| e.to_string())?;

    let mut terms: Vec<_> = freq.into_iter().collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1));

    for (term, count) in terms.into_iter().take(100) {
        diesel::insert_into(search_trending_daily::table)
            .values((
                search_trending_daily::id.eq(uuid::Uuid::new_v4()),
                search_trending_daily::term.eq(&term),
                search_trending_daily::frequency.eq(count),
                search_trending_daily::computed_date.eq(today),
                search_trending_daily::created_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn)
            .ok();
    }

    log::info!("Trending terms recomputed for {}", today);
    Ok(())
}

fn run_metrics_snapshot(pool: &DbPool) -> Result<(), String> {
    use crate::schema::{items, metrics_snapshots, users};
    use diesel::prelude::*;

    let conn = &mut pool.get().map_err(|e| e.to_string())?;

    let item_count: i64 = items::table
        .count()
        .get_result(conn)
        .map_err(|e| e.to_string())?;
    let user_count: i64 = users::table
        .count()
        .get_result(conn)
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    diesel::insert_into(metrics_snapshots::table)
        .values((
            metrics_snapshots::id.eq(uuid::Uuid::new_v4()),
            metrics_snapshots::snapshot_type.eq("hourly"),
            metrics_snapshots::time_range
                .eq(serde_json::json!({"at": now.to_rfc3339()})),
            metrics_snapshots::metrics.eq(serde_json::json!({
                "total_items": item_count,
                "total_users": user_count,
            })),
            metrics_snapshots::created_at.eq(now.naive_utc()),
        ))
        .execute(conn)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn run_retention_cleanup(pool: &DbPool, config: &AppConfig) -> Result<(), String> {
    use crate::schema::{audits, sessions};
    use diesel::prelude::*;

    let conn = &mut pool.get().map_err(|e| e.to_string())?;

    // Clean expired sessions
    let expired = Utc::now() - chrono::Duration::hours(config.session_inactivity_hours);
    diesel::delete(
        sessions::table.filter(sessions::last_activity_at.lt(expired.naive_utc())),
    )
    .execute(conn)
    .map_err(|e| e.to_string())?;

    // Audit retention
    let retention_cutoff =
        Utc::now() - chrono::Duration::days(config.audit_retention_years * 365);
    diesel::delete(
        audits::table.filter(audits::created_at.lt(retention_cutoff.naive_utc())),
    )
    .execute(conn)
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn process_pending_jobs(pool: &DbPool, config: &AppConfig) -> Result<(), String> {
    use crate::schema::standardization_jobs;
    use diesel::prelude::*;

    let conn = &mut pool.get().map_err(|e| e.to_string())?;

    let pending: Vec<(uuid::Uuid, uuid::Uuid)> = standardization_jobs::table
        .filter(standardization_jobs::status.eq("queued"))
        .select((
            standardization_jobs::id,
            standardization_jobs::mapping_version_id,
        ))
        .order(standardization_jobs::created_at.asc())
        .limit(5)
        .load(conn)
        .map_err(|e| e.to_string())?;

    for (job_id, mapping_version_id) in pending {
        if let Err(e) =
            crate::standardization::execute_job(pool, config, job_id, mapping_version_id)
        {
            log::error!("Standardization job {} failed: {}", job_id, e);
            diesel::update(
                standardization_jobs::table
                    .filter(standardization_jobs::id.eq(job_id)),
            )
            .set((
                standardization_jobs::status.eq("failed"),
                standardization_jobs::error_info.eq(Some(e.to_string())),
                standardization_jobs::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn)
            .ok();
        }
    }

    Ok(())
}
