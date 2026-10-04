use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::models::*;

/// 时间统一格式。SQLite 无原生日期类型，一律以 TEXT 存储，
/// 写入必须全部经过本常量，否则 date() 与字符串比较会出错。
pub const TS_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn now_str() -> String {
    Local::now().format(TS_FORMAT).to_string()
}

/// 统计窗口的日期边界，在 Rust 侧按本地时区算好后绑定进 SQL。
///
/// MySQL 的 NOW()/CURDATE() 走会话时区（CST），SQLite 内置的 datetime('now')
/// 是 UTC。若直接翻译方言会让「今日新增」整体偏移 8 小时。
/// 语义对齐要点：CURDATE() 是日期，DATE_SUB 后按 00:00:00 参与 datetime 比较。
pub struct DateWindow {
    pub today: String,
    pub week_ago_midnight: String,
    pub month_ago_midnight: String,
}

impl DateWindow {
    pub fn build() -> Self {
        Self::from_date(Local::now().date_naive())
    }

    pub fn from_date(today: NaiveDate) -> Self {
        let midnight = |d: NaiveDate| {
            d.and_hms_opt(0, 0, 0)
                .unwrap()
                .format(TS_FORMAT)
                .to_string()
        };
        Self {
            today: today.format("%Y-%m-%d").to_string(),
            week_ago_midnight: midnight(today - Duration::days(7)),
            month_ago_midnight: midnight(today - Duration::days(30)),
        }
    }
}

/// 打开（或创建）本地 SQLite 并确保 schema 就绪。
/// 数据库刻意不放在坚果云同步目录：SQLite 是 .db + -wal + -shm 多文件，
/// WAL 频繁变动会导致同步出损坏状态。
pub async fn connect(config: &AppConfig) -> Result<SqlitePool> {
    crate::config::ensure_dir(&config.data_dir)?;

    let opts = SqliteConnectOptions::new()
        .filename(config.database_path())
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(60))
        .connect_with(opts)
        .await?;

    init_schema(&pool).await?;
    Ok(pool)
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS records (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  record_name       TEXT    NOT NULL DEFAULT '',
  season            INTEGER,
  remark            TEXT    DEFAULT '',
  `type`            TEXT,
  status            TEXT,
  end_time          TEXT,
  country           TEXT,
  tags              TEXT    DEFAULT '',
  current_episode   INTEGER,
  total_episode     INTEGER,
  `year`            INTEGER,
  modify_time       TEXT    NOT NULL
);

-- 旧 MySQL 表只有主键，以下索引为新增
CREATE INDEX IF NOT EXISTS idx_records_end_time   ON records(end_time DESC);
CREATE INDEX IF NOT EXISTS idx_records_modify_time ON records(modify_time DESC);
CREATE INDEX IF NOT EXISTS idx_records_type       ON records(`type`);
CREATE INDEX IF NOT EXISTS idx_records_status     ON records(status);

PRAGMA user_version = 1;
"#;

async fn init_schema(pool: &SqlitePool) -> Result<()> {
    for stmt in SCHEMA.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

const SELECT_COLS: &str = "id, record_name, season, remark, `type`, status, end_time, \
                           country, tags, current_episode, total_episode, \
                           CAST(`year` AS INTEGER) as year, modify_time";

pub async fn list_records(pool: &SqlitePool, filter: RecordFilter) -> Result<PaginatedResult> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).max(1).min(200);
    let offset = (page - 1) * page_size;

    let mut where_conditions: Vec<String> = Vec::new();
    let mut search_param: Option<String> = None;
    let mut type_param: Option<String> = None;
    let mut status_param: Option<String> = None;
    let mut tag_param: Option<String> = None;
    let mut end_start_param: Option<String> = None;
    let mut end_end_param: Option<String> = None;

    if let Some(ref s) = filter.search {
        if !s.is_empty() {
            search_param = Some(format!("%{}%", s));
            // MySQL 的 utf8mb4_0900_ai_ci 不区分大小写，
            // SQLite 的 LIKE 默认对 ASCII 也不敏感，但对非 ASCII 严格。
            // 显式 COLLATE NOCASE 统一为「不区分大小写」。
            where_conditions.push("record_name LIKE ? COLLATE NOCASE".to_string());
        }
    }
    if let Some(ref t) = filter.media_type {
        if !t.is_empty() && t != "全部" {
            type_param = Some(t.clone());
            where_conditions.push("`type` = ? COLLATE NOCASE".to_string());
        }
    }
    if let Some(ref s) = filter.status {
        if !s.is_empty() && s != "全部" {
            status_param = Some(s.clone());
            where_conditions.push("status = ? COLLATE NOCASE".to_string());
        }
    }
    if let Some(ref t) = filter.tag {
        if !t.is_empty() {
            tag_param = Some(format!("%{}%", t));
            where_conditions.push("tags LIKE ? COLLATE NOCASE".to_string());
        }
    }
    if let Some(ref d) = filter.end_time_start {
        if !d.is_empty() {
            end_start_param = Some(d.clone());
            where_conditions.push("end_time >= ?".to_string());
        }
    }
    if let Some(ref d) = filter.end_time_end {
        if !d.is_empty() {
            end_end_param = Some(d.clone());
            where_conditions.push("end_time < ?".to_string());
        }
    }

    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM records {}", where_clause);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
    for v in [
        &search_param,
        &type_param,
        &status_param,
        &tag_param,
        &end_start_param,
        &end_end_param,
    ] {
        if let Some(val) = v {
            count_query = count_query.bind(val);
        }
    }
    let total: (i64,) = count_query.fetch_one(pool).await?;

    let data_sql = format!(
        "SELECT {} FROM records {} ORDER BY COALESCE(end_time, '1970-01-01') DESC, modify_time DESC LIMIT ? OFFSET ?",
        SELECT_COLS, where_clause
    );
    let mut data_query = sqlx::query_as::<_, RecordRow>(&data_sql);
    for v in [
        &search_param,
        &type_param,
        &status_param,
        &tag_param,
        &end_start_param,
        &end_end_param,
    ] {
        if let Some(val) = v {
            data_query = data_query.bind(val);
        }
    }
    data_query = data_query.bind(page_size).bind(offset);
    let records: Vec<RecordRow> = data_query.fetch_all(pool).await?;

    Ok(PaginatedResult {
        total: total.0,
        records: records.into_iter().map(|r| r.into()).collect(),
    })
}

pub async fn get_record(pool: &SqlitePool, id: i64) -> Result<Option<Record>> {
    let sql = format!("SELECT {} FROM records WHERE id = ?", SELECT_COLS);
    let row: Option<RecordRow> = sqlx::query_as(&sql).bind(id).fetch_optional(pool).await?;
    Ok(row.map(|r| r.into()))
}

pub async fn add_record(pool: &SqlitePool, record: NewRecord) -> Result<i64> {
    let now = now_str();
    let end_time = normalize_ts(&record.end_time);

    let row: (i64,) = sqlx::query_as(
        r#"INSERT INTO records
        (record_name, season, remark, `type`, status, end_time, country, tags,
         current_episode, total_episode, `year`, modify_time)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id"#,
    )
    .bind(&record.record_name)
    .bind(record.season)
    .bind(remark_or_default(&record.remark))
    .bind(&record.media_type)
    .bind(&record.status)
    .bind(&end_time)
    .bind(&record.country)
    .bind(tags_or_default(&record.tags))
    .bind(record.current_episode)
    .bind(record.total_episode)
    .bind(record.year)
    .bind(&now)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn update_record(pool: &SqlitePool, record: UpdateRecord) -> Result<bool> {
    let end_time = normalize_ts(&record.end_time);

    let rows = sqlx::query(
        r#"UPDATE records SET
        record_name=?, season=?, remark=?, `type`=?, status=?,
        end_time=?, country=?, tags=?, current_episode=?, total_episode=?, `year`=?,
        modify_time=?
        WHERE id=?"#,
    )
    .bind(&record.record_name)
    .bind(record.season)
    .bind(remark_or_default(&record.remark))
    .bind(&record.media_type)
    .bind(&record.status)
    .bind(&end_time)
    .bind(&record.country)
    .bind(tags_or_default(&record.tags))
    .bind(record.current_episode)
    .bind(record.total_episode)
    .bind(record.year)
    .bind(now_str())
    .bind(record.id)
    .execute(pool)
    .await?;

    Ok(rows.rows_affected() > 0)
}

pub async fn delete_record(pool: &SqlitePool, id: i64) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM records WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(rows.rows_affected() > 0)
}

pub async fn count_records(pool: &SqlitePool) -> Result<i64> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM records").fetch_one(pool).await?;
    Ok(n)
}

/// 按 id 升序导出全部记录，用于快照与迁移。
pub async fn all_records(pool: &SqlitePool) -> Result<Vec<Record>> {
    let sql = format!("SELECT {} FROM records ORDER BY id", SELECT_COLS);
    let rows: Vec<RecordRow> = sqlx::query_as(&sql).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// 整表替换：单事务内先清空再插入，中途失败则整体回滚，不会留下半份数据。
/// 保留 sqlite_sequence 使新 id 从 AUTOINCREMENT 序列继续，而非从头 1 开始。
pub async fn replace_all_records(
    pool: &SqlitePool,
    records: &[Record],
    seed_sequence: bool,
) -> Result<usize> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM records").execute(&mut *tx).await?;
    if seed_sequence {
        sqlx::query("DELETE FROM sqlite_sequence WHERE name = 'records'")
            .execute(&mut *tx)
            .await?;
    }

    for r in records {
        sqlx::query(
            r#"INSERT INTO records
            (id, record_name, season, remark, `type`, status, end_time, country, tags,
             current_episode, total_episode, `year`, modify_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(r.id)
        .bind(&r.record_name)
        .bind(r.season)
        .bind(remark_or_default(&r.remark))
        .bind(&r.media_type)
        .bind(&r.status)
        .bind(normalize_ts(&r.end_time))
        .bind(&r.country)
        .bind(tags_or_default(&r.tags))
        .bind(r.current_episode)
        .bind(r.total_episode)
        .bind(r.year)
        .bind(normalize_ts(&r.modify_time).unwrap_or_else(now_str))
        .execute(&mut *tx)
        .await?;
    }

    if seed_sequence {
        let max_id = records.iter().map(|r| r.id).max().unwrap_or(0);
        sqlx::query("INSERT INTO sqlite_sequence(name, seq) VALUES('records', ?)")
            .bind(max_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(records.len())
}

fn normalize_ts(v: &Option<String>) -> Option<String> {
    match v {
        Some(t) if !t.is_empty() => Some(t.clone()),
        _ => None,
    }
}

fn remark_or_default(v: &Option<String>) -> Option<String> {
    match v {
        Some(t) if !t.is_empty() => Some(t.clone()),
        _ => Some(String::new()),
    }
}

fn tags_or_default(v: &Option<String>) -> Option<String> {
    match v {
        Some(t) if !t.is_empty() => Some(t.clone()),
        _ => Some(String::new()),
    }
}

const UNKNOWN: &str = "未知";
const DONE: &str = "已完成";

pub async fn get_stats(pool: &SqlitePool) -> Result<Stats> {
    let w = DateWindow::build();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM records")
        .fetch_one(pool)
        .await?;

    let by_type: Vec<(String, i64)> =
        sqlx::query_as("SELECT COALESCE(`type`, ?) as t, COUNT(*) as c FROM records GROUP BY `type` COLLATE NOCASE ORDER BY c DESC, t")
            .bind(UNKNOWN)
            .fetch_all(pool)
            .await?;

    let by_status: Vec<(String, i64)> =
        sqlx::query_as("SELECT COALESCE(status, ?) as s, COUNT(*) as c FROM records GROUP BY status COLLATE NOCASE ORDER BY c DESC, s")
            .bind(UNKNOWN)
            .fetch_all(pool)
            .await?;

    let by_year: Vec<(i64, i64)> =
        sqlx::query_as("SELECT CAST(`year` AS INTEGER) as y, COUNT(*) FROM records WHERE `year` IS NOT NULL GROUP BY `year` ORDER BY y")
            .fetch_all(pool)
            .await?;

    let by_country: Vec<(String, i64)> =
        sqlx::query_as("SELECT country, COUNT(*) FROM records WHERE country IS NOT NULL AND country != '' GROUP BY country ORDER BY COUNT(*) DESC, country")
            .fetch_all(pool)
            .await?;

    let type_status: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT COALESCE(`type`, ?) as t, COALESCE(status, ?) as s, COUNT(*) as c FROM records GROUP BY `type` COLLATE NOCASE, status COLLATE NOCASE ORDER BY t, s")
            .bind(UNKNOWN)
            .bind(UNKNOWN)
            .fetch_all(pool)
            .await?;

    let progress_rows: Vec<(Option<i64>, Option<i64>)> =
        sqlx::query_as("SELECT current_episode, total_episode FROM records WHERE current_episode IS NOT NULL AND total_episode IS NOT NULL AND total_episode > 0")
            .fetch_all(pool)
            .await?;

    let tags_rows: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT tags FROM records WHERE tags IS NOT NULL AND tags != ''")
            .fetch_all(pool)
            .await?;

    let series_rows: Vec<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT tags, `type`, status FROM records WHERE tags IS NOT NULL AND tags != ''")
            .fetch_all(pool)
            .await?;

    let daily: Vec<(String, i64)> =
        sqlx::query_as("SELECT date(modify_time) as day, COUNT(*) as c FROM records WHERE modify_time IS NOT NULL GROUP BY date(modify_time) ORDER BY day")
            .fetch_all(pool)
            .await?;

    let monthly_end: Vec<(String, i64)> =
        sqlx::query_as("SELECT strftime('%Y-%m', end_time) as month, COUNT(*) as c FROM records WHERE end_time IS NOT NULL GROUP BY strftime('%Y-%m', end_time) ORDER BY month")
            .fetch_all(pool)
            .await?;

    let (new_today,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM records WHERE date(modify_time) = ?")
        .bind(&w.today)
        .fetch_one(pool)
        .await?;
    let (new_week,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM records WHERE modify_time >= ?")
            .bind(&w.week_ago_midnight)
            .fetch_one(pool)
            .await?;
    let (new_month,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM records WHERE modify_time >= ?")
            .bind(&w.month_ago_midnight)
            .fetch_one(pool)
            .await?;

    let (completed_today,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM records WHERE status = ? AND date(end_time) = ?")
            .bind(DONE)
            .bind(&w.today)
            .fetch_one(pool)
            .await?;
    let (completed_week,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM records WHERE status = ? AND end_time >= ?")
            .bind(DONE)
            .bind(&w.week_ago_midnight)
            .fetch_one(pool)
            .await?;
    let (completed_month,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM records WHERE status = ? AND end_time >= ?")
            .bind(DONE)
            .bind(&w.month_ago_midnight)
            .fetch_one(pool)
            .await?;

    // 进度分桶
    let mut buckets = vec![0i64; 5];
    for (cur, total) in &progress_rows {
        if let (Some(c), Some(t)) = (cur, total) {
            if *t > 0 {
                let pct = (*c as f64 / *t as f64) * 100.0;
                if pct >= 100.0 {
                    buckets[4] += 1;
                } else if pct >= 75.0 {
                    buckets[3] += 1;
                } else if pct >= 50.0 {
                    buckets[2] += 1;
                } else if pct >= 25.0 {
                    buckets[1] += 1;
                } else {
                    buckets[0] += 1;
                }
            }
        }
    }

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
    // 367 个标签里绝大多数 count 相同，若只按 count 排序，
// 并列项会保留 HashMap 的随机迭代顺序，导致每次启动标签顺序都在变。
// 统一加名称作为次级排序键，使输出稳定可预期。
by_tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));

    let tag_types: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT tags, COALESCE(`type`, ?) as t, COUNT(*) as c FROM records WHERE tags IS NOT NULL AND tags != '' GROUP BY tags, `type` COLLATE NOCASE ORDER BY tags")
            .bind(UNKNOWN)
            .fetch_all(pool)
            .await?;
    let tag_types: Vec<TagTypeCount> = tag_types
        .into_iter()
        .map(|(tag, media_type, count)| TagTypeCount { tag, media_type, count })
        .collect();

    struct SeriesEntry {
        total: i64,
        completed: i64,
        types: std::collections::HashMap<String, i64>,
    }
    let mut series_map: std::collections::HashMap<String, SeriesEntry> =
        std::collections::HashMap::new();
    fn is_series_tag(tag: &str) -> bool {
        tag.contains("系列")
            || tag.contains("宇宙")
            || tag.contains("传奇")
            || tag.contains("纪")
    }
    for (t, media_type, status) in &series_rows {
        if let Some(tags_str) = t {
            for tag in tags_str.split(',') {
                let trimmed = tag.trim();
                if !trimmed.is_empty() && is_series_tag(trimmed) {
                    let entry = series_map
                        .entry(trimmed.to_string())
                        .or_insert(SeriesEntry {
                            total: 0,
                            completed: 0,
                            types: std::collections::HashMap::new(),
                        });
                    entry.total += 1;
                    if status.as_deref() == Some(DONE) {
                        entry.completed += 1;
                    }
                    if let Some(mt) = media_type {
                        *entry.types.entry(mt.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    let mut series_stats: Vec<SeriesStat> = series_map
        .into_iter()
        .map(|(tag, entry)| {
            let mut by_type: Vec<TypeCount> = entry
                .types
                .into_iter()
                .map(|(media_type, count)| TypeCount { media_type, count })
                .collect();
            by_type.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.media_type.cmp(&b.media_type)));
            SeriesStat {
                tag,
                total: entry.total,
                completed: entry.completed,
                by_type,
            }
        })
        .collect();
    series_stats.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.tag.cmp(&b.tag)));

    let mut rate_map: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();
    for (t, s, c) in &type_status {
        let entry = rate_map.entry(t.clone()).or_insert((0, 0));
        entry.1 += c;
        if s == DONE {
            entry.0 += c;
        }
    }
    let mut completion_rates: Vec<CompletionRate> = rate_map
        .into_iter()
        .map(|(media_type, (completed, total))| CompletionRate {
            media_type,
            completed,
            total,
        })
        .collect();
    completion_rates.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.media_type.cmp(&b.media_type)));

    Ok(Stats {
        total: total.0,
        by_type: by_type
            .into_iter()
            .map(|(t, c)| TypeCount { media_type: t, count: c })
            .collect(),
        by_status: by_status
            .into_iter()
            .map(|(s, c)| StatusCount { status: s, count: c })
            .collect(),
        by_year: by_year
            .into_iter()
            .map(|(y, c)| YearCount { year: y as i32, count: c })
            .collect(),
        by_country: by_country
            .into_iter()
            .map(|(c, n)| CountryCount { country: c, count: n })
            .collect(),
        by_tags,
        tag_types,
        series_stats,
        progress_buckets: vec![
            ProgressBucket { label: "0-25%".into(), count: buckets[0] },
            ProgressBucket { label: "25-50%".into(), count: buckets[1] },
            ProgressBucket { label: "50-75%".into(), count: buckets[2] },
            ProgressBucket { label: "75-99%".into(), count: buckets[3] },
            ProgressBucket { label: "100%".into(), count: buckets[4] },
        ],
        type_status: type_status
            .into_iter()
            .map(|(t, s, c)| TypeStatusCount { media_type: t, status: s, count: c })
            .collect(),
        completion_rates,
        daily_activity: daily
            .into_iter()
            .map(|(d, c)| DailyActivity { date: d, count: c })
            .collect(),
        monthly_end: monthly_end
            .into_iter()
            .map(|(m, c)| MonthCount { month: m, count: c })
            .collect(),
        recent: RecentActivity {
            new_today,
            new_week,
            new_month,
            completed_today,
            completed_week,
            completed_month,
        },
    })
}

#[derive(sqlx::FromRow)]
struct RecordRow {
    pub id: i64,
    pub record_name: String,
    pub season: Option<i64>,
    pub remark: Option<String>,
    #[sqlx(rename = "type")]
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub end_time: Option<chrono::NaiveDateTime>,
    pub country: Option<String>,
    pub tags: Option<String>,
    pub current_episode: Option<i64>,
    pub total_episode: Option<i64>,
    pub year: Option<i64>,
    pub modify_time: Option<chrono::NaiveDateTime>,
}

impl From<RecordRow> for Record {
    fn from(r: RecordRow) -> Self {
        Self {
            id: r.id,
            record_name: r.record_name,
            season: r.season.map(|v| v as i32),
            remark: r.remark,
            media_type: r.media_type,
            status: r.status,
            end_time: r.end_time.map(|t| t.format(TS_FORMAT).to_string()),
            country: r.country,
            tags: r.tags,
            current_episode: r.current_episode.map(|v| v as i32),
            total_episode: r.total_episode.map(|v| v as i32),
            year: r.year.map(|y| y as i32),
            modify_time: r.modify_time.map(|t| t.format(TS_FORMAT).to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    async fn temp_pool(tag: &str) -> (SqlitePool, PathBuf) {
        let dir = std::env::temp_dir().join(format!("process_db_test_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = AppConfig {
            data_dir: dir.clone(),
            snapshot_dir: None,
            keep_snapshots: 10,
        };
        let pool = connect(&cfg).await.expect("connect");
        (pool, dir)
    }

    fn new_record(name: &str) -> NewRecord {
        NewRecord {
            record_name: name.into(),
            season: Some(1),
            remark: Some("备注".into()),
            media_type: Some("电影".into()),
            status: Some("进行中".into()),
            end_time: None,
            country: Some("美国".into()),
            tags: Some("系列, 经典".into()),
            current_episode: Some(3),
            total_episode: Some(12),
            year: Some(2021),
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let (pool, dir) = temp_pool("crud").await;

        let id = add_record(&pool, new_record("星际穿越")).await.unwrap();
        assert!(id > 0, "应返回自增 id");

        let got = get_record(&pool, id).await.unwrap().expect("应能读回");
        assert_eq!(got.record_name, "星际穿越");
        assert_eq!(got.season, Some(1));
        assert_eq!(got.current_episode, Some(3));
        assert_eq!(got.total_episode, Some(12));
        assert_eq!(got.year, Some(2021));
        assert!(
            got.modify_time
                .as_deref()
                .map(|t| t.len() == 19)
                .unwrap_or(false),
            "modify_time 应为 YYYY-MM-DD HH:MM:SS，实际 {:?}",
            got.modify_time
        );

        // 空字符串应被规范化为 None 而非写入空串
        let mut upd = UpdateRecord {
            id,
            record_name: "星际穿越 (重制)".into(),
            season: Some(2),
            remark: Some(String::new()),
            media_type: Some("电影".into()),
            status: Some("已完成".into()),
            end_time: Some("2026-10-04 08:00:00".into()),
            country: Some("美国".into()),
            tags: Some(String::new()),
            current_episode: Some(12),
            total_episode: Some(12),
            year: Some(2021),
        };
        assert!(update_record(&pool, upd.clone()).await.unwrap());

        let got = get_record(&pool, id).await.unwrap().unwrap();
        assert_eq!(got.record_name, "星际穿越 (重制)");
        assert_eq!(got.status, Some("已完成".into()));
        assert_eq!(got.end_time.as_deref(), Some("2026-10-04 08:00:00"));
        assert_eq!(got.tags.as_deref(), Some(""), "空 tags 应存空串而非 NULL");

        // 显式置空 end_time 应变NULL
        upd.end_time = Some(String::new());
        update_record(&pool, upd).await.unwrap();
        let got = get_record(&pool, id).await.unwrap().unwrap();
        assert!(got.end_time.is_none(), "空字符串时间应转成 NULL");

        assert!(delete_record(&pool, id).await.unwrap());
        assert!(get_record(&pool, id).await.unwrap().is_none());
        assert!(!delete_record(&pool, id).await.unwrap(), "重复删除应返回 false");

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn search_is_case_insensitive_like_mysql() {
        // MySQL 的 utf8mb4_0900_ai_ci 不区分大小写，SQLite 侧必须对齐
        let (pool, dir) = temp_pool("case").await;
        add_record(&pool, new_record("Naruto Shippuden")).await.unwrap();
        add_record(&pool, new_record("BERSERK")).await.unwrap();

        for needle in ["naruto", "NARUTO", "NaRuTo"] {
            let r = list_records(
                &pool,
                RecordFilter {
                    search: Some(needle.into()),
                    page: Some(1),
                    page_size: Some(50),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(r.total, 1, "搜索 {needle} 应命中 1 条（大小写不敏感）");
        }

        let r = list_records(
            &pool,
            RecordFilter {
                search: Some("berserk".into()),
                page: Some(1),
                page_size: Some(50),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(r.total, 1, "小写搜索应命中 BERSERK");

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn filter_by_type_and_status() {
        let (pool, dir) = temp_pool("filter").await;
        let mut a = new_record("A");
        a.media_type = Some("电影".into());
        a.status = Some("已完成".into());
        add_record(&pool, a).await.unwrap();

        let mut b = new_record("B");
        b.media_type = Some("动漫".into());
        b.status = Some("进行中".into());
        add_record(&pool, b).await.unwrap();

        let f = |t: Option<&str>, s: Option<&str>| RecordFilter {
            media_type: t.map(str::to_string),
            status: s.map(str::to_string),
            page: Some(1),
            page_size: Some(50),
            ..Default::default()
        };

        assert_eq!(
            list_records(&pool, f(Some("电影"), None)).await.unwrap().total,
            1
        );
        assert_eq!(
            list_records(&pool, f(None, Some("进行中"))).await.unwrap().total,
            1
        );
        assert_eq!(
            list_records(&pool, f(Some("电影"), Some("已完成")))
                .await
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            list_records(&pool, f(Some("书籍"), None)).await.unwrap().total,
            0
        );
        // 「全部」应等价于不过滤
        assert_eq!(
            list_records(&pool, f(Some("全部"), Some("全部")))
                .await
                .unwrap()
                .total,
            2
        );

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ordering_and_pagination() {
        let (pool, dir) = temp_pool("page").await;
        // end_time 越新越靠前；null 排最后（COALESCE '1970-01-01'）
        for (i, end) in [(1, None), (2, Some("2020-01-01 00:00:00")), (3, Some("2026-01-01 00:00:00"))] {
            let mut r = new_record(&format!("R{i}"));
            r.end_time = end.map(str::to_string);
            add_record(&pool, r).await.unwrap();
        }

        let all = list_records(
            &pool,
            RecordFilter { page: Some(1), page_size: Some(50), ..Default::default() },
        )
        .await
        .unwrap();
        let names: Vec<&str> = all.records.iter().map(|r| r.record_name.as_str()).collect();
        assert_eq!(names, vec!["R3", "R2", "R1"], "应按 end_time 倒序，NULL 在最后");

        let p1 = list_records(
            &pool,
            RecordFilter { page: Some(1), page_size: Some(2), ..Default::default() },
        )
        .await
        .unwrap();
        let p2 = list_records(
            &pool,
            RecordFilter { page: Some(2), page_size: Some(2), ..Default::default() },
        )
        .await
        .unwrap();
        assert_eq!(p1.total, 3, "total 应为总条数而非当前页");
        assert_eq!(p1.records.len(), 2);
        assert_eq!(p2.records.len(), 1);

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn progress_buckets_match_mysql_logic() {
        let (pool, dir) = temp_pool("bucket").await;
        // 0 / 20 / 40 / 60 / 80 / 100 各一条
        for (cur, total) in [(0, 100), (20, 100), (40, 100), (60, 100), (80, 100), (100, 100)] {
            let mut r = new_record(&format!("{cur}"));
            r.current_episode = Some(cur);
            r.total_episode = Some(total);
            add_record(&pool, r).await.unwrap();
        }

        let s = get_stats(&pool).await.unwrap();
        let buckets = &s.progress_buckets;
        assert_eq!(buckets[0].count, 2, "0-25%: 0 与 20");
        assert_eq!(buckets[1].count, 1, "25-50%: 40");
        assert_eq!(buckets[2].count, 1, "50-75%: 60");
        assert_eq!(buckets[3].count, 1, "75-99%: 80");
        assert_eq!(buckets[4].count, 1, "100%");
        assert_eq!(
            buckets.iter().map(|b| b.count).sum::<i64>(),
            6,
            "分桶总数应等于有进度数据的记录数"
        );

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn null_type_becomes_unknown_bucket() {
        let (pool, dir) = temp_pool("unknown").await;
        let mut r = new_record("无类型");
        r.media_type = None;
        r.status = None;
        add_record(&pool, r).await.unwrap();

        let s = get_stats(&pool).await.unwrap();
        assert_eq!(s.by_type[0].media_type, "未知");
        assert_eq!(s.by_type[0].count, 1);
        assert_eq!(s.by_status[0].status, "未知");

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn tags_are_split_and_counted() {
        let (pool, dir) = temp_pool("tags").await;
        let mut a = new_record("A");
        a.tags = Some("系列, 经典 ,宇宙".into());
        a.status = Some("已完成".into());
        add_record(&pool, a).await.unwrap();
        let mut b = new_record("B");
        b.tags = Some("系列".into());
        add_record(&pool, b).await.unwrap();

        let s = get_stats(&pool).await.unwrap();
        let series = s.by_tags.iter().find(|t| t.tag == "系列").expect("应拆出「系列」");
        assert_eq!(series.count, 2, "系列应出现 2 次");
        // 「经典」前后有空格，应被 trim
        assert_eq!(s.by_tags.iter().find(|t| t.tag == "经典").unwrap().count, 1);

        let st = s.series_stats.iter().find(|x| x.tag == "系列").unwrap();
        assert_eq!(st.total, 2);
        assert_eq!(st.completed, 1);

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn replace_all_records_preserves_id_sequence() {
        let (pool, dir) = temp_pool("replace").await;

        let mut records: Vec<Record> = (1..=5)
            .map(|i| Record {
                id: i * 10,
                record_name: format!("R{i}"),
                season: None,
                remark: None,
                media_type: Some("电影".into()),
                status: Some("已完成".into()),
                end_time: None,
                country: None,
                tags: None,
                current_episode: None,
                total_episode: None,
                year: Some(2020),
                modify_time: Some("2026-10-04 00:00:00".into()),
            })
            .collect();
        records[0].record_name = "覆盖后".into();

        assert_eq!(replace_all_records(&pool, &records, true).await.unwrap(), 5);
        assert_eq!(count_records(&pool).await.unwrap(), 5);
        assert_eq!(get_record(&pool, 10).await.unwrap().unwrap().record_name, "覆盖后");

        // 序列应从 50 继续，而不是回到 1
        let new_id = add_record(&pool, new_record("新记录")).await.unwrap();
        assert!(new_id > 50, "新 id 应大于原有最大值 50，实际 {new_id}");

        // 再次替换应清空旧数据
        replace_all_records(&pool, &records[..2], true).await.unwrap();
        assert_eq!(count_records(&pool).await.unwrap(), 2);

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn connect_is_idempotent() {
        let dir = std::env::temp_dir().join("process_db_test_idem");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = AppConfig {
            data_dir: dir.clone(),
            snapshot_dir: None,
            keep_snapshots: 10,
        };

        for _ in 0..3 {
            let pool = connect(&cfg).await.expect("重复 connect 应成功");
            drop(pool);
        }
        assert!(cfg.database_path().exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn date_window_uses_midnight_boundaries() {
        // MySQL: modify_time >= DATE_SUB(CURDATE(), INTERVAL 7 DAY)
        // 中 CURDATE() 是日期，与 datetime 比较时按 00:00:00 处理
        let w = DateWindow::from_date(NaiveDate::from_ymd_opt(2026, 10, 4).unwrap());
        assert_eq!(w.today, "2026-10-04");
        assert_eq!(w.week_ago_midnight, "2026-09-27 00:00:00");
        assert_eq!(w.month_ago_midnight, "2026-09-04 00:00:00");
    }

    #[test]
    fn date_window_crosses_month_boundary() {
        let w = DateWindow::from_date(NaiveDate::from_ymd_opt(2026, 3, 5).unwrap());
        assert_eq!(w.week_ago_midnight, "2026-02-26 00:00:00");
        assert_eq!(w.month_ago_midnight, "2026-02-03 00:00:00");
    }

    #[test]
    fn date_window_handles_leap_year() {
        let w = DateWindow::from_date(NaiveDate::from_ymd_opt(2028, 3, 1).unwrap());
        assert_eq!(w.week_ago_midnight, "2028-02-23 00:00:00");
    }

    /// 方言等价性验证：导入真实 MySQL 导出的记录，跑 SQLite 版get_stats，
    /// 结果写到 STATS_OUT 供与 MySQL 基线逐字段 diff。
    /// 默认跳过。手动运行：
    ///   RECORDS_JSON=... STATS_BASELINE=... STATS_OUT=... cargo test --lib -- --ignored sqlite_baseline --nocapture
    #[tokio::test]
    #[ignore = "需要 RECORDS_JSON / STATS_OUT 环境变量与真实数据"]
    async fn sqlite_baseline() {
        let records_json =
            std::env::var("RECORDS_JSON").expect("缺少 RECORDS_JSON");
        let out = std::env::var("STATS_OUT").expect("缺少 STATS_OUT");

        let dir = std::env::temp_dir().join("process_baseline_sqlite");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = AppConfig {
            data_dir: dir.clone(),
            snapshot_dir: None,
            keep_snapshots: 10,
        };
        let pool = connect(&cfg).await.expect("connect");

        let n = crate::snapshot::import_records_file(&pool, &records_json)
            .await
            .expect("导入 MySQL 导出数据失败");
        println!("imported {n} records");

        let stats = get_stats(&pool).await.expect("get_stats");
        std::fs::write(&out, serde_json::to_vec_pretty(&stats).unwrap()).unwrap();

        println!("wrote {out}");
        println!(
            "sanity: total={} by_type={} by_status={} by_year={} daily={} monthly={}",
            stats.total,
            stats.by_type.len(),
            stats.by_status.len(),
            stats.by_year.len(),
            stats.daily_activity.len(),
            stats.monthly_end.len()
        );

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 一次性迁移：把 MySQL 导出的 JSON 导入真实配置指向的数据库。
    /// 默认跳过。手动运行：
    ///   RECORDS_JSON=... cargo test --lib -- --ignored import_into_default_storage --nocapture
    #[tokio::test]
    #[ignore = "会写入真实用户数据库，需显式触发"]
    async fn import_into_default_storage() {
        let records_json = std::env::var("RECORDS_JSON").expect("缺少 RECORDS_JSON");
        let cfg = crate::config::load_app_config();
        println!("目标数据库: {}", cfg.database_path().display());

        let pool = connect(&cfg).await.expect("connect");
        let before = count_records(&pool).await.unwrap_or(0);
        println!("导入前已有 {before} 条");

        let n = crate::snapshot::import_records_file(&pool, &records_json)
            .await
            .expect("导入失败");
        println!("已导入 {n} 条");

        let after = count_records(&pool).await.unwrap_or(0);
        println!("导入后共 {after} 条");
        assert_eq!(after as usize, n, "导入后总数应等于导入条数");

        let stats = get_stats(&pool).await.expect("get_stats");
        println!(
            "校验: total={} by_type={} by_status={} by_year={} by_tags={} daily={} monthly={}",
            stats.total,
            stats.by_type.len(),
            stats.by_status.len(),
            stats.by_year.len(),
            stats.by_tags.len(),
            stats.daily_activity.len(),
            stats.monthly_end.len()
        );

        drop(pool);
    }
}

impl Default for RecordFilter {
    fn default() -> Self {
        Self {
            search: None,
            media_type: None,
            status: None,
            tag: None,
            end_time_start: None,
            end_time_end: None,
            page: None,
            page_size: None,
        }
    }
}