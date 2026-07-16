use std::sync::{Arc, Mutex};
use sqlx::MySqlPool;
use tauri::State;

use crate::config::{self, DbConfig};
use crate::db;
use crate::models::*;

pub struct AppState {
    pub pool: Arc<Mutex<Option<MySqlPool>>>,
}

#[tauri::command]
pub async fn test_connection(config: DbConfig) -> Result<String, String> {
    match db::connect(&config).await {
        Ok(_) => Ok("连接成功".into()),
        Err(e) => Err(format!("连接失败: {}", e)),
    }
}

#[tauri::command]
pub async fn get_config() -> Result<Option<DbConfig>, String> {
    Ok(config::load_config())
}

#[tauri::command]
pub async fn save_config(config_data: DbConfig) -> Result<String, String> {
    config::save_config(&config_data).map_err(|e| format!("保存配置失败: {}", e))?;
    Ok("配置已保存".into())
}

#[tauri::command]
pub async fn init_db(state: State<'_, AppState>, config: DbConfig) -> Result<String, String> {
    // Background task may have already connected
    if state.pool.lock().map_err(|e| e.to_string())?.is_some() {
        config::save_config(&config).ok();
        return Ok("数据库连接成功".into());
    }
    match db::connect(&config).await {
        Ok(pool) => {
            *state.pool.lock().map_err(|e| e.to_string())? = Some(pool);
            config::save_config(&config).ok();
            Ok("数据库连接成功".into())
        }
        Err(e) => Err(format!("连接失败: {}", e)),
    }
}

fn get_pool(state: &AppState) -> Result<MySqlPool, String> {
    state
        .pool
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "数据库未连接".into())
}

#[tauri::command]
pub async fn list_records(
    state: State<'_, AppState>,
    filter: RecordFilter,
) -> Result<PaginatedResult, String> {
    let pool = get_pool(&state)?;
    db::list_records(&pool, filter)
        .await
        .map_err(|e| format!("查询失败: {}", e))
}

#[tauri::command]
pub async fn get_record(state: State<'_, AppState>, id: i64) -> Result<Option<Record>, String> {
    let pool = get_pool(&state)?;
    db::get_record(&pool, id)
        .await
        .map_err(|e| format!("查询失败: {}", e))
}

#[tauri::command]
pub async fn add_record(
    state: State<'_, AppState>,
    record: NewRecord,
) -> Result<i64, String> {
    let pool = get_pool(&state)?;
    db::add_record(&pool, record)
        .await
        .map_err(|e| format!("添加失败: {}", e))
}

#[tauri::command]
pub async fn update_record(
    state: State<'_, AppState>,
    record: UpdateRecord,
) -> Result<bool, String> {
    let pool = get_pool(&state)?;
    db::update_record(&pool, record)
        .await
        .map_err(|e| format!("更新失败: {}", e))
}

#[tauri::command]
pub async fn delete_record(state: State<'_, AppState>, id: i64) -> Result<bool, String> {
    let pool = get_pool(&state)?;
    db::delete_record(&pool, id)
        .await
        .map_err(|e| format!("删除失败: {}", e))
}

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> Result<Stats, String> {
    let pool = get_pool(&state)?;
    db::get_stats(&pool)
        .await
        .map_err(|e| format!("统计失败: {}", e))
}
