use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::{env, path::PathBuf};

use crate::quota::models::ProviderType;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientLaunchResult {
    pub provider_type: ProviderType,
    pub app_name: String,
    pub launched: bool,
}

#[derive(Debug)]
struct ClientDefinition {
    app_name: &'static str,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    command_names: &'static [&'static str],
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    windows_relative_paths: &'static [&'static str],
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    windows_app_ids: &'static [&'static str],
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    mac_app_names: &'static [&'static str],
}

fn client_definition(provider: &ProviderType) -> Option<ClientDefinition> {
    match provider {
        ProviderType::Codex => Some(ClientDefinition {
            app_name: "Codex",
            // Windows Store 版附带的 codex.exe 是 CLI 辅助程序，不能用来启动桌面窗口。
            command_names: &[],
            windows_relative_paths: &["Programs\\Codex\\Codex.exe"],
            windows_app_ids: &["OpenAI.Codex_2p2nqsd0c76g0!App"],
            mac_app_names: &["Codex"],
        }),
        ProviderType::Cursor => Some(ClientDefinition {
            app_name: "Cursor",
            command_names: &["cursor", "Cursor"],
            windows_relative_paths: &[
                "Programs\\cursor\\Cursor.exe",
                "Programs\\Cursor\\Cursor.exe",
            ],
            windows_app_ids: &[],
            mac_app_names: &["Cursor"],
        }),
        ProviderType::Qcode => Some(ClientDefinition {
            app_name: "Qoder",
            command_names: &["qoder", "Qoder"],
            windows_relative_paths: &["Programs\\Qoder\\Qoder.exe", "Programs\\qoder\\Qoder.exe"],
            windows_app_ids: &[],
            mac_app_names: &["Qoder"],
        }),
        ProviderType::Zcode => Some(ClientDefinition {
            app_name: "ZCode",
            command_names: &["zcode", "ZCode"],
            windows_relative_paths: &["Programs\\ZCode\\ZCode.exe"],
            windows_app_ids: &[],
            mac_app_names: &["ZCode"],
        }),
        ProviderType::Trae => Some(ClientDefinition {
            app_name: "Trae",
            command_names: &["trae", "Trae"],
            windows_relative_paths: &[
                "Programs\\Trae\\Trae.exe",
                "Programs\\TRAE\\TRAE.exe",
                "Programs\\Trae CN\\Trae.exe",
            ],
            windows_app_ids: &[],
            mac_app_names: &["Trae", "TRAE"],
        }),
        ProviderType::Workbuddy => Some(ClientDefinition {
            app_name: "WorkBuddy",
            command_names: &["workbuddy", "WorkBuddy"],
            windows_relative_paths: &[
                "Programs\\WorkBuddy\\WorkBuddy.exe",
                "Programs\\workbuddy\\WorkBuddy.exe",
            ],
            windows_app_ids: &[],
            mac_app_names: &["WorkBuddy"],
        }),
        ProviderType::Gemini => Some(ClientDefinition {
            app_name: "Antigravity",
            command_names: &["antigravity", "Antigravity"],
            windows_relative_paths: &[
                "Programs\\Antigravity\\Antigravity.exe",
                "Programs\\antigravity\\Antigravity.exe",
            ],
            windows_app_ids: &[],
            mac_app_names: &["Antigravity"],
        }),
        _ => None,
    }
}

#[tauri::command]
pub fn launch_ai_client(provider_type: ProviderType) -> Result<ClientLaunchResult, String> {
    let definition = client_definition(&provider_type).ok_or_else(|| {
        format!(
            "{} 没有可启动的本地桌面客户端",
            provider_type.display_name()
        )
    })?;

    launch_definition(&definition)?;
    Ok(ClientLaunchResult {
        provider_type,
        app_name: definition.app_name.to_string(),
        launched: true,
    })
}

#[cfg(target_os = "windows")]
fn launch_definition(definition: &ClientDefinition) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    for app_id in definition.windows_app_ids {
        if Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{app_id}"))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
    }

    if let Some(executable) = find_windows_executable(definition) {
        return Command::new(&executable)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("启动 {} 失败: {error}", definition.app_name));
    }

    Err(format!(
        "未找到 {}，请先安装客户端或从系统开始菜单启动一次",
        definition.app_name
    ))
}

#[cfg(target_os = "windows")]
fn find_windows_executable(definition: &ClientDefinition) -> Option<PathBuf> {
    if let Some(base) = env::var_os("LOCALAPPDATA") {
        for relative in definition.windows_relative_paths {
            let candidate = PathBuf::from(&base).join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    for base in [
        env::var_os("ProgramFiles"),
        env::var_os("ProgramFiles(x86)"),
    ]
    .into_iter()
    .flatten()
    {
        for command_name in definition.command_names {
            for candidate in [
                PathBuf::from(&base)
                    .join(definition.app_name)
                    .join(format!("{command_name}.exe")),
                PathBuf::from(&base).join(format!("{command_name}.exe")),
            ] {
                if candidate.is_file() && is_allowed_executable(&candidate, definition) {
                    return Some(candidate);
                }
            }
        }
    }

    for command_name in definition.command_names {
        let output = Command::new("where.exe").arg(command_name).output().ok()?;
        if output.status.success() {
            if let Some(candidate) = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .map(PathBuf::from)
                .find(|path| path.is_file() && is_allowed_executable(path, definition))
            {
                return Some(candidate);
            }
        }
    }

    find_windows_registry_executable(definition)
}

#[cfg(target_os = "windows")]
fn find_windows_registry_executable(definition: &ClientDefinition) -> Option<PathBuf> {
    const ROOTS: [&str; 3] = [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];
    for root in ROOTS {
        let output = Command::new("reg")
            .args(["query", root, "/s", "/f", definition.app_name, "/d"])
            .output()
            .ok()?;
        if !output.status.success() {
            continue;
        }
        for key in String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("HKEY_"))
        {
            if let Some(path) = read_registry_executable(key, definition) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn read_registry_executable(key: &str, definition: &ClientDefinition) -> Option<PathBuf> {
    for value_name in ["DisplayIcon", "InstallLocation"] {
        let output = Command::new("reg")
            .args(["query", key, "/v", value_name])
            .output()
            .ok()?;
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let Some(value) = text
            .lines()
            .find_map(|line| line.split_once("REG_SZ").map(|(_, value)| value.trim()))
        else {
            continue;
        };
        let cleaned = value.trim_matches('"').split(',').next()?.trim_matches('"');
        let path = PathBuf::from(cleaned);
        if path.is_file() && is_allowed_executable(&path, definition) {
            return Some(path);
        }
        let install_directory = if path.is_dir() {
            Some(path.as_path())
        } else {
            path.parent()
        };
        if let Some(install_directory) = install_directory {
            for executable_name in definition.command_names {
                let candidate = install_directory.join(format!("{executable_name}.exe"));
                if candidate.is_file() && is_allowed_executable(&candidate, definition) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn is_allowed_executable(path: &std::path::Path, definition: &ClientDefinition) -> bool {
    let Some(file_name) = path.file_stem().and_then(|name| name.to_str()) else {
        return false;
    };
    definition
        .command_names
        .iter()
        .any(|allowed| file_name.eq_ignore_ascii_case(allowed))
}

#[cfg(target_os = "macos")]
fn launch_definition(definition: &ClientDefinition) -> Result<(), String> {
    for app_name in definition.mac_app_names {
        match Command::new("open").args(["-a", app_name]).status() {
            Ok(status) if status.success() => return Ok(()),
            _ => continue,
        }
    }
    Err(format!(
        "未找到 {}，请确认应用已安装到 Applications",
        definition.app_name
    ))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn launch_definition(definition: &ClientDefinition) -> Result<(), String> {
    for command_name in definition.command_names {
        if Command::new(command_name).spawn().is_ok() {
            return Ok(());
        }
    }
    Err(format!(
        "未在 PATH 中找到 {}，请确认客户端已安装",
        definition.app_name
    ))
}

#[cfg(test)]
mod tests {
    use super::client_definition;
    use crate::quota::models::ProviderType;

    #[test]
    fn exposes_only_supported_desktop_clients() {
        assert_eq!(
            client_definition(&ProviderType::Codex).map(|definition| definition.app_name),
            Some("Codex")
        );
        assert_eq!(
            client_definition(&ProviderType::Cursor).map(|definition| definition.app_name),
            Some("Cursor")
        );
        assert_eq!(
            client_definition(&ProviderType::Gemini).map(|definition| definition.app_name),
            Some("Antigravity")
        );
        assert!(client_definition(&ProviderType::Deepseek).is_none());
        assert!(client_definition(&ProviderType::Custom).is_none());
    }
}
