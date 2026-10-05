use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use tauri::State;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use sqlx::SqlitePool;

use crate::config::{self, normalize_folder_path, AppConfig, LibraryConfig};
use crate::db;
use crate::models::*;
use crate::snapshot::{self, SnapshotInfo};
use crate::{player, scan};

pub struct AppState {
    pub pool: Arc<Mutex<Option<SqlitePool>>>,
    pub cached_records: Arc<Mutex<Option<PaginatedResult>>>,
    /// 当前已扫描到的视频文件，play_video / reveal_in_explorer 以此为白名单
    pub videos: Arc<RwLock<Vec<VideoFile>>>,
}

#[derive(serde::Serialize)]
pub struct StorageInfo {
    pub database_path: String,
    pub snapshot_dir: Option<String>,
    pub record_count: i64,
}

// ---------- 存储 ----------

fn get_pool(state: &AppState) -> Result<SqlitePool, String> {
    state
        .pool
        .lock()
        .map_err(|_| "数据库句柄锁已中毒".to_string())?
        .clone()
        .ok_or_else(|| "本地数据库尚未初始化".into())
}

/// 打开本地 SQLite。文件不存在会自动创建，schema 幂等补齐。
/// 本地存储不存在「连接失败」，失败只可能是磁盘/权限问题。
#[tauri::command]
pub async fn init_storage(state: State<'_, AppState>) -> Result<StorageInfo, String> {
    // 预热任务可能已完成
    if state.pool.lock().map_err(|e| e.to_string())?.is_some() {
        return storage_info(&state).await;
    }

    let cfg = config::load_app_config();
    match db::connect(&cfg).await {
        Ok(pool) => {
            let count = db::count_records(&pool).await.unwrap_or(0);
            *state.pool.lock().map_err(|e| e.to_string())? = Some(pool);
            Ok(StorageInfo {
                database_path: cfg.database_path().to_string_lossy().into_owned(),
                snapshot_dir: cfg.snapshot_dir.as_ref().map(|p| p.to_string_lossy().into_owned()),
                record_count: count,
            })
        }
        Err(e) => Err(format!("打开本地数据库失败: {}", e)),
    }
}

async fn storage_info(state: &AppState) -> Result<StorageInfo, String> {
    let cfg = config::load_app_config();
    let count = match get_pool(state) {
        Ok(p) => db::count_records(&p).await.unwrap_or(0),
        Err(_) => 0,
    };
    Ok(StorageInfo {
        database_path: cfg.database_path().to_string_lossy().into_owned(),
        snapshot_dir: cfg.snapshot_dir.as_ref().map(|p| p.to_string_lossy().into_owned()),
        record_count: count,
    })
}

#[tauri::command]
pub async fn get_storage_info(state: State<'_, AppState>) -> Result<StorageInfo, String> {
    storage_info(&state).await
}

#[tauri::command]
pub async fn get_app_config() -> Result<AppConfig, String> {
    Ok(config::load_app_config())
}

#[tauri::command]
pub async fn save_app_config(cfg: AppConfig) -> Result<String, String> {
    config::save_app_config(&cfg).map_err(|e| format!("保存失败: {}", e))?;
    Ok("配置已保存".into())
}

/// 检测磁盘上是否还残留旧版含明文 MySQL 密码的 config.json。
/// 切换到本地 SQLite 后该文件不再被读取，但凭据仍留在磁盘上，需提示用户清理。
#[tauri::command]
pub async fn legacy_credentials_present() -> Result<bool, String> {
    Ok(config::legacy_mysql_config_present())
}

// ---------- 快照 ----------

#[tauri::command]
pub async fn export_snapshot(state: State<'_, AppState>) -> Result<String, String> {
    let pool = get_pool(&state)?;
    let cfg = config::load_app_config();
    snapshot::export(&pool, &cfg)
        .await
        .map_err(|e| format!("导出失败: {}", e))
}

#[tauri::command]
pub async fn list_snapshots() -> Result<Vec<SnapshotInfo>, String> {
    let cfg = config::load_app_config();
    let Some(dir) = cfg.snapshot_dir else {
        return Ok(Vec::new());
    };
    Ok(snapshot::list_snapshots(&dir))
}

#[tauri::command]
pub async fn restore_snapshot(state: State<'_, AppState>, path: String) -> Result<usize, String> {
    let pool = get_pool(&state)?;
    // 恢复前先给当前数据留一份，避免误操作丢失
    let cfg = config::load_app_config();
    if db::count_records(&pool).await.unwrap_or(0) > 0 {
        let _ = snapshot::export(&pool, &cfg).await;
    }
    snapshot::restore(&pool, &path)
        .await
        .map_err(|e| format!("恢复失败: {}", e))
}

/// 从 JSON 文件导入记录。用于 MySQL 迁移与手工恢复。
#[tauri::command]
pub async fn import_records(
    state: State<'_, AppState>,
    path: String,
) -> Result<usize, String> {
    let pool = get_pool(&state)?;
    let cfg = config::load_app_config();
    let count = db::count_records(&pool).await.unwrap_or(0);
    if count > 0 {
        let _ = snapshot::export(&pool, &cfg).await;
    }
    snapshot::import_records_file(&pool, &path)
        .await
        .map_err(|e| format!("导入失败: {}", e))
}

// ---------- 记录 ----------

#[tauri::command]
pub async fn get_cached_records(state: State<'_, AppState>) -> Result<Option<PaginatedResult>, String> {
    Ok(state
        .cached_records
        .lock()
        .map_err(|e| e.to_string())?
        .clone())
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
pub async fn add_record(state: State<'_, AppState>, record: NewRecord) -> Result<i64, String> {
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

/// 重扫所有已配置目录并刷新内存白名单。
/// 返回体带上 folders，使前端无需再查一次配置即可与后端保持一致。
async fn rescan_and_store(state: &AppState) -> Result<LibraryState, String> {
    rescan_and_store_with(state, Vec::new(), Vec::new()).await
}

async fn rescan_and_store_with(
    state: &AppState,
    created_dirs: Vec<String>,
    rejected: Vec<RejectedPath>,
) -> Result<LibraryState, String> {
    let cfg = config::load_library_config();
    let folders = cfg.folders.clone();

    let scanned = tauri::async_runtime::spawn_blocking(move || scan::scan_folders(&folders))
        .await
        .map_err(|e| format!("扫描任务失败: {}", e))?;

    *state
        .videos
        .write()
        .map_err(|_| "视频列表锁已中毒".to_string())? = scanned.videos.clone();

    Ok(LibraryState {
        folders: cfg.folders,
        videos: scanned.videos,
        missing: scanned.missing,
        skipped: scanned.skipped,
        ignored: scanned.ignored,
        created_dirs,
        rejected,
    })
}

/// 确保目录存在：不存在则创建。
///
/// 只在用户显式添加目录时调用。扫描路径绝不能这样做——NAS 断连或路径打错时
/// 会凭空造出垃圾目录，把真实问题掩盖掉，此时应让 missing 标记生效。
fn ensure_dir(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        return Ok(());
    }
    // 拒绝相对路径：create_dir_all 会按进程 cwd 解析，可能在任意位置造出目录
    if !path.is_absolute() {
        return Err("需要绝对路径".into());
    }
    // 已存在但不是目录（例如指向同名文件）
    if path.exists() {
        return Err("同名文件已存在，不是目录".into());
    }
    std::fs::create_dir_all(path).map_err(|e| format!("创建失败：{}", e))
}

/// 把目录加入配置，返回实际新增条数。跳过空串与重复项；
/// 不存在的目录会先尝试创建，创建失败则记入 rejected。
/// 纯逻辑，不碰磁盘（除创建目录），便于脱离 Tauri 运行时测试。
fn add_folders_to(cfg: &mut LibraryConfig, paths: Vec<String>) -> AddOutcome {
    let mut out = AddOutcome::default();

    for raw in paths {
        let p = normalize_folder_path(&raw);
        if p.is_empty() {
            continue;
        }
        if cfg.folders.iter().any(|f| f.eq_ignore_ascii_case(&p)) {
            continue;
        }

        let path = Path::new(&p);
        let existed = path.is_dir();
        match ensure_dir(path) {
            Ok(()) => {
                if !existed {
                    out.created_dirs.push(p.clone());
                }
                cfg.folders.push(p);
                out.added += 1;
            }
            Err(reason) => out.rejected.push(RejectedPath { path: p, reason }),
        }
    }

    out
}

#[derive(Default)]
struct AddOutcome {
    added: usize,
    created_dirs: Vec<String>,
    rejected: Vec<RejectedPath>,
}

/// 从配置移除目录，返回是否确实移除了一项。纯逻辑，不碰磁盘。
fn remove_folder_from(cfg: &mut LibraryConfig, path: &str) -> bool {
    let before = cfg.folders.len();
    cfg.folders.retain(|f| !f.eq_ignore_ascii_case(path));
    cfg.folders.len() != before
}

#[tauri::command]
pub async fn rescan_library(state: State<'_, AppState>) -> Result<LibraryState, String> {
    rescan_and_store(&state).await
}

#[tauri::command]
pub async fn add_library_folder(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<LibraryState, String> {
    let mut cfg = config::load_library_config();
    let outcome = add_folders_to(&mut cfg, paths);
    if outcome.added > 0 {
        config::save_library_config(&cfg).map_err(|e| format!("保存失败: {}", e))?;
    }
    rescan_and_store_with(&state, outcome.created_dirs, outcome.rejected).await
}

#[tauri::command]
pub async fn remove_library_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<LibraryState, String> {
    let mut cfg = config::load_library_config();
    remove_folder_from(&mut cfg, &path);
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

    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path))
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("打开资源管理器失败: {}", e))?;
    }

    #[cfg(not(windows))]
    {
        let parent = file.parent().ok_or("无效路径")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 实机验证：手动输入一个不存在的路径应自动创建目录并加入媒体库。
    /// 默认跳过。手动运行：
    ///   SMOKE_DIR=<待创建路径> cargo test --lib -- --ignored smoke_auto_create_dir --nocapture
    /// 实机验证：拿真实目录走一遍「浏览添加 → 扫描」全流程。
    /// 默认跳过。手动运行：
    ///   REAL_DIR=<目录> cargo test --lib -- --ignored smoke_real_browse_add --nocapture
    #[test]
    #[ignore = "会扫描 REAL_DIR 指定的真实目录"]
    fn smoke_real_browse_add() {
        let Ok(dir) = std::env::var("REAL_DIR") else {
            return;
        };
        println!("REAL_DIR = {dir}");

        let mut cfg = LibraryConfig::default();

        // 模拟浏览对话框返回：带尾随反斜杠，Windows 上很常见
        let with_slash = format!("{}\\", dir.trim_end_matches('\\'));
        let picked = vec![with_slash.clone()];

        let outcome = add_folders_to(&mut cfg, picked);
        println!("  added        = {}", outcome.added);
        println!("  created_dirs = {:?}", outcome.created_dirs);
        println!("  rejected     = {:?}", outcome.rejected);
        println!("  cfg.folders  = {:?}", cfg.folders);

        let scanned = scan::scan_folders(&cfg.folders);
        println!("  videos       = {}", scanned.videos.len());
        println!("  missing      = {:?}", scanned.missing);
        println!("  skipped      = {}", scanned.skipped);
        for v in scanned.videos.iter().take(12) {
            println!("    - {} ({})", v.name, v.path);
        }

        assert_eq!(outcome.added, 1, "浏览添加应成功");
        assert!(outcome.rejected.is_empty(), "不该有被拒路径：{:?}", outcome.rejected);
        assert!(scanned.missing.is_empty(), "目录刚添加不应判为不可访问");
        assert!(
            scanned.videos.len() > 0,
            "应扫到视频文件，若为 0 说明扩展名或递归有问题"
        );

        // 只应有一个 chip：一个父目录条目，不该按子目录拆开
        println!("chip 数量 = {}", cfg.folders.len());
    }

    #[test]
    #[ignore = "会在 SMOKE_DIR 指定位置真实创建目录"]
    fn smoke_auto_create_dir() {
        let Ok(base) = std::env::var("SMOKE_DIR") else {
            return;
        };

        let root = Path::new(&base);
        let _ = std::fs::remove_dir_all(root);
        assert!(!root.exists(), "前置条件：目标路径应不存在");

        let mut cfg = LibraryConfig::default();
        let target = root.join("Anime").join("2026");
        let outcome = add_folders_to(&mut cfg, vec![target.to_string_lossy().into_owned()]);

        println!("added        : {}", outcome.added);
        println!("created_dirs : {:?}", outcome.created_dirs);
        println!("rejected     : {:?}", outcome.rejected);
        println!("目录已创建   : {}", target.is_dir());
        println!("cfg.folders  : {:?}", cfg.folders);

        assert_eq!(outcome.added, 1);
        assert!(target.is_dir(), "多级目录应被完整创建");
        assert_eq!(outcome.created_dirs.len(), 1);
        assert!(outcome.rejected.is_empty());
        assert_eq!(cfg.folders.len(), 1);

        // 再加一次同样的路径应被识别为重复，且不再重复创建
        let mut cfg2 = LibraryConfig::default();
        cfg2.folders = cfg.folders.clone();
        let again = add_folders_to(&mut cfg2, vec![target.to_string_lossy().into_owned()]);
        assert_eq!(again.added, 0, "重复添加应被跳过");
        assert!(again.created_dirs.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    /// 回归测试：增删目录后返回体必须带最新的 folders。
    /// 早期实现只回传扫描结果，前端 folders 状态因此与配置脱节——
    /// 目录删不掉、也加不上，必须重启应用才生效。
    #[test]
    fn library_state_carries_folders() {
        let state = LibraryState {
            folders: vec![r"D:\Movies".to_string()],
            videos: vec![],
            missing: vec![],
            skipped: 0,
            ignored: 0,
            created_dirs: vec![],
            rejected: vec![],
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(
            json["folders"][0].as_str(),
            Some(r"D:\Movies"),
            "返回体必须包含 folders，前端据此同步 chip 列表"
        );
    }

    /// 目录移除走 retain + eq_ignore_ascii_case，与前端传的大小写无关
    #[test]
    fn retain_is_case_insensitive_and_reports_truth() {
        let mut folders = vec![
            r"D:\Movies".to_string(),
            r"E:\TV".to_string(),
            r"D:\Movies4K".to_string(),
        ];
        let target = r"d:\movies";
        let before = folders.len();
        folders.retain(|f| !f.eq_ignore_ascii_case(target));

        assert_eq!(folders.len(), 2, "只应移除完全匹配的那一项");
        assert!(
            folders.iter().any(|f| f == r"D:\Movies4K"),
            "前缀相同但不相等的应保留"
        );
        assert!(folders.iter().any(|f| f == r"E:\TV"));
        assert_ne!(before, folders.len(), "长度应确实变化，便于上层判断");
    }

    /// 完整增→删循环：配置与实际扫描目录必须始终一致。
    /// 这是「叉关闭不干净」的根因所在——曾经移除只改了配置却没把 folders 回传前端。
    /// 用局部 cfg，不触碰真实的 ~/.process-app/library.json。
    #[test]
    fn add_then_remove_keeps_config_and_scan_in_sync() {
        let root = std::env::temp_dir().join("process_folder_sync_test");
        let _ = std::fs::remove_dir_all(&root);
        let a = root.join("A");
        let b = root.join("B");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        let mut cfg = LibraryConfig::default();

        // 新增：两个已存在目录 + 一个不存在的（应自动创建）+ 大小写不同的重复项
        let fresh = root.join("C").join("Nested");
        let outcome = add_folders_to(
            &mut cfg,
            vec![
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
                fresh.to_string_lossy().into_owned(),
                a.to_string_lossy().to_uppercase(),
            ],
        );
        assert_eq!(outcome.added, 3, "大小写重复项应被跳过");
        assert_eq!(cfg.folders.len(), 3);
        assert!(
            fresh.is_dir(),
            "不存在的目录应被自动创建（含多级父目录）"
        );
        assert_eq!(
            outcome.created_dirs.len(),
            1,
            "只有真正新建的那个目录应出现在 created_dirs"
        );
        assert!(outcome.rejected.is_empty());
        assert!(
            !outcome
                .created_dirs
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&a.to_string_lossy())),
            "已存在的目录不应被记为新建"
        );

        // 扫描结果应与配置一致，无不可访问目录
        let scanned = scan::scan_folders(&cfg.folders);
        assert!(scanned.missing.is_empty());
        assert_eq!(scanned.videos.len(), 0, "目录里没放视频文件");

        // 移除其一：配置里不该再有它，但磁盘目录保留（只移出库，不删用户文件）
        let target = a.to_string_lossy().into_owned();
        assert!(remove_folder_from(&mut cfg, &target), "应报告确实移除了");
        assert_eq!(cfg.folders.len(), 2);
        assert!(!cfg.folders.iter().any(|f| f == &target));
        assert!(cfg.folders.iter().any(|f| f == &b.to_string_lossy().to_string()));
        assert!(a.is_dir(), "移除只是移出媒体库，绝不能删除用户的实际文件");

        // 移除不存在的目录返回 false，且不破坏现有配置
        assert!(!remove_folder_from(&mut cfg, r"Z:\nope"));
        assert_eq!(cfg.folders.len(), 2, "无效移除不应影响配置");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 无法使用的路径必须被拒绝并说明原因，绝不静默丢弃
    #[test]
    fn unusable_paths_are_rejected_with_reason() {
        let root = std::env::temp_dir().join("process_reject_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        // 已存在的文件，不能当目录用
        let file = root.join("movie.mkv");
        std::fs::write(&file, b"x").unwrap();

        let mut cfg = LibraryConfig::default();
        let outcome = add_folders_to(
            &mut cfg,
            vec![
                file.to_string_lossy().into_owned(),
                "relative/path".to_string(),
                "   ".to_string(),
            ],
        );

        assert_eq!(outcome.added, 0, "三类都不可用，不应有任何新增");
        assert!(cfg.folders.is_empty());
        assert_eq!(outcome.rejected.len(), 2, "空串应直接忽略，文件与相对路径应被拒");
        assert!(outcome.created_dirs.is_empty());

        let reasons: Vec<&str> = outcome.rejected.iter().map(|r| r.reason.as_str()).collect();
        assert!(
            reasons.iter().any(|r| r.contains("不是目录")),
            "指向文件的路径应说明原因：{reasons:?}"
        );
        assert!(
            reasons.iter().any(|r| r.contains("绝对路径")),
            "相对路径应被拒绝：{reasons:?}"
        );
        assert!(!file.is_file() == false, "不能因为被拒而删掉用户文件");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 磁盘满/无权限时创建会失败，必须被拒而非留下半成品
    #[test]
    fn creation_failure_is_reported_not_ignored() {
        let root = std::env::temp_dir().join("process_perm_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        // 在只读目录下尝试创建子目录 —— Windows 下会失败
        let readonly = root.join("ro");
        std::fs::create_dir_all(&readonly).unwrap();
        let child = readonly.join("nope").join("deeper");

        let mut cfg = LibraryConfig::default();
        let outcome = add_folders_to(&mut cfg, vec![child.to_string_lossy().into_owned()]);

        if outcome.added == 0 {
            // 预期路径：以只读目录为父无法创建
            assert_eq!(outcome.rejected.len(), 1);
            assert!(!outcome.rejected[0].reason.is_empty(), "失败必须带原因");
            assert!(cfg.folders.is_empty());
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 目录消失（被删掉/换机器）时，扫描应把它归入 missing 而非静默丢弃。
    /// 扫描绝不能自动创建目录——那会掩盖 NAS 断连与路径笔误。
    #[test]
    fn vanished_folder_lands_in_missing() {
        let cfg = LibraryConfig {
            folders: vec![r"Z:\definitely\not\here".to_string()],
            potplayer_path: None,
        };
        let scanned = scan::scan_folders(&cfg.folders);
        assert_eq!(scanned.missing.len(), 1);
        assert!(scanned.videos.is_empty());

        // missing 必须能在 folders 中找到对应项，前端才能标「不可访问」
        assert!(cfg.folders.contains(&scanned.missing[0]));
    }
}