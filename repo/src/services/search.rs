use crate::config::{AppConfig, DbPool};
use crate::db_instrumentation::timed_query;
use crate::errors::AppError;
use crate::models::PaginationParams;
use crate::schema::*;
use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub sort: Option<String>,
    pub channel: Option<Uuid>,
    pub tag: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub item_id: Uuid,
    pub auto_number: String,
    pub title: String,
    pub status: String,
    pub channel_id: Uuid,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
pub struct SuggestionResult {
    pub term: String,
    pub frequency: i64,
}

#[derive(Serialize)]
pub struct TrendingResult {
    pub term: String,
    pub frequency: i32,
    pub computed_date: chrono::NaiveDate,
}

#[derive(Serialize)]
pub struct HistoryEntry {
    pub id: Uuid,
    pub query: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(QueryableByName)]
struct SearchRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub item_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub auto_number: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub title: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub status: String,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub channel_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    pub created_at: chrono::NaiveDateTime,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub cnt: i64,
}

pub fn search_items(pool: &DbPool, config: &AppConfig, params: &SearchParams, user_id: Uuid) -> Result<(Vec<SearchResult>, i64), AppError> {
    let conn = &mut pool.get()?;
    let threshold = config.slow_query_threshold_ms;
    let q = params.q.as_deref().unwrap_or("");
    let normalized = crate::search::normalize_query(q);

    // Record search + history
    if !q.is_empty() {
        diesel::insert_into(searches::table)
            .values((
                searches::id.eq(Uuid::new_v4()),
                searches::user_id.eq(Some(user_id)),
                searches::query_raw.eq(q),
                searches::query_normalized.eq(&normalized),
                searches::channel_filter.eq(params.channel),
                searches::tag_filter.eq(params.tag.as_deref()),
                searches::created_at.eq(Utc::now().naive_utc()),
                searches::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn).ok();

        diesel::insert_into(search_history::table)
            .values((
                search_history::id.eq(Uuid::new_v4()),
                search_history::user_id.eq(user_id),
                search_history::query.eq(q),
                search_history::created_at.eq(Utc::now().naive_utc()),
                search_history::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn).ok();

        trim_search_history(conn, user_id, 200);
    }

    // Tag pre-filter
    if let Some(ref tag_name) = params.tag {
        let tag_lower = tag_name.trim().to_lowercase();
        let count: i64 = item_version_tags::table
            .inner_join(tags::table)
            .filter(tags::name.eq(&tag_lower))
            .count()
            .get_result(conn)
            .unwrap_or(0);
        if count == 0 {
            return Ok((vec![], 0));
        }
    }

    let page = params.page.unwrap_or(1).max(1) - 1;
    let page_size = params.page_size.unwrap_or(20).min(100).max(1);

    // Build tsquery safely
    let tsquery_str = if !normalized.is_empty() {
        let ts = crate::search::to_tsquery(&normalized);
        if !ts.is_empty() { Some(ts) } else { None }
    } else { None };

    let use_relevance = params.sort.as_deref() == Some("relevance") && tsquery_str.is_some();

    // Use fully parameterized sql_query with fixed bind positions.
    // All 7 params are always bound; unused filters use trivially-true SQL conditions.
    //   $1 = channel_id (Uuid),  condition: ($1 IS NULL OR items.channel_id = $1)
    //   $2 = tag_name (Text),    condition: ($2 = '' OR items.current_version_id IN (...tag subquery...))
    //   $3 = from_ts (Timestamp),condition: ($3 = '0001-01-01' OR items.created_at >= $3)
    //   $4 = to_ts (Timestamp),  condition: ($4 = '0001-01-01' OR items.created_at <= $4)
    //   $5 = tsquery (Text),     condition: ($5 = '' OR search_vector @@ to_tsquery('english', $5))
    //   $6 = offset (BigInt)
    //   $7 = limit (BigInt)

    let channel_bind: Option<Uuid> = params.channel;
    let tag_bind: String = params.tag.as_ref().map(|t| t.trim().to_lowercase()).unwrap_or_default();
    let epoch = chrono::NaiveDateTime::parse_from_str("0001-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let from_bind = params.from.as_ref()
        .and_then(|f| chrono::NaiveDateTime::parse_from_str(&format!("{} 00:00:00", f), "%Y-%m-%d %H:%M:%S").ok())
        .unwrap_or(epoch);
    let to_bind = params.to.as_ref()
        .and_then(|t| chrono::NaiveDateTime::parse_from_str(&format!("{} 23:59:59", t), "%Y-%m-%d %H:%M:%S").ok())
        .unwrap_or(epoch);
    let ts_bind = tsquery_str.clone().unwrap_or_default();

    let base_where = "\
        items.current_version_id = item_versions.id \
        AND ($1::uuid IS NULL OR items.channel_id = $1) \
        AND ($2 = '' OR items.current_version_id IN (SELECT ivt.item_version_id FROM item_version_tags ivt JOIN tags t ON ivt.tag_id = t.id WHERE t.name = $2)) \
        AND ($3 = '0001-01-01'::timestamp OR items.created_at >= $3) \
        AND ($4 = '0001-01-01'::timestamp OR items.created_at <= $4) \
        AND ($5 = '' OR item_versions.search_vector @@ to_tsquery('english', $5))";

    let order_clause = if use_relevance {
        "ts_rank(item_versions.search_vector, to_tsquery('english', $5)) DESC"
    } else {
        "items.created_at DESC"
    };

    let count_sql = format!(
        "SELECT COUNT(*)::bigint AS cnt FROM items INNER JOIN item_versions ON items.current_version_id = item_versions.id WHERE {}",
        base_where
    );

    let select_sql = format!(
        "SELECT items.id AS item_id, items.auto_number, item_versions.title, \
         items.status, items.channel_id, items.created_at \
         FROM items INNER JOIN item_versions ON items.current_version_id = item_versions.id \
         WHERE {} ORDER BY {} OFFSET $6 LIMIT $7",
        base_where, order_clause
    );

    let total: i64 = timed_query("search_items_count", threshold, || {
        diesel::sql_query(&count_sql)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(channel_bind)
            .bind::<diesel::sql_types::Text, _>(&tag_bind)
            .bind::<diesel::sql_types::Timestamp, _>(from_bind)
            .bind::<diesel::sql_types::Timestamp, _>(to_bind)
            .bind::<diesel::sql_types::Text, _>(&ts_bind)
            .get_result::<CountRow>(conn)
            .map(|r| r.cnt)
            .unwrap_or(0)
    });

    let rows: Vec<SearchRow> = timed_query("search_items_fetch", threshold, || {
        diesel::sql_query(&select_sql)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(channel_bind)
            .bind::<diesel::sql_types::Text, _>(&tag_bind)
            .bind::<diesel::sql_types::Timestamp, _>(from_bind)
            .bind::<diesel::sql_types::Timestamp, _>(to_bind)
            .bind::<diesel::sql_types::Text, _>(&ts_bind)
            .bind::<diesel::sql_types::BigInt, _>(page * page_size)
            .bind::<diesel::sql_types::BigInt, _>(page_size)
            .load(conn)
    })?;

    let items_out = rows.into_iter().map(|r| SearchResult {
        item_id: r.item_id, auto_number: r.auto_number, title: r.title,
        status: r.status, channel_id: r.channel_id, created_at: r.created_at,
    }).collect();

    Ok((items_out, total))
}

#[derive(QueryableByName)]
struct SuggestionRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub term: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub frequency: i64,
}

pub fn get_suggestions(pool: &DbPool, prefix: &str, limit: i64) -> Result<Vec<SuggestionResult>, AppError> {
    let conn = &mut pool.get()?;
    let since = Utc::now().naive_utc() - chrono::Duration::days(30);
    let prefix_pattern = format!("{}%", prefix.to_lowercase());

    let results: Vec<SuggestionRow> = diesel::sql_query(
        "SELECT query_normalized as term, COUNT(*) as frequency \
         FROM searches WHERE created_at >= $1 AND query_normalized LIKE $2 \
         GROUP BY query_normalized ORDER BY frequency DESC LIMIT $3"
    )
    .bind::<diesel::sql_types::Timestamp, _>(since)
    .bind::<diesel::sql_types::Text, _>(prefix_pattern)
    .bind::<diesel::sql_types::BigInt, _>(limit.min(20))
    .load(conn)
    .map_err(|e| AppError::Internal(format!("Suggestion query failed: {}", e)))?;

    Ok(results.into_iter().map(|r| SuggestionResult { term: r.term, frequency: r.frequency }).collect())
}

pub fn get_trending(pool: &DbPool, window_days: i64) -> Result<Vec<TrendingResult>, AppError> {
    let conn = &mut pool.get()?;
    let since = Utc::now().date_naive() - chrono::Duration::days(window_days);

    let results: Vec<(Uuid, String, i32, chrono::NaiveDate, chrono::NaiveDateTime)> = search_trending_daily::table
        .filter(search_trending_daily::computed_date.ge(since))
        .order(search_trending_daily::frequency.desc())
        .limit(50)
        .load(conn)?;

    Ok(results.into_iter().map(|(_, term, frequency, computed_date, _)| {
        TrendingResult { term, frequency, computed_date }
    }).collect())
}

pub fn get_history(pool: &DbPool, user_id: Uuid, pagination: &PaginationParams) -> Result<(Vec<HistoryEntry>, i64), AppError> {
    let conn = &mut pool.get()?;
    let total: i64 = search_history::table.filter(search_history::user_id.eq(user_id)).count().get_result(conn)?;

    let results: Vec<(Uuid, Uuid, String, chrono::NaiveDateTime, chrono::NaiveDateTime)> = search_history::table
        .filter(search_history::user_id.eq(user_id))
        .order(search_history::created_at.desc())
        .offset(pagination.offset())
        .limit(pagination.page_size())
        .load(conn)?;

    Ok((results.into_iter().map(|(id, _, query, created_at, _)| HistoryEntry { id, query, created_at }).collect(), total))
}

pub fn clear_history(pool: &DbPool, user_id: Uuid, before: Option<chrono::NaiveDateTime>) -> Result<i64, AppError> {
    let conn = &mut pool.get()?;
    let deleted = if let Some(before_ts) = before {
        diesel::delete(search_history::table.filter(search_history::user_id.eq(user_id)).filter(search_history::created_at.lt(before_ts))).execute(conn)? as i64
    } else {
        diesel::delete(search_history::table.filter(search_history::user_id.eq(user_id))).execute(conn)? as i64
    };
    Ok(deleted)
}

fn trim_search_history(conn: &mut PgConnection, user_id: Uuid, max_entries: i64) {
    diesel::sql_query(
        "DELETE FROM search_history WHERE id IN (SELECT id FROM search_history WHERE user_id = $1 ORDER BY created_at DESC OFFSET $2)"
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::BigInt, _>(max_entries)
    .execute(conn).ok();
}
