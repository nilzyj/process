use std::path::Path;
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::models::VideoFile;

/// 一次目录遍历的原始产出。只关心文件系统，不含配置概念。
#[derive(Debug, Default)]
pub struct ScanOutcome {
    pub videos: Vec<VideoFile>,
    /// 无法访问的目录
    pub missing: Vec<String>,
    /// 因权限或 I/O 错误跳过的文件数
    pub skipped: usize,
    /// 判定为非视频而排除的文件数（过小，或 TypeScript 声明文件）
    pub ignored: usize,
}

/// `.d.ts` 是 TypeScript 声明文件，按定义不可能是视频。
///
/// 体积下限能挡掉常见的几十字节存根，但 lucide-react 这类包会生成
/// 2.5 MB 的 `.d.ts`，仍然漏网，因此按文件名精确排除。
/// 不能靠提高体积门槛解决——总有更大的声明文件。
///
/// 注意 `tsconfig.tsbuildinfo` 无需在此处理：其扩展名不是 `ts`，
/// 在扩展名白名单那一步就已被排除。
fn is_typescript_declaration(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".d.ts") || n.ends_with(".d.mts") || n.ends_with(".d.cts"))
        .unwrap_or(false)
}

/// 视频扩展名白名单（小写，不含点）。
/// 刻意排除 iso —— 光盘镜像需要挂载/解轨逻辑，不在当前范围内。
const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "ts", "m2ts", "mpg", "mpeg", "vob", "webm", "rm",
    "rmvb",
];

/// 视频文件体积下限（1 MiB）。
///
/// `.ts` 是二义扩展名：既指 MPEG 传输流，也指 TypeScript。
/// 在含 node_modules 的目录里扫描时，成千上万个几十字节的 `.d.ts`
/// 会被仅凭扩展名判成视频，进而造出名为 `fp` / `dts` / `compat`
/// 的几百个无意义分组。体积下限能一次性排除这类伪视频，
/// 同时顺带滤掉损坏或占位的空文件。
///
/// 不能改为直接剔除 `.ts`：电视录制的 MPEG-TS 是正常媒体文件。
const MIN_VIDEO_BYTES: u64 = 1024 * 1024;

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
pub fn scan_folders(folders: &[String]) -> ScanOutcome {
    let mut videos = Vec::new();
    let mut missing = Vec::new();
    let mut skipped = 0usize;
    let mut ignored = 0usize;

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

            // TypeScript 声明文件无论多大都不是视频
            if is_typescript_declaration(entry.path()) {
                ignored += 1;
                continue;
            }

            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };

            // 仅凭扩展名不足以判定：.ts 同时是 MPEG 传输流与 TypeScript
            if meta.len() < MIN_VIDEO_BYTES {
                ignored += 1;
                continue;
            }

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

    ScanOutcome {
        videos,
        missing,
        skipped,
        ignored,
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

        // 真实视频不可能只有 1 字节，fixture 必须越过体积下限
        let payload = vec![0u8; MIN_VIDEO_BYTES as usize + 1];

        // 期望收录 3 个
        fs::create_dir_all(root.join("Movies/Action")).unwrap();
        fs::create_dir_all(root.join("@eaDir")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("Movies/Top_Movie.mkv"), &payload).unwrap();
        fs::write(root.join("Movies/Action/Inner.MP4"), &payload).unwrap();
        fs::write(root.join("Movies/notes.txt"), b"x").unwrap();
        fs::write(root.join("Movies/Action/skip_this.rmvb"), &payload).unwrap();
        // 以下三个都应被跳过
        fs::write(root.join("@eaDir/junk.mkv"), &payload).unwrap();
        fs::write(root.join(".hidden/secret.mkv"), &payload).unwrap();
        fs::write(root.join("Movies/readme.md"), b"x").unwrap();

        let folders = vec![root.to_string_lossy().to_string()];
        let r = scan_folders(&folders);

        assert!(r.missing.is_empty(), "目录应可访问");
        assert_eq!(r.skipped, 0, "无权限错误时 skipped 应为 0");
        assert_eq!(r.ignored, 0, "fixture 均越过体积下限且非 .d.ts");

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

    /// 回归测试：`.ts` 既指 MPEG 传输流也指 TypeScript。
    /// node_modules 里几十字节的 .d.ts 曾被当成视频，
    /// 造出名为 fp / dts / compat 的成百上千个无意义分组。
    /// 体积下限挡不住大型声明文件：lucide-react 的 dynamic.d.ts 有 2.5 MB。
    #[test]
    fn large_typescript_declarations_are_excluded() {
        use std::fs;

        let root = std::env::temp_dir().join("process_dts_test");
        let _ = fs::remove_dir_all(&root);
        let pkg = root.join("node_modules/lucide-react");
        fs::create_dir_all(&pkg).unwrap();

        // 2.5 MB 的声明文件，远超 1 MiB 门槛
        fs::write(
            pkg.join("dynamic.d.ts"),
            vec![b'x'; 2_500_000],
        )
        .unwrap();
        // 扩展名不是 ts，本就在白名单外，不计入 ignored
        fs::write(pkg.join("tsconfig.tsbuildinfo"), vec![b'x'; 1_200_000]).unwrap();

        let r = scan_folders(&[root.to_string_lossy().to_string()]);

        assert!(r.videos.is_empty(), "声明文件不应被收录");
        assert_eq!(r.ignored, 1, "只有 .d.ts 计入 ignored");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn typescript_declarations_are_not_videos() {
        use std::fs;

        let root = std::env::temp_dir().join("process_ts_test");
        let _ = fs::remove_dir_all(&root);
        let pkg = root.join("node_modules/@types/lodash/fp");
        fs::create_dir_all(&pkg).unwrap();

        // 386 个几十字节的 .d.ts，正是 node_modules 的真实形态
        for name in ["add", "after", "all", "allPass", "always"] {
            fs::write(pkg.join(format!("{name}.d.ts")), b"export declare function x(): void;\n").unwrap();
        }
        // 同目录下一个真正的 MPEG-TS 录制文件应当被收录
        fs::write(
            pkg.join("recording.ts"),
            vec![0u8; MIN_VIDEO_BYTES as usize + 1],
        )
        .unwrap();

        let r = scan_folders(&[root.to_string_lossy().to_string()]);

        assert_eq!(
            r.videos.len(),
            1,
            "只应收录真正的 TS 视频，实际: {:?}",
            r.videos.iter().map(|v| &v.name).collect::<Vec<_>>()
        );
        assert_eq!(r.videos[0].name, "recording");
        assert_eq!(r.ignored, 5, "5 个 .d.ts 应被计入 ignored");

        let _ = fs::remove_dir_all(&root);
    }

    /// 体积下限应同时滤掉损坏或占位的空视频文件
    #[test]
    fn tiny_files_are_excluded_and_counted() {
        use std::fs;

        let root = std::env::temp_dir().join("process_tiny_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        fs::write(root.join("good.mkv"), vec![0u8; MIN_VIDEO_BYTES as usize + 1]).unwrap();
        fs::write(root.join("empty.mp4"), b"").unwrap();
        fs::write(root.join("stub.avi"), b"RIFFxxxx").unwrap();

        let r = scan_folders(&[root.to_string_lossy().to_string()]);

        assert_eq!(r.videos.len(), 1);
        assert_eq!(r.videos[0].name, "good");
        assert_eq!(r.ignored, 2, "空文件与占位文件都应计入");
        assert_eq!(r.skipped, 0, "体积过小不是 I/O 错误，不该计入 skipped");

        let _ = fs::remove_dir_all(&root);
    }
}