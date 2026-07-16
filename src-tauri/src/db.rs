use anyhow::Result;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

use crate::config::DbConfig;
use crate::models::*;

pub async fn connect(config: &DbConfig) -> Result<MySqlPool> {
    use sqlx::mysql::MySqlConnectOptions;

    let opts = MySqlConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .username(&config.user)
        .password(&config.password)
        .database(&config.database);

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    Ok(pool)
}

pub async fn list_records(
    pool: &MySqlPool,
    filter: RecordFilter,
) -> Result<PaginatedResult> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).max(1).min(200);
    let offset = (page - 1) * page_size;

    let mut where_conditions: Vec<String> = Vec::new();
    let mut search_param: Option<String> = None;
    let mut type_param: Option<String> = None;
    let mut status_param: Option<String> = None;

    if let Some(ref s) = filter.search {
        if !s.is_empty() {
            search_param = Some(format!("%{}%", s));
            where_conditions.push("record_name LIKE ?".to_string());
        }
    }
    if let Some(ref t) = filter.media_type {
        if !t.is_empty() && t != "全部" {
            type_param = Some(t.clone());
            where_conditions.push("`type` = ?".to_string());
        }
    }
    if let Some(ref s) = filter.status {
        if !s.is_empty() && s != "全部" {
            status_param = Some(s.clone());
            where_conditions.push("status = ?".to_string());
        }
    }

    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    // Count query
    let count_sql = format!("SELECT COUNT(*) FROM `process` {}", where_clause);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(ref v) = search_param { count_query = count_query.bind(v); }
    if let Some(ref v) = type_param { count_query = count_query.bind(v); }
    if let Some(ref v) = status_param { count_query = count_query.bind(v); }
    let total: (i64,) = count_query.fetch_one(pool).await?;

    // Data query
    let data_sql = format!(
        "SELECT id, record_name, season, remark, `type`, status, end_time, country, tags, current_episode, total_episode, CAST(year AS UNSIGNED) as year, modify_time FROM `process` {} ORDER BY modify_time DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut data_query = sqlx::query_as::<_, RecordRow>(&data_sql);
    if let Some(ref v) = search_param { data_query = data_query.bind(v); }
    if let Some(ref v) = type_param { data_query = data_query.bind(v); }
    if let Some(ref v) = status_param { data_query = data_query.bind(v); }
    data_query = data_query.bind(page_size).bind(offset);
    let records: Vec<RecordRow> = data_query.fetch_all(pool).await?;

    Ok(PaginatedResult {
        total: total.0,
        records: records.into_iter().map(|r| r.into()).collect(),
    })
}

pub async fn get_record(pool: &MySqlPool, id: i64) -> Result<Option<Record>> {
    let row: Option<RecordRow> = sqlx::query_as(
        "SELECT id, record_name, season, remark, `type`, status, end_time, country, tags, current_episode, total_episode, CAST(year AS UNSIGNED) as year, modify_time FROM `process` WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.into()))
}

pub async fn add_record(pool: &MySqlPool, record: NewRecord) -> Result<i64> {
    let end_time = if record.status.as_deref() == Some("已完成") {
        Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    } else {
        record.end_time
    };

    let result = sqlx::query(
        r#"INSERT INTO `process` 
        (record_name, season, remark, `type`, status, end_time, country, tags, current_episode, total_episode, `year`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&record.record_name)
    .bind(record.season)
    .bind(&record.remark)
    .bind(&record.media_type)
    .bind(&record.status)
    .bind(&end_time)
    .bind(&record.country)
    .bind(&record.tags)
    .bind(record.current_episode)
    .bind(record.total_episode)
    .bind(record.year)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn update_record(pool: &MySqlPool, record: UpdateRecord) -> Result<bool> {
    let end_time = match record.status.as_deref() {
        Some("已完成") => Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        _ => None,
    };

    let rows = sqlx::query(
        r#"UPDATE `process` SET 
        record_name=?, season=?, remark=?, `type`=?, status=?, 
        end_time=?, country=?, tags=?, current_episode=?, total_episode=?, `year`=?
        WHERE id=?"#,
    )
    .bind(&record.record_name)
    .bind(record.season)
    .bind(&record.remark)
    .bind(&record.media_type)
    .bind(&record.status)
    .bind(&end_time)
    .bind(&record.country)
    .bind(&record.tags)
    .bind(record.current_episode)
    .bind(record.total_episode)
    .bind(record.year)
    .bind(record.id)
    .execute(pool)
    .await?;

    Ok(rows.rows_affected() > 0)
}

pub async fn delete_record(pool: &MySqlPool, id: i64) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM `process` WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(rows.rows_affected() > 0)
}

pub async fn get_stats(pool: &MySqlPool) -> Result<Stats> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM `process`")
        .fetch_one(pool)
        .await?;

    let by_type: Vec<(String, i64)> =
        sqlx::query_as("SELECT COALESCE(`type`,'未知') as t, COUNT(*) as c FROM `process` GROUP BY `type` ORDER BY c DESC")
            .fetch_all(pool)
            .await?;

    let by_status: Vec<(String, i64)> =
        sqlx::query_as("SELECT COALESCE(status,'未知') as s, COUNT(*) as c FROM `process` GROUP BY status ORDER BY c DESC")
            .fetch_all(pool)
            .await?;

    Ok(Stats {
        total: total.0,
        by_type: by_type
            .into_iter()
            .map(|(t, c)| TypeCount {
                media_type: t,
                count: c,
            })
            .collect(),
        by_status: by_status
            .into_iter()
            .map(|(s, c)| StatusCount {
                status: s,
                count: c,
            })
            .collect(),
    })
}

#[derive(sqlx::FromRow)]
struct RecordRow {
    pub id: i64,
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
    #[sqlx(rename = "type")]
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub end_time: Option<chrono::NaiveDateTime>,
    pub country: Option<String>,
    pub tags: Option<String>,
    pub current_episode: Option<i32>,
    pub total_episode: Option<i32>,
    pub year: Option<u64>,
    pub modify_time: Option<chrono::NaiveDateTime>,
}

impl From<RecordRow> for Record {
    fn from(r: RecordRow) -> Self {
        Self {
            id: r.id,
            record_name: r.record_name,
            season: r.season,
            remark: r.remark,
            media_type: r.media_type,
            status: r.status,
            end_time: r.end_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            country: r.country,
            tags: r.tags,
            current_episode: r.current_episode,
            total_episode: r.total_episode,
            year: r.year.map(|y| y as i32),
            modify_time: r
                .modify_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}
