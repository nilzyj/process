use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3306,
            user: "root".into(),
            password: String::new(),
            database: "process".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LibraryConfig {
    /// 已添加的媒体库目录（绝对路径）
    #[serde(default)]
    pub folders: Vec<String>,
    /// 手动指定的 PotPlayer 可执行文件路径，None 表示自动探测
    #[serde(default)]
    pub potplayer_path: Option<String>,
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".process-app")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// 媒体库配置独立于 DbConfig。
/// 前端“断开”按钮会以空值覆写 config.json，若把 folders 放进 DbConfig 会被一并清空。
fn library_config_path() -> PathBuf {
    config_dir().join("library.json")
}

pub fn load_config() -> Option<DbConfig> {
    let path = config_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_config(config: &DbConfig) -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path(), content)?;
    Ok(())
}

pub fn load_library_config() -> LibraryConfig {
    match std::fs::read_to_string(library_config_path()) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => LibraryConfig::default(),
    }
}

pub fn save_library_config(config: &LibraryConfig) -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(library_config_path(), content)?;
    Ok(())
}
