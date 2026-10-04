use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
    pub id: i64,
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
    /// 序列化名沿用前端契约的 media_type；
    /// alias 让导入侧也接受 MySQL 导出的 "type" 键，避免整列静默丢失。
    #[serde(alias = "type")]
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub end_time: Option<String>,
    pub country: Option<String>,
    pub tags: Option<String>,
    pub current_episode: Option<i32>,
    pub total_episode: Option<i32>,
    pub year: Option<i32>,
    pub modify_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewRecord {
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
    #[serde(alias = "type")]
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub end_time: Option<String>,
    pub country: Option<String>,
    pub tags: Option<String>,
    pub current_episode: Option<i32>,
    pub total_episode: Option<i32>,
    pub year: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateRecord {
    pub id: i64,
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
    #[serde(alias = "type")]
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub end_time: Option<String>,
    pub country: Option<String>,
    pub tags: Option<String>,
    pub current_episode: Option<i32>,
    pub total_episode: Option<i32>,
    pub year: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stats {
    pub total: i64,
    pub by_type: Vec<TypeCount>,
    pub by_status: Vec<StatusCount>,
    pub by_year: Vec<YearCount>,
    pub by_country: Vec<CountryCount>,
    pub by_tags: Vec<TagCount>,
    pub tag_types: Vec<TagTypeCount>,
    pub series_stats: Vec<SeriesStat>,
    pub progress_buckets: Vec<ProgressBucket>,
    pub type_status: Vec<TypeStatusCount>,
    pub completion_rates: Vec<CompletionRate>,
    pub daily_activity: Vec<DailyActivity>,
    pub monthly_end: Vec<MonthCount>,
    pub recent: RecentActivity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeriesStat {
    pub tag: String,
    pub total: i64,
    pub completed: i64,
    pub by_type: Vec<TypeCount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeCount {
    pub media_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YearCount {
    pub year: i32,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountryCount {
    pub country: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagTypeCount {
    pub tag: String,
    pub media_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeStatusCount {
    pub media_type: String,
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRate {
    pub media_type: String,
    pub completed: i64,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyActivity {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonthCount {
    pub month: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentActivity {
    pub new_today: i64,
    pub new_week: i64,
    pub new_month: i64,
    pub completed_today: i64,
    pub completed_week: i64,
    pub completed_month: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordFilter {
    pub search: Option<String>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub tag: Option<String>,
    pub end_time_start: Option<String>,
    pub end_time_end: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult {
    pub records: Vec<Record>,
    pub total: i64,
}

/// 扫描到的一个本地视频文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub path: String,
    /// 显示用名称：去扩展名 + 下划线转空格
    pub name: String,
    pub size: u64,
    /// Unix 秒
    pub mtime: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub videos: Vec<VideoFile>,
    /// 已配置但无法访问的目录
    pub missing: Vec<String>,
    /// 因权限或 I/O 错误跳过的文件数
    pub skipped: usize,
}
