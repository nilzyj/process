use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 运行时存储配置。SQLite 为本地单文件，路径由 data_dir 决定。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// SQLite 数据库所在目录
    pub data_dir: PathBuf,
    /// 快照导出目录（通常设为坚果云同步文件夹）。None 表示不导出
    #[serde(default)]
    pub snapshot_dir: Option<PathBuf>,
    /// 快照保留份数，超出后删除最旧的
    #[serde(default = "default_keep_snapshots")]
    pub keep_snapshots: usize,
}

fn default_keep_snapshots() -> usize {
    10
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            snapshot_dir: None,
            keep_snapshots: default_keep_snapshots(),
        }
    }
}

impl AppConfig {
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("data.sqlite")
    }
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".process-app")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// 媒体库配置独立于存储配置：
/// 旧「断开」按钮会以空值覆写 config.json，若把 folders 放进去会被连带清空。
fn library_config_path() -> PathBuf {
    config_dir().join("library.json")
}

pub fn default_data_dir() -> PathBuf {
    config_dir()
}

/// 旧版 config.json 存的是扁平的 MySQL 结构，且含明文密码。
/// 切换到 SQLite 后该结构不再被识别，这里只做只读检测，
/// 供设置页提示用户手动清理磁盘上的遗留凭据。
pub fn legacy_mysql_config_present() -> bool {
    let Ok(content) = std::fs::read_to_string(config_path()) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    value.get("host").is_some() && value.get("password").is_some()
}

pub fn load_app_config() -> AppConfig {
    match std::fs::read_to_string(config_path()) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_app_config(config: &AppConfig) -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path(), content)?;
    Ok(())
}

pub fn load_library_config() -> LibraryConfig {
    let mut cfg = match std::fs::read_to_string(library_config_path()) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => LibraryConfig::default(),
    };
    cfg.folders = normalize_folders(cfg.folders);
    cfg
}

pub fn save_library_config(config: &LibraryConfig) -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let mut out = config.clone();
    out.folders = normalize_folders(out.folders);
    let content = serde_json::to_string_pretty(&out)?;
    std::fs::write(library_config_path(), content)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LibraryConfig {
    /// 已添加的媒体库目录（绝对路径，已归一化）
    #[serde(default)]
    pub folders: Vec<String>,
    /// 手动指定的 PotPlayer 可执行文件路径，None 表示自动探测
    #[serde(default)]
    pub potplayer_path: Option<String>,
}

/// 规范化目录路径：统一分隔符、去掉尾随分隔符。
///
/// Windows 上 `E:\电影\` 与 `E:\电影` 会被判为不同目录，
/// 于是同一文件夹出现两个 chip。手动输入时带尾随斜杠很常见，必须归一。
/// 根路径（`E:\`）不能剥成 `E:`，那会变成「当前驱动器目录」。
pub fn normalize_folder_path(p: &str) -> String {
    let s = p.trim().replace('/', "\\");
    if s.is_empty() {
        return s;
    }

    let trimmed = s.trim_end_matches('\\');

    // `E:` / `E:\` —— 剥完只剩盘符，补回分隔符
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        return format!("{}\\", trimmed);
    }
    // 剥空说明本来就是根（`\` 或 `\\`），交还原值
    if trimmed.is_empty() {
        return s;
    }
    trimmed.to_string()
}

/// 归一并去重。加载时应用，使旧配置里遗留的 `E:\foo\` 自动修正。
fn normalize_folders(folders: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(folders.len());
    for f in folders {
        let n = normalize_folder_path(&f);
        if n.is_empty() {
            continue;
        }
        if !out.iter().any(|e| e.eq_ignore_ascii_case(&n)) {
            out.push(n);
        }
    }
    out
}

pub fn ensure_dir(path: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

#[test]
    fn app_config_tolerates_legacy_config_json() {
        // 旧 config.json 是扁平的 MySQL 结构（含明文密码），
        // 反序列化到 AppConfig 必须失败并回落默认值，而不是 panic
        let legacy = r#"{"host":"h","port":3306,"user":"u","password":"p","database":"d"}"#;
        let parsed: Result<AppConfig, _> = serde_json::from_str(legacy);
        assert!(parsed.is_err(), "旧结构不应被误认作 AppConfig");
        assert!(AppConfig::default().database_path().ends_with("data.sqlite"));
    }

    #[test]
    fn app_config_roundtrip() {
        let mut c = AppConfig::default();
        c.keep_snapshots = 3;
        c.snapshot_dir = Some(PathBuf::from(r"D:\Nutstore\process"));
        let json = serde_json::to_string(&c).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.keep_snapshots, 3);
        assert_eq!(back.snapshot_dir, c.snapshot_dir);
        assert_eq!(back.data_dir, c.data_dir);
    }

    #[test]
    fn keep_snapshots_defaults_when_absent() {
        let c: AppConfig = serde_json::from_str(r#"{"data_dir":"D:\\x"}"#).unwrap();
        assert_eq!(c.keep_snapshots, 10);
        assert!(c.snapshot_dir.is_none());
    }

    #[test]
    fn library_config_defaults_when_empty() {
        let c: LibraryConfig = serde_json::from_str("{}").unwrap();
        assert!(c.folders.is_empty());
        assert!(c.potplayer_path.is_none());
    }

    #[test]
    fn normalizes_trailing_separator() {
        assert_eq!(normalize_folder_path(r"E:\电影\"), r"E:\电影");
        assert_eq!(normalize_folder_path(r"E:\电影"), r"E:\电影");
        assert_eq!(normalize_folder_path(r"E:\电影\\"), r"E:\电影");
        assert_eq!(normalize_folder_path("E:/电影/"), r"E:\电影");
        assert_eq!(normalize_folder_path("  E:\\电影  "), r"E:\电影");
    }

    #[test]
    fn preserves_drive_roots() {
        // 剥成 "E:" 会被解释成「当前驱动器目录」，必须补回分隔符
        assert_eq!(normalize_folder_path(r"E:\"), r"E:\");
        assert_eq!(normalize_folder_path("E:"), r"E:\");
        assert_eq!(normalize_folder_path(r"\\server\share\"), r"\\server\share");
        assert_eq!(normalize_folder_path(r"\"), r"\");
        assert_eq!(normalize_folder_path(""), "");
    }

    /// 同一目录的两种写法必须折叠成一条，否则界面出现重复 chip
    #[test]
    fn dedups_equivalent_spellings() {
        let out = normalize_folders(vec![
            r"E:\电影\".into(),
            r"E:\电影".into(),
            r"E:/电影".into(),
            r"e:\电影".into(),
            r"E:\剧集\".into(),
        ]);
        assert_eq!(out.len(), 2, "实际: {out:?}");
        assert_eq!(out[0], r"E:\电影");
        assert_eq!(out[1], r"E:\剧集");
    }
}