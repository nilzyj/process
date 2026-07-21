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
        .acquire_timeout(std::time::Duration::from_secs(8))
        .connect_with(opts)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接超时(8s): {}", e))?;

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
        "SELECT id, record_name, season, remark, `type`, status, end_time, country, tags, current_episode, total_episode, CAST(year AS UNSIGNED) as year, modify_time FROM `process` {} ORDER BY COALESCE(end_time, '1970-01-01') DESC, modify_time DESC LIMIT ? OFFSET ?",
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
    let end_time = match (&record.end_time, record.status.as_deref()) {
        (Some(t), _) if !t.is_empty() => Some(t.clone()),
        (_, Some("已完成")) => Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
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

    let by_year: Vec<(i32, i64)> =
        sqlx::query_as("SELECT CAST(year AS SIGNED) as y, COUNT(*) FROM `process` WHERE year IS NOT NULL GROUP BY y ORDER BY y")
            .fetch_all(pool)
            .await?;

    let by_country: Vec<(String, i64)> =
        sqlx::query_as("SELECT country, COUNT(*) FROM `process` WHERE country IS NOT NULL AND country != '' GROUP BY country ORDER BY COUNT(*) DESC")
            .fetch_all(pool)
            .await?;

    let type_status: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT COALESCE(`type`,'未知') as t, COALESCE(status,'未知') as s, COUNT(*) as c FROM `process` GROUP BY `type`, status ORDER BY t, s")
            .fetch_all(pool)
            .await?;

    let progress_rows: Vec<(Option<i32>, Option<i32>)> =
        sqlx::query_as("SELECT current_episode, total_episode FROM `process` WHERE current_episode IS NOT NULL AND total_episode IS NOT NULL AND total_episode > 0")
            .fetch_all(pool)
            .await?;

    let tags_rows: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT tags FROM `process` WHERE tags IS NOT NULL AND tags != ''")
            .fetch_all(pool)
            .await?;

    let daily: Vec<(String, i64)> =
        sqlx::query_as("SELECT DATE_FORMAT(modify_time, '%Y-%m-%d') as day, COUNT(*) as c FROM `process` WHERE modify_time >= DATE_SUB(CURDATE(), INTERVAL 1 YEAR) GROUP BY DATE_FORMAT(modify_time, '%Y-%m-%d') ORDER BY day")
            .fetch_all(pool)
            .await?;

    let (new_today,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM `process` WHERE DATE(modify_time) = CURDATE()")
            .fetch_one(pool)
            .await?;
    let (new_week,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM `process` WHERE modify_time >= DATE_SUB(CURDATE(), INTERVAL 7 DAY)")
            .fetch_one(pool)
            .await?;
    let (new_month,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM `process` WHERE modify_time >= DATE_SUB(CURDATE(), INTERVAL 30 DAY)")
            .fetch_one(pool)
            .await?;
    let (completed_today,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM `process` WHERE status = '已完成' AND DATE(end_time) = CURDATE()")
            .fetch_one(pool)
            .await?;
    let (completed_week,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM `process` WHERE status = '已完成' AND end_time >= DATE_SUB(CURDATE(), INTERVAL 7 DAY)")
            .fetch_one(pool)
            .await?;
    let (completed_month,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM `process` WHERE status = '已完成' AND end_time >= DATE_SUB(CURDATE(), INTERVAL 30 DAY)")
            .fetch_one(pool)
            .await?;

    // Process progress buckets
    let mut buckets = vec![0i64; 5];
    for (cur, total) in &progress_rows {
        if let (Some(c), Some(t)) = (cur, total) {
            if *t > 0 {
                let pct = (*c as f64 / *t as f64) * 100.0;
                if pct >= 100.0 { buckets[4] += 1; }
                else if pct >= 75.0 { buckets[3] += 1; }
                else if pct >= 50.0 { buckets[2] += 1; }
                else if pct >= 25.0 { buckets[1] += 1; }
                else { buckets[0] += 1; }
            }
        }
    }

    // Process tags
    let mut tag_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for (t,) in &tags_rows {
        if let Some(tags_str) = t {
            for tag in tags_str.split(',') {
                let trimmed = tag.trim().to_string();
                if !trimmed.is_empty() {
                    *tag_map.entry(trimmed).or_insert(0) += 1;
                }
            }
        }
    }
    let mut by_tags: Vec<TagCount> = tag_map
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    by_tags.sort_by(|a, b| b.count.cmp(&a.count));

    // Compute completion rates per type
    let mut rate_map: std::collections::HashMap<String, (i64, i64)> = std::collections::HashMap::new();
    for (t, s, c) in &type_status {
        let entry = rate_map.entry(t.clone()).or_insert((0, 0));
        entry.1 += c;
        if s == "已完成" {
            entry.0 += c;
        }
    }
    let mut completion_rates: Vec<CompletionRate> = rate_map
        .into_iter()
        .map(|(media_type, (completed, total))| CompletionRate { media_type, completed, total })
        .collect();
    completion_rates.sort_by(|a, b| b.total.cmp(&a.total));

    Ok(Stats {
        total: total.0,
        by_type: by_type.into_iter().map(|(t, c)| TypeCount { media_type: t, count: c }).collect(),
        by_status: by_status.into_iter().map(|(s, c)| StatusCount { status: s, count: c }).collect(),
        by_year: by_year.into_iter().map(|(y, c)| YearCount { year: y, count: c }).collect(),
        by_country: by_country.into_iter().map(|(c, n)| CountryCount { country: c, count: n }).collect(),
        by_tags,
        progress_buckets: vec![
            ProgressBucket { label: "0-25%".into(), count: buckets[0] },
            ProgressBucket { label: "25-50%".into(), count: buckets[1] },
            ProgressBucket { label: "50-75%".into(), count: buckets[2] },
            ProgressBucket { label: "75-99%".into(), count: buckets[3] },
            ProgressBucket { label: "100%".into(), count: buckets[4] },
        ],
        type_status: type_status.into_iter().map(|(t, s, c)| TypeStatusCount { media_type: t, status: s, count: c }).collect(),
        completion_rates,
        daily_activity: daily.into_iter().map(|(d, c)| DailyActivity { date: d, count: c }).collect(),
        recent: RecentActivity {
            new_today, new_week, new_month,
            completed_today, completed_week, completed_month,
        },
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
