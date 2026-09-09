use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tauri::State;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Default)]
pub struct AppLaunchState {
    pub initial_file: Mutex<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownFileInfo {
    pub path: String,
    pub name: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownFileMetadata {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified_at: Option<u64>,
}

fn clean_path_string(path: &Path) -> String {
    let raw = path.to_string_lossy().to_string();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

fn is_likely_encrypted_bytes(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    // 1. 亿赛通 / CDG TSGF magic header or other corporate DLP tags
    if bytes.len() >= 4
        && (&bytes[0..4] == b"TSGF" || &bytes[0..4] == b"CDG " || &bytes[0..4] == b"ESAF")
    {
        return true;
    }
    if bytes.starts_with(b"<![CDATA[") && bytes.len() > 30 {
        if bytes[9..25]
            .iter()
            .all(|b| b.is_ascii_hexdigit() || *b == b'=' || *b == b'/')
        {
            return true;
        }
    }
    // 2. Binary ciphertext check: Markdown text files shouldn't have null bytes or heavy control characters
    let sample_len = bytes.len().min(512);
    if sample_len > 0 {
        if bytes[..sample_len].contains(&0) {
            return true;
        }
        let binary_ctrl_count = bytes[..sample_len]
            .iter()
            .filter(|&&b| (b < 9 && b != 0) || (b > 13 && b < 32))
            .count();
        if binary_ctrl_count > sample_len / 15 {
            return true;
        }
    }
    false
}

#[cfg(windows)]
fn read_file_via_system_fallback(path: &str) -> Option<String> {
    // 1. Try PowerShell Get-Content -Raw -Encoding UTF8 (powershell.exe is in corporate DLP whitelist)
    let mut ps_cmd = Command::new("powershell");
    ps_cmd.creation_flags(CREATE_NO_WINDOW);
    ps_cmd.env("HALOWAKE_MARKDOWN_READ_PATH", path);
    ps_cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Content -Raw -Encoding UTF8 -LiteralPath $env:HALOWAKE_MARKDOWN_READ_PATH",
    ]);

    if let Ok(out) = ps_cmd.output() {
        if out.status.success() && !out.stdout.is_empty() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            let trimmed = s.trim_start_matches('\u{feff}');
            if !is_likely_encrypted_bytes(trimmed.as_bytes()) {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn read_file_via_system_fallback(_path: &str) -> Option<String> {
    None
}

fn decode_bytes(bytes: &[u8]) -> String {
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            let (cow, _, had_errors) = encoding_rs::GB18030.decode(bytes);
            if had_errors {
                String::from_utf8_lossy(bytes).to_string()
            } else {
                cow.to_string()
            }
        }
    }
}

#[tauri::command]
pub async fn read_markdown_file(file_path: String) -> Result<MarkdownFileInfo, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("文件路径为空".to_string());
    }

    let target = Path::new(trimmed);
    if !target.exists() || !target.is_file() {
        return Err(format!("文件不存在或不是有效文件: {}", trimmed));
    }

    let canonical = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let clean_path = clean_path_string(&canonical);
    let name = canonical
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| trimmed.to_string());

    let (content, final_size) = match fs::read(&canonical) {
        Ok(bytes) => {
            if is_likely_encrypted_bytes(&bytes) {
                if let Some(decrypted) = read_file_via_system_fallback(&clean_path) {
                    let len = decrypted.len() as u64;
                    (decrypted, len)
                } else {
                    return Err(format!(
                        "文件内容疑似被加密，且无法通过 PowerShell 读取解密内容: {}",
                        clean_path
                    ));
                }
            } else {
                let decoded = decode_bytes(&bytes);
                let len = bytes.len() as u64;
                (decoded, len)
            }
        }
        Err(e) => {
            // When direct read fails (e.g. Access Denied / OS Error 5 from DLP driver)
            if let Some(decrypted) = read_file_via_system_fallback(&clean_path) {
                let len = decrypted.len() as u64;
                (decrypted, len)
            } else {
                return Err(format!("读取文件内容失败 ({}): {}", clean_path, e));
            }
        }
    };

    let meta = fs::metadata(&canonical).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(final_size);
    let modified_at = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    Ok(MarkdownFileInfo {
        path: clean_path,
        name,
        content,
        size,
        modified_at,
    })
}

#[tauri::command]
pub async fn get_markdown_file_metadata(file_path: String) -> Result<MarkdownFileMetadata, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("文件路径为空".to_string());
    }

    let target = Path::new(trimmed);
    if !target.exists() || !target.is_file() {
        return Err(format!("文件不存在: {}", trimmed));
    }

    let canonical = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let clean_path = clean_path_string(&canonical);
    let name = canonical
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| trimmed.to_string());

    let meta = fs::metadata(&canonical)
        .map_err(|e| format!("获取文件元信息失败 ({}): {}", clean_path, e))?;

    let size = meta.len();
    let modified_at = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    Ok(MarkdownFileMetadata {
        path: clean_path,
        name,
        size,
        modified_at,
    })
}

#[tauri::command]
pub fn get_initial_markdown_file(state: State<'_, AppLaunchState>) -> Option<String> {
    state
        .initial_file
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

#[tauri::command]
pub async fn open_markdown_in_editor(file_path: String) -> Result<(), String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("文件路径为空".to_string());
    }

    let target = Path::new(trimmed);
    if !target.exists() {
        return Err(format!("文件不存在: {}", trimmed));
    }

    let mut command = if cfg!(target_os = "windows") {
        Command::new("explorer")
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };

    command
        .arg(target)
        .spawn()
        .map_err(|e| format!("启动外部编辑器失败 ({}): {}", trimmed, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_read_markdown_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_halowake_preview.md");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "# Halowake Markdown Test\n\nHello world").unwrap();

        let res = read_markdown_file(file_path.to_string_lossy().to_string())
            .await
            .unwrap();
        assert_eq!(res.name, "test_halowake_preview.md");
        assert!(res.content.contains("# Halowake Markdown Test"));
        assert!(res.size > 0);
        assert!(res.modified_at.is_some());

        let meta = get_markdown_file_metadata(file_path.to_string_lossy().to_string())
            .await
            .unwrap();
        assert_eq!(meta.name, "test_halowake_preview.md");
        assert_eq!(meta.size, res.size);

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn detects_encrypted_content_before_using_system_fallback() {
        assert!(!is_likely_encrypted_bytes(b"# Plain Markdown\n\nHello"));
        assert!(is_likely_encrypted_bytes(b"TSGF encrypted payload"));
        assert!(is_likely_encrypted_bytes(b"plain\0binary payload"));
    }
}
