use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// 与 winapi 常量一致，硬编码以避免引入 windows crate
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 相对固定盘的候选路径。按盘逐个探测，覆盖装在非 C 盘的情况。
#[cfg(windows)]
const REL_CANDIDATES: &[&str] = &[
    r"Program Files\DAUM\PotPlayer\PotPlayerMini64.exe",
    r"Program Files\DAUM\PotPlayer\PotPlayerMini.exe",
    r"Program Files (x86)\DAUM\PotPlayer\PotPlayerMini64.exe",
    r"Program Files (x86)\DAUM\PotPlayer\PotPlayerMini.exe",
    r"Program Files\PotPlayer\PotPlayerMini64.exe",
    r"DAUM\PotPlayer\PotPlayerMini64.exe",
];

/// 遍历 A-Z 固定盘查找常见安装位置。最多 26 次廉价 exists() 检查。
#[cfg(windows)]
fn from_common_paths() -> Option<String> {
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if !Path::new(&drive).exists() {
            continue;
        }
        for rel in REL_CANDIDATES {
            let candidate = PathBuf::from(&drive).join(rel);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// 定位 PotPlayer：手填配置 > 注册表 potplay 协议 > App Paths > 常见安装目录
#[cfg(windows)]
pub fn locate(configured: Option<&str>) -> Option<String> {
    if let Some(p) = configured {
        let p = p.trim();
        if !p.is_empty() && Path::new(p).is_file() {
            return Some(p.to_string());
        }
    }

    if let Some(p) = from_registry() {
        return Some(p);
    }

    from_common_paths()
}

/// 从注册表解析 PotPlayer 安装路径。
/// potplay 协议的命令行形如 `"C:\...\PotPlayerMini64.exe" "%1"`，需剥掉引号与参数。
#[cfg(windows)]
fn from_registry() -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let candidates: [(winreg::HKEY, &str); 3] = [
        (
            HKEY_CURRENT_USER,
            r"SOFTWARE\Classes\potplay\shell\open\command",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Classes\potplay\shell\open\command",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\PotPlayerMini64.exe",
        ),
    ];

    for (root, sub) in candidates {
        let Ok(key) = RegKey::predef(root).open_subkey(sub) else {
            continue;
        };
        let Ok(raw) = key.get_value::<String, _>("") else {
            continue;
        };
        let exe = parse_command(&raw);
        if !exe.is_empty() && Path::new(&exe).is_file() {
            return Some(exe);
        }
    }
    None
}

/// 从形如 `"C:\path\exe.exe" "%1"` 的命令行里取出可执行文件路径
#[cfg(windows)]
fn parse_command(raw: &str) -> String {
    let mut exe = String::new();
    let mut in_quotes = false;
    for ch in raw.chars() {
        match ch {
            '"' => {
                if in_quotes {
                    break;
                }
                in_quotes = true;
            }
            _ if !in_quotes => {
                if ch == ' ' || ch == '\t' {
                    if !exe.is_empty() {
                        break;
                    }
                } else {
                    exe.push(ch);
                }
            }
            _ => exe.push(ch),
        }
    }
    // 少数安装会写成未加引号的裸路径
    if exe.is_empty() {
        return raw.trim().to_string();
    }
    exe
}

#[cfg(not(windows))]
pub fn locate(_configured: Option<&str>) -> Option<String> {
    None
}

/// Windows 长路径：超过 248 字符时需补 `\\?\` 前缀，否则 CreateProcess 会失败
#[cfg(windows)]
fn long_path(path: &Path) -> PathBuf {
    let s = path.as_os_str().to_string_lossy();
    if s.starts_with(r"\\?\") || s.len() < 248 {
        return path.to_path_buf();
    }
    if let Some(stripped) = s.strip_prefix(r"\\") {
        // UNC 路径
        return PathBuf::from(format!(r"\\?\UNC\{stripped}"));
    }
    PathBuf::from(format!(r"\\?\{s}"))
}

#[cfg(not(windows))]
fn long_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(windows)]
pub fn launch(exe: &str, file: &Path) -> anyhow::Result<()> {
    std::process::Command::new(exe)
        .arg(long_path(file))
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()?;
    // 必须立即 drop Child，否则等待句柄泄漏
    Ok(())
}

#[cfg(not(windows))]
pub fn launch(exe: &str, file: &Path) -> anyhow::Result<()> {
    std::process::Command::new(exe).arg(file).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn parses_quoted_command() {
        assert_eq!(
            parse_command(r#""C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe" "%1""#),
            r"C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe"
        );
    }

    #[cfg(windows)]
    #[test]
    fn parses_unquoted_command() {
        assert_eq!(
            parse_command(r"C:\PP\PotPlayerMini.exe %1"),
            r"C:\PP\PotPlayerMini.exe"
        );
    }

    #[test]
    fn short_paths_untouched() {
        let p = Path::new(r"C:\a.mkv");
        assert_eq!(long_path(p), p.to_path_buf());
    }

    /// 依赖本机是否安装 PotPlayer，默认跳过。
    /// 手动运行以诊断定位结果：cargo test --lib -- --ignored --nocapture
    #[test]
    #[ignore = "依赖本机环境，用于诊断 locate() 结果"]
    fn report_located_potplayer() {
        println!("locate(None) = {:?}", locate(None));
        println!(
            "locate(bogus) = {:?}（无效配置应回落到自动探测）",
            locate(Some(r"D:\nope\PotPlayerMini64.exe"))
        );
    }

    /// 会真的拉起 PotPlayer 窗口，默认跳过。
    /// 手动运行以验证启动链路：cargo test --lib -- --ignored --nocapture launch
    #[test]
    #[ignore = "会启动 GUI 程序，用于诊断 launch() 链路"]
    fn report_launch_smoke() {
        use std::fs;
        let exe = locate(None).expect("本机应已安装 PotPlayer");
        let dir = std::env::temp_dir().join("process_launch_test");
        let _ = fs::create_dir_all(&dir);
        let video = dir.join("launch_smoke_test.mkv");
        fs::write(&video, b"not a real video").unwrap();

        println!("launching {} with {}", exe, video.display());
        launch(&exe, &video).expect("launch 应成功");
        println!("launch() 返回 Ok，进程句柄已释放");
        let _ = fs::remove_dir_all(&dir);
    }
}