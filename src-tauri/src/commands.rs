use std::sync::{Arc, Mutex, RwLock};
use sqlx::MySqlPool;
use tauri::State;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::{self, DbConfig, LibraryConfig};
use crate::db;
use crate::models::*;
use crate::{player, scan};

pub struct AppState {
    pub pool: Arc<Mutex<Option<MySqlPool>>>,
    pub cached_records: Arc<Mutex<Option<PaginatedResult>>>,
    /// 当前已扫描到的视频文件，play_video / reveal_in_explorer 以此为白名单
    pub videos: Arc<RwLock<Vec<VideoFile>>>,
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

#[tauri::command]
pub async fn get_cached_records(state: State<'_, AppState>) -> Result<Option<PaginatedResult>, String> {
    Ok(state
        .cached_records
        .lock()
        .map_err(|e| e.to_string())?
        .clone())
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

// ---------- 媒体库 ----------

#[tauri::command]
pub async fn get_library_config() -> Result<LibraryConfig, String> {
    Ok(config::load_library_config())
}

#[tauri::command]
pub async fn save_library_config(cfg: LibraryConfig) -> Result<String, String> {
    config::save_library_config(&cfg).map_err(|e| format!("保存失败: {}", e))?;
    Ok("配置已保存".into())
}

/// 重新扫描所有已配置目录，并刷新内存白名单
async fn rescan_and_store(state: &AppState) -> Result<ScanResult, String> {
    let cfg = config::load_library_config();
    let result = tauri::async_runtime::spawn_blocking(move || scan::scan_folders(&cfg.folders))
        .await
        .map_err(|e| format!("扫描任务失败: {}", e))?;

    *state
        .videos
        .write()
        .map_err(|_| "视频列表锁已中毒".to_string())? = result.videos.clone();

    Ok(result)
}

#[tauri::command]
pub async fn rescan_library(state: State<'_, AppState>) -> Result<ScanResult, String> {
    rescan_and_store(&state).await
}

#[tauri::command]
pub async fn add_library_folder(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ScanResult, String> {
    let mut cfg = config::load_library_config();
    let mut added = 0usize;

    for p in paths {
        let p = p.trim().to_string();
        if p.is_empty() {
            continue;
        }
        // Rust 侧校验，前端无需判断（拖放场景尤其重要）
        if !std::path::Path::new(&p).is_dir() {
            continue;
        }
        if !cfg.folders.iter().any(|f| f.eq_ignore_ascii_case(&p)) {
            cfg.folders.push(p);
            added += 1;
        }
    }

    if added == 0 {
        // 全部非法或重复时也要刷新一次，让 UI 拿到真实状态
        return rescan_and_store(&state).await;
    }

    config::save_library_config(&cfg).map_err(|e| format!("保存失败: {}", e))?;
    rescan_and_store(&state).await
}

#[tauri::command]
pub async fn remove_library_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<ScanResult, String> {
    let mut cfg = config::load_library_config();
    cfg.folders.retain(|f| !f.eq_ignore_ascii_case(&path));
    config::save_library_config(&cfg).map_err(|e| format!("保存失败: {}", e))?;
    rescan_and_store(&state).await
}

/// 校验 path 确实来自扫描结果。
/// 前端内容可被注入，不校验就等于开放任意程序执行。
fn is_known_video(state: &AppState, path: &str) -> Result<(), String> {
    let videos = state
        .videos
        .read()
        .map_err(|_| "视频列表锁已中毒".to_string())?;
    if videos.iter().any(|v| v.path == path) {
        Ok(())
    } else {
        Err("文件不在媒体库中".into())
    }
}

#[tauri::command]
pub async fn play_video(state: State<'_, AppState>, path: String) -> Result<String, String> {
    is_known_video(&state, &path)?;
    let file = std::path::Path::new(&path);
    if !file.is_file() {
        return Err("文件不存在或已被移动".into());
    }

    let configured = config::load_library_config().potplayer_path;
    let exe = player::locate(configured.as_deref())
        .ok_or_else(|| "未找到 PotPlayer，请手动指定其安装路径".to_string())?;

    player::launch(&exe, file).map_err(|e| format!("启动 PotPlayer 失败: {}", e))?;
    Ok("已启动".into())
}

#[tauri::command]
pub async fn reveal_in_explorer(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    is_known_video(&state, &path)?;
    let file = std::path::Path::new(&path);
    if !file.is_file() {
        return Err("文件不存在".into());
    }

    let parent = file.parent().ok_or("无效路径")?;

    #[cfg(windows)]
    {
        let _ = &parent;
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path))
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("打开资源管理器失败: {}", e))?;
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    Ok("已定位".into())
}

#[tauri::command]
pub async fn locate_potplayer() -> Result<Option<String>, String> {
    let configured = config::load_library_config().potplayer_path;
    Ok(player::locate(configured.as_deref()))
}
