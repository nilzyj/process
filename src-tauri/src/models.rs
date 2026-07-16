use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
    pub id: i64,
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
    #[serde(rename = "type")]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct NewRecord {
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
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
pub struct UpdateRecord {
    pub id: i64,
    pub record_name: String,
    pub season: Option<i32>,
    pub remark: Option<String>,
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
pub struct RecordFilter {
    pub search: Option<String>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResult {
    pub records: Vec<Record>,
    pub total: i64,
}
