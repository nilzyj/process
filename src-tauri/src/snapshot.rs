use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::db;
use crate::models::Record;

pub const SNAPSHOT_VERSION: u32 = 1;

/// 快照文件格式。既是导出备份，也是 MySQL→SQLite 迁移与恢复的载体，
/// 因此导入逻辑只有一条路径。
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub exported_at: String,
    pub checksum: String,
    pub records: Vec<Record>,
}

/// FNV-1a 64 位。仅用于发现传输截断/文件损坏，不需要密码学强度，避免引入额外依赖。
fn fnv1a64(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn checksum_of(records: &[Record]) -> Result<String> {
    let json = serde_json::to_vec(records).context("序列化记录失败")?;
    Ok(fnv1a64(&json))
}

fn parse_snapshot(raw: &str) -> Result<Snapshot> {
    let snap: Snapshot =
        serde_json::from_str(raw).context("快照文件格式不正确，无法解析为 JSON")?;

    if snap.version != SNAPSHOT_VERSION {
        anyhow::bail!(
            "快照版本不兼容：文件为 v{}，当前程序支持 v{}",
            snap.version,
            SNAPSHOT_VERSION
        );
    }

    let actual = checksum_of(&snap.records)?;
    if actual != snap.checksum {
        anyhow::bail!("快照校验失败，文件可能已损坏（期望 {}，实际 {}）", snap.checksum, actual);
    }

    Ok(snap)
}

/// 先写临时文件再 rename。坚果云同步目录中断写入时，
/// 目标文件要么是旧的完整内容，要么是新内容，不会是半截。
fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let dir = path.parent().context("快照路径缺少父目录")?;
    crate::config::ensure_dir(dir)?;

    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents).context("写临时文件失败")?;
    std::fs::rename(&tmp, path).context("重命名临时文件失败")?;
    Ok(())
}

fn stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string()
}

/// 秒级时间戳在同一秒内会重复，导致连续导出互相覆盖、历史快照丢失。
/// 已存在时追加序号，直到找到空位。
fn unique_path(dir: &Path) -> PathBuf {
    let base = stamp();
    let candidate = dir.join(format!("process_{base}.json"));
    if !candidate.exists() {
        return candidate;
    }
    for n in 2..10_000 {
        let p = dir.join(format!("process_{base}_{n}.json"));
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!("process_{base}_{}.json", std::process::id()))
}

/// 导出快照到坚果云目录（未配置则返回错误）。
/// 额外写一份 process_latest.json 便于人工取用。
pub async fn export(pool: &sqlx::SqlitePool, config: &AppConfig) -> Result<String> {
    let dir = config
        .snapshot_dir
        .clone()
        .context("尚未配置快照目录，请在设置中指定坚果云同步文件夹")?;
    crate::config::ensure_dir(&dir)?;

    let records = db::all_records(pool).await?;
    let snap = Snapshot {
        version: SNAPSHOT_VERSION,
        exported_at: db::now_str(),
        checksum: checksum_of(&records)?,
        records,
    };
    let json = serde_json::to_string(&snap).context("序列化快照失败")?;

    let path = unique_path(&dir);
    atomic_write(&path, &json)?;
    atomic_write(&dir.join("process_latest.json"), &json)?;

    let keep = config.keep_snapshots.max(1);
    prune_old(&dir, keep)?;

    Ok(path.to_string_lossy().into_owned())
}

/// 保留最近 keep 份带时间戳的快照，latest 与当前导出不动。
fn prune_old(dir: &Path, keep: usize) -> Result<()> {
    let mut dated: Vec<PathBuf> = std::fs::read_dir(dir)
        .context("读取快照目录失败")?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("process_") && n.ends_with(".json") && n != "process_latest.json")
                .unwrap_or(false)
        })
        .collect();

    if dated.len() <= keep {
        return Ok(());
    }

    // 文件名内嵌时间戳，字典序即时间序
    dated.sort();
    let to_remove = dated.len() - keep;
    for p in dated.into_iter().take(to_remove) {
        std::fs::remove_file(&p).with_context(|| format!("删除旧快照失败：{}", p.display()))?;
    }
    Ok(())
}

/// 列出可用快照，按新到旧排序，供恢复界面展示。
pub fn list_snapshots(dir: &Path) -> Vec<SnapshotInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut out: Vec<SnapshotInfo> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        })
        .map(|p| {
            let meta = std::fs::metadata(&p).ok();
            SnapshotInfo {
                path: p.to_string_lossy().into_owned(),
                name: p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string(),
                size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            }
        })
        .collect();

    out.sort_by(|a, b| b.name.cmp(&a.name));
    out
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    /// Unix 秒
    pub modified: u64,
}

/// 从快照文件恢复。会先校验 checksum，再整表替换。
pub async fn restore(pool: &sqlx::SqlitePool, path: &str) -> Result<usize> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("读取失败：{path}"))?;
    let snap = parse_snapshot(&raw)?;
    let count = db::replace_all_records(pool, &snap.records, true).await?;
    Ok(count)
}

/// 从导出的 JSON 记录数组导入（MySQL 迁移路径）。
/// 兼容两种顶层形态：裸数组，或 { records: [...] }。
pub async fn import_records_file(pool: &sqlx::SqlitePool, path: &str) -> Result<usize> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("读取失败：{path}"))?;

    let records: Vec<Record> = match serde_json::from_str::<Vec<Record>>(&raw) {
        Ok(v) => v,
        Err(_) => {
            let snap = parse_snapshot(&raw)?;
            snap.records
        }
    };

    if records.is_empty() {
        anyhow::bail!("文件中没有可导入的记录");
    }

    let count = db::replace_all_records(pool, &records, true).await?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewRecord;

    fn rec(id: i64, name: &str) -> Record {
        Record {
            id,
            record_name: name.into(),
            season: None,
            remark: Some(String::new()),
            media_type: Some("电影".into()),
            status: Some("已完成".into()),
            end_time: Some("2026-10-04 12:00:00".into()),
            country: Some("美国".into()),
            tags: Some("系列, 经典".into()),
            current_episode: Some(1),
            total_episode: Some(1),
            year: Some(2020),
            modify_time: Some("2026-10-04 12:00:00".into()),
        }
    }

    #[test]
    fn checksum_detects_tampering() {
        let records = vec![rec(1, "A"), rec(2, "B")];
        let snap = Snapshot {
            version: SNAPSHOT_VERSION,
            exported_at: "2026-10-04 00:00:00".into(),
            checksum: checksum_of(&records).unwrap(),
            records,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(parse_snapshot(&json).is_ok(), "正常快照应通过校验");

        // 改动一条记录的名称后，checksum 应对不上
        let mut tampered = snap;
        tampered.records[0].record_name = "篡改".into();
        let bad = serde_json::to_string(&tampered).unwrap();
        let err = parse_snapshot(&bad).unwrap_err().to_string();
        assert!(err.contains("校验失败"), "实际错误：{err}");
    }

    #[test]
    fn rejects_wrong_version() {
        let records = vec![rec(1, "A")];
        let snap = Snapshot {
            version: SNAPSHOT_VERSION + 99,
            exported_at: "2026-10-04 00:00:00".into(),
            checksum: checksum_of(&records).unwrap(),
            records,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let err = parse_snapshot(&json).unwrap_err().to_string();
        assert!(err.contains("版本不兼容"), "实际错误：{err}");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_snapshot("not json").is_err());
        assert!(parse_snapshot("{}").is_err());
    }

    /// 回归测试：MySQL 导出的 JSON 用 "type" 作为列名，
    /// 而 serde 字段名是 media_type。若缺少 alias，整列会被静默丢成 NULL。
    #[test]
    fn accepts_mysql_style_type_key() {
        let mysql_dump = r#"[
            {"id":1,"record_name":"A","type":"电影","status":"已完成",
             "end_time":"2026-10-04 00:00:00","country":"美国","tags":"系列",
             "current_episode":1,"total_episode":1,"year":2020,
             "modify_time":"2026-10-04 00:00:00"},
            {"id":2,"record_name":"B","type":"动漫","status":"进行中",
             "end_time":null,"country":null,"tags":null,
             "current_episode":null,"total_episode":null,"year":null,
             "modify_time":"2026-10-04 00:00:00"}
        ]"#;

        let parsed: Vec<Record> = serde_json::from_str(mysql_dump).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].media_type.as_deref(), Some("电影"));
        assert_eq!(parsed[1].media_type.as_deref(), Some("动漫"));
    }

    /// 反向：快照自身写出的键名是 media_type，需能被自己读回
    #[test]
    fn snapshot_roundtrip_preserves_media_type() {
        let records = vec![rec(1, "A")];
        let snap = Snapshot {
            version: SNAPSHOT_VERSION,
            exported_at: "2026-10-04 00:00:00".into(),
            checksum: checksum_of(&records).unwrap(),
            records,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"media_type\""), "序列化键名应为 media_type");
        let back = parse_snapshot(&json).unwrap();
        assert_eq!(back.records[0].media_type.as_deref(), Some("电影"));
    }

    #[tokio::test]
    async fn export_restore_roundtrip() {
        let dir = std::env::temp_dir().join("process_e2e_test");
        let _ = std::fs::remove_dir_all(&dir);
        let snap_dir = dir.join("snapshots");
        let cfg = crate::config::AppConfig {
            data_dir: dir.join("db"),
            snapshot_dir: Some(snap_dir.clone()),
            keep_snapshots: 3,
        };
        let pool = crate::db::connect(&cfg).await.unwrap();

        // 写入 4 条
        for i in 1..=4 {
            let mut r = rec(i, &format!("R{i}"));
            r.status = Some("已完成".into());
            pool_let(&pool, r).await;
        }
        assert_eq!(crate::db::count_records(&pool).await.unwrap(), 4);

        let path = export(&pool, &cfg).await.unwrap();
        assert!(std::path::Path::new(&path).exists(), "快照文件应存在");
        assert!(snap_dir.join("process_latest.json").exists(), "应同时写 latest");

        // 校验导出文件可被解析且 checksum 通过
        let raw = std::fs::read_to_string(&path).unwrap();
        let snap = parse_snapshot(&raw).unwrap();
        assert_eq!(snap.records.len(), 4);
        assert_eq!(snap.version, SNAPSHOT_VERSION);

        // 清库后从快照恢复
        crate::db::replace_all_records(&pool, &[], true).await.unwrap();
        assert_eq!(crate::db::count_records(&pool).await.unwrap(), 0);

        let n = restore(&pool, &path).await.unwrap();
        assert_eq!(n, 4);
        let back = crate::db::all_records(&pool).await.unwrap();
        assert_eq!(back.len(), 4);
        assert_eq!(back[2].record_name, "R3");
        assert_eq!(back[0].media_type.as_deref(), Some("电影"));

        // 恢复后再导出一次，触发保留份数裁剪
        for _ in 0..4 {
            export(&pool, &cfg).await.unwrap();
        }
        let dated: Vec<String> = std::fs::read_dir(&snap_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("process_") && n != "process_latest.json")
            .collect();
        assert_eq!(dated.len(), 3, "应只保留 3 份带时间戳快照，实际 {dated:?}");

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    async fn pool_let(pool: &sqlx::SqlitePool, r: Record) {
        crate::db::add_record(
            pool,
            NewRecord {
                record_name: r.record_name,
                season: r.season,
                remark: r.remark,
                media_type: r.media_type,
                status: r.status,
                end_time: r.end_time,
                country: r.country,
                tags: r.tags,
                current_episode: r.current_episode,
                total_episode: r.total_episode,
                year: r.year,
            },
        )
        .await
        .unwrap();
    }

    /// 对真实配置指向的数据库导出一份快照，用于验证坚果云目录写入。
    /// 默认跳过：RECORDS_JSON 环境变量存在时才运行。
    #[tokio::test]
    #[ignore = "会向真实快照目录写文件，需显式触发"]
    async fn export_real_snapshot() {
        if std::env::var("RECORDS_JSON").is_err() {
            println!("未设置 RECORDS_JSON，跳过");
            return;
        }
        let cfg = crate::config::load_app_config();
        println!("快照目录: {:?}", cfg.snapshot_dir);
        let pool = crate::db::connect(&cfg).await.expect("connect");
        println!("库内记录: {}", crate::db::count_records(&pool).await.unwrap_or(0));

        let path = export(&pool, &cfg).await.expect("导出失败");
        println!("导出到: {path}");

        let raw = std::fs::read_to_string(&path).unwrap();
        let snap = parse_snapshot(&raw).expect("导出文件应能通过校验");
        println!("快照内记录: {}", snap.records.len());
        println!("checksum: {}", snap.checksum);
        println!("exported_at: {}", snap.exported_at);
        println!("文件大小: {} bytes", std::fs::metadata(&path).unwrap().len());

        drop(pool);
    }

    #[tokio::test]
    async fn import_keeps_type_column_from_mysql_dump() {
        let dir = std::env::temp_dir().join("process_import_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let src = dir.join("records.json");
        std::fs::write(
            &src,
            r#"[{"id":1,"record_name":"A","type":"电影","status":"已完成",
                 "modify_time":"2026-10-04 00:00:00"}]"#,
        )
        .unwrap();

        let cfg = crate::config::AppConfig {
            data_dir: dir.clone(),
            snapshot_dir: None,
            keep_snapshots: 10,
        };
        let pool = crate::db::connect(&cfg).await.unwrap();

        let n = import_records_file(&pool, &src.to_string_lossy()).await.unwrap();
        assert_eq!(n, 1);

        let all = crate::db::all_records(&pool).await.unwrap();
        assert_eq!(
            all[0].media_type.as_deref(),
            Some("电影"),
            "type 列不应在导入后丢失"
        );

        // 统计侧也不应塌成一组
        let s = crate::db::get_stats(&pool).await.unwrap();
        assert_eq!(s.by_type.len(), 1);
        assert_eq!(s.by_type[0].media_type, "电影");
        assert_eq!(s.by_type[0].count, 1);

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prunes_oldest_beyond_keep() {
        let dir = std::env::temp_dir().join("process_prune_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        for t in [
            "2026-10-01_090000",
            "2026-10-02_090000",
            "2026-10-03_090000",
            "2026-10-04_090000",
            "2026-10-05_090000",
        ] {
            std::fs::write(dir.join(format!("process_{t}.json")), "{}").unwrap();
        }
        std::fs::write(dir.join("process_latest.json"), "{}").unwrap();

        prune_old(&dir, 3).unwrap();

        let left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(left.contains(&"process_latest.json".to_string()), "latest 不应被删");
        assert_eq!(left.len(), 4, "保留 3 份带时间戳 + latest，实际：{left:?}");
        assert!(
            !left.contains(&"process_2026-10-01_090000.json".to_string()),
            "最旧的应被删：{left:?}"
        );
        assert!(left.contains(&"process_2026-10-05_090000.json".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}