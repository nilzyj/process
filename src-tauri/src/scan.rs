use std::path::Path;
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::models::{ScanResult, VideoFile};

/// 视频扩展名白名单（小写，不含点）。
/// 刻意排除 iso —— 光盘镜像需要挂载/解轨逻辑，不在当前范围内。
const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "ts", "m2ts", "mpg", "mpeg", "vob", "webm", "rm",
    "rmvb",
];

/// NAS 同步工具产生的垃圾目录，跳过可显著减少无谓遍历
const JUNK_DIRS: &[&str] = &["@eaDir", "$RECYCLE.BIN", "System Volume Information", "#recycle"];

fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .map(|e| VIDEO_EXTS.contains(&e.as_str()))
        .unwrap_or(false)
}

fn is_skippable_dir(name: &str) -> bool {
    name.starts_with('.') || JUNK_DIRS.contains(&name)
}

/// 生成显示名：去扩展名、下划线转空格、压缩多余空白
fn display_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    stem.replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn to_unix_secs(t: std::io::Result<std::time::SystemTime>) -> i64 {
    t.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 递归扫描所有目录，返回视频文件列表与无法访问的目录。
/// 单个文件读取失败只计数，不中断整体扫描。
pub fn scan_folders(folders: &[String]) -> ScanResult {
    let mut videos = Vec::new();
    let mut missing = Vec::new();
    let mut skipped = 0usize;

    for folder in folders {
        let root = Path::new(folder);
        if !root.is_dir() {
            missing.push(folder.clone());
            continue;
        }

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| e.depth() == 0 || !is_skippable_dir(n))
                    .unwrap_or(true)
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };

            if !entry.file_type().is_file() || !is_video(entry.path()) {
                continue;
            }

            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };

            videos.push(VideoFile {
                path: entry.path().to_string_lossy().to_string(),
                name: display_name(entry.path()),
                size: meta.len(),
                mtime: to_unix_secs(meta.modified()),
            });
        }
    }

    // 按显示名排序，中文用 Unicode 码点序，行为可预测
    videos.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));

    ScanResult {
        videos,
        missing,
        skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_video_extensions() {
        assert!(is_video(Path::new("a.mkv")));
        assert!(is_video(Path::new("a.MP4")));
        assert!(!is_video(Path::new("a.txt")));
        assert!(!is_video(Path::new("a")));
    }

    #[test]
    fn cleans_display_name() {
        assert_eq!(display_name(Path::new("Some_Movie_Name.mkv")), "Some Movie Name");
        assert_eq!(display_name(Path::new("A__B.mkv")), "A B");
    }

    #[test]
    fn skips_junk_dirs() {
        assert!(is_skippable_dir("@eaDir"));
        assert!(is_skippable_dir(".hidden"));
        assert!(!is_skippable_dir("Movies"));
    }

    /// 在真实目录树上验证递归、过滤、垃圾目录跳过与 missing 统计
    #[test]
    fn scans_real_tree() {
        use std::fs;

        let root = std::env::temp_dir().join("process_scan_test");
        let _ = fs::remove_dir_all(&root);

        // 期望收录 3 个
        fs::create_dir_all(root.join("Movies/Action")).unwrap();
        fs::create_dir_all(root.join("@eaDir")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("Movies/Top_Movie.mkv"), b"x").unwrap();
        fs::write(root.join("Movies/Action/Inner.MP4"), b"x").unwrap();
        fs::write(root.join("Movies/notes.txt"), b"x").unwrap();
        fs::write(root.join("Movies/Action/skip_this.rmvb"), b"x").unwrap();
        // 以下三个都应被跳过
        fs::write(root.join("@eaDir/junk.mkv"), b"x").unwrap();
        fs::write(root.join(".hidden/secret.mkv"), b"x").unwrap();
        fs::write(root.join("Movies/readme.md"), b"x").unwrap();

        let folders = vec![root.to_string_lossy().to_string()];
        let r = scan_folders(&folders);

        assert!(r.missing.is_empty(), "目录应可访问");
        assert_eq!(r.skipped, 0, "无权限错误时 skipped 应为 0");

        let names: Vec<&str> = r.videos.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names.len(), 3, "实际收录: {:?}", names);
        assert!(names.contains(&"Top Movie"), "下划线应转空格: {:?}", names);
        assert!(names.contains(&"Inner"), "递归应进入子目录: {:?}", names);
        assert!(names.contains(&"skip this"), "小写扩展名应识别: {:?}", names);
        assert!(
            !r.videos.iter().any(|v| v.path.contains("@eaDir")),
            "@eaDir 应被跳过"
        );
        assert!(
            !r.videos.iter().any(|v| v.path.contains(".hidden")),
            "隐藏目录应被跳过"
        );
        assert!(
            !r.videos.iter().any(|v| v.path.ends_with(".txt")),
            "非视频应被过滤"
        );

        // 结果按显示名排序
        let sorted: Vec<&str> = r.videos.iter().map(|v| v.name.as_str()).collect();
        let mut expect = sorted.clone();
        expect.sort();
        assert_eq!(sorted, expect, "结果应有序");

        // 不存在的目录应进 missing，且不中断其他目录
        let mut mixed = folders.clone();
        mixed.push("Z:\\definitely\\not\\exist".to_string());
        let r2 = scan_folders(&mixed);
        assert_eq!(r2.videos.len(), 3, "有效目录仍应产出结果");
        assert_eq!(r2.missing.len(), 1, "无效目录应进 missing");

        let _ = fs::remove_dir_all(&root);
    }
}