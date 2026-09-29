use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

const MAX_ENTRIES_PER_LOCATION: usize = 200_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStorageLocation {
    kind: &'static str,
    path: String,
    bytes: u64,
    incomplete: bool,
    largest_children: Vec<AgentStorageChild>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStorageChild {
    name: String,
    path: String,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStorageItem {
    id: &'static str,
    label: &'static str,
    total_bytes: u64,
    locations: Vec<AgentStorageLocation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStorageOverview {
    total_bytes: u64,
    system_drive: Option<String>,
    system_drive_bytes: u64,
    agents: Vec<AgentStorageItem>,
    caches: Vec<AgentCacheItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCacheItem {
    key: &'static str,
    agent_label: &'static str,
    path: String,
    bytes: u64,
    incomplete: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCacheCleanupResult {
    reclaimed_bytes: u64,
    skipped_entries: u64,
}

struct Candidate {
    id: &'static str,
    label: &'static str,
    kind: &'static str,
    path: PathBuf,
}

struct CacheCandidate {
    key: &'static str,
    agent_label: &'static str,
    path: PathBuf,
}

fn candidate(
    id: &'static str,
    label: &'static str,
    kind: &'static str,
    path: PathBuf,
) -> Candidate {
    Candidate {
        id,
        label,
        kind,
        path,
    }
}

fn known_locations() -> Vec<Candidate> {
    let Some(home) = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
    else {
        return Vec::new();
    };
    let roaming = env::var_os("APPDATA").map(PathBuf::from);
    let local = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let codex = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    let claude = env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"));
    let mut locations = vec![
        candidate("codex", "Codex", "home", codex),
        candidate("claude", "Claude Code", "home", claude),
        candidate(
            "gemini",
            "Gemini / Antigravity",
            "home",
            home.join(".gemini"),
        ),
        candidate("cursor", "Cursor", "home", home.join(".cursor")),
        candidate("trae", "Trae", "home", home.join(".trae")),
        candidate("trae", "Trae", "home", home.join(".trae-cn")),
        candidate("opencode", "OpenCode", "home", home.join(".opencode")),
        candidate(
            "opencode",
            "OpenCode",
            "data",
            home.join(".local").join("share").join("opencode"),
        ),
        candidate("qoder", "Qoder", "home", home.join(".qoder")),
        candidate("qoder", "Qoder", "home", home.join(".qoder-cli")),
        candidate("windsurf", "Windsurf", "home", home.join(".windsurf")),
        candidate("cline", "Cline", "home", home.join(".cline")),
        candidate("agents", "Shared Agents", "home", home.join(".agents")),
        candidate("agents", "Shared Agents", "home", home.join(".skills")),
    ];
    if let Some(roaming) = roaming {
        locations.extend([
            candidate("cursor", "Cursor", "data", roaming.join("Cursor")),
            candidate("trae", "Trae", "data", roaming.join("Trae")),
            candidate("opencode", "OpenCode", "data", roaming.join("opencode")),
            candidate("windsurf", "Windsurf", "data", roaming.join("Windsurf")),
        ]);
    }
    if let Some(local) = local {
        locations.extend([
            candidate("cursor", "Cursor", "cache", local.join("Cursor")),
            candidate("trae", "Trae", "cache", local.join("Trae")),
            candidate("opencode", "OpenCode", "cache", local.join("opencode")),
            candidate("windsurf", "Windsurf", "cache", local.join("Windsurf")),
        ]);
    }
    #[cfg(target_os = "macos")]
    {
        let support = home.join("Library").join("Application Support");
        let cache = home.join("Library").join("Caches");
        for (id, label, folder) in [
            ("cursor", "Cursor", "Cursor"),
            ("trae", "Trae", "Trae"),
            ("opencode", "OpenCode", "opencode"),
            ("windsurf", "Windsurf", "Windsurf"),
        ] {
            locations.push(candidate(id, label, "data", support.join(folder)));
            locations.push(candidate(id, label, "cache", cache.join(folder)));
        }
    }
    #[cfg(target_os = "linux")]
    {
        let config = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|path| path.is_absolute()).unwrap_or_else(|| home.join(".config"));
        let cache = env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|path| path.is_absolute()).unwrap_or_else(|| home.join(".cache"));
        if let Some(data) = env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|path| path.is_absolute()) {
            locations.push(candidate("opencode", "OpenCode", "data", data.join("opencode")));
        }
        for (id, label, folder) in [
            ("cursor", "Cursor", "Cursor"),
            ("trae", "Trae", "Trae"),
            ("opencode", "OpenCode", "opencode"),
            ("windsurf", "Windsurf", "Windsurf"),
        ] {
            locations.push(candidate(id, label, "data", config.join(folder)));
            locations.push(candidate(id, label, "cache", cache.join(folder)));
        }
    }
    locations
}

fn known_cache_locations() -> Vec<CacheCandidate> {
    let Some(home) = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
    else {
        return Vec::new();
    };
    let codex = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    let claude = env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"));
    let mut caches = vec![
        CacheCandidate {
            key: "codex-cache",
            agent_label: "Codex",
            path: codex.join("cache"),
        },
        CacheCandidate {
            key: "claude-cache",
            agent_label: "Claude Code",
            path: claude.join("cache"),
        },
    ];
    if let Some(roaming) = env::var_os("APPDATA").map(PathBuf::from) {
        for (agent, folder) in [
            ("cursor", "Cursor"),
            ("trae", "Trae"),
            ("windsurf", "Windsurf"),
        ] {
            let label = match agent {
                "cursor" => "Cursor",
                "trae" => "Trae",
                _ => "Windsurf",
            };
            for (suffix, directory) in [
                ("cache", "Cache"),
                ("code-cache", "Code Cache"),
                ("gpu-cache", "GPUCache"),
            ] {
                let key = match (agent, suffix) {
                    ("cursor", "cache") => "cursor-cache",
                    ("cursor", "code-cache") => "cursor-code-cache",
                    ("cursor", _) => "cursor-gpu-cache",
                    ("trae", "cache") => "trae-cache",
                    ("trae", "code-cache") => "trae-code-cache",
                    ("trae", _) => "trae-gpu-cache",
                    (_, "cache") => "windsurf-cache",
                    (_, "code-cache") => "windsurf-code-cache",
                    _ => "windsurf-gpu-cache",
                };
                caches.push(CacheCandidate {
                    key,
                    agent_label: label,
                    path: roaming.join(folder).join(directory),
                });
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let support = home.join("Library").join("Application Support");
        for (label, folder, cache_key) in [
            ("Cursor", "Cursor", "cursor-app-cache"),
            ("Trae", "Trae", "trae-app-cache"),
            ("Windsurf", "Windsurf", "windsurf-app-cache"),
        ] {
            caches.push(CacheCandidate { key: cache_key, agent_label: label, path: support.join(folder).join("Cache") });
        }
        for (label, folder, key) in [
            ("Cursor", "Cursor", "cursor-system-cache"),
            ("Trae", "Trae", "trae-system-cache"),
            ("OpenCode", "opencode", "opencode-system-cache"),
            ("Windsurf", "Windsurf", "windsurf-system-cache"),
        ] {
            caches.push(CacheCandidate { key, agent_label: label, path: home.join("Library").join("Caches").join(folder) });
        }
    }
    #[cfg(target_os = "linux")]
    {
        let cache = env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|path| path.is_absolute()).unwrap_or_else(|| home.join(".cache"));
        for (label, folder, key) in [
            ("Cursor", "Cursor", "cursor-system-cache"),
            ("Trae", "Trae", "trae-system-cache"),
            ("OpenCode", "opencode", "opencode-system-cache"),
            ("Windsurf", "Windsurf", "windsurf-system-cache"),
        ] {
            caches.push(CacheCandidate { key, agent_label: label, path: cache.join(folder) });
        }
    }
    caches
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

fn measure_path(path: &Path) -> (u64, bool, Vec<AgentStorageChild>) {
    let mut stack = vec![(path.to_path_buf(), None::<PathBuf>)];
    let mut bytes = 0u64;
    let mut scanned = 0usize;
    let mut incomplete = false;
    let mut child_bytes: HashMap<PathBuf, u64> = HashMap::new();
    while let Some((current, top_child)) = stack.pop() {
        if scanned >= MAX_ENTRIES_PER_LOCATION {
            incomplete = true;
            break;
        }
        scanned += 1;
        let metadata = match fs::symlink_metadata(&current) {
            Ok(value) => value,
            Err(_) => {
                incomplete = true;
                continue;
            }
        };
        if is_link_or_reparse(&metadata) {
            incomplete = true;
            continue;
        }
        if metadata.is_file() {
            bytes = bytes.saturating_add(metadata.len());
            if let Some(child) = top_child {
                let entry = child_bytes.entry(child).or_default();
                *entry = entry.saturating_add(metadata.len());
            }
        } else if metadata.is_dir() {
            match fs::read_dir(&current) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(value) => {
                                if scanned + stack.len() >= MAX_ENTRIES_PER_LOCATION {
                                    incomplete = true;
                                    break;
                                }
                                let child_path = value.path();
                                let first_child =
                                    top_child.clone().unwrap_or_else(|| child_path.clone());
                                child_bytes.entry(first_child.clone()).or_default();
                                stack.push((child_path, Some(first_child)));
                            }
                            Err(_) => incomplete = true,
                        }
                    }
                }
                Err(_) => incomplete = true,
            }
        }
    }
    let mut largest_children: Vec<_> = child_bytes
        .into_iter()
        .map(|(path, bytes)| AgentStorageChild {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_string_lossy().to_string(),
            bytes,
        })
        .collect();
    largest_children.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    largest_children.truncate(5);
    (bytes, incomplete, largest_children)
}

fn scan_agent_storage_sync() -> AgentStorageOverview {
    let mut agents: Vec<AgentStorageItem> = Vec::new();
    let mut scanned_roots: Vec<PathBuf> = Vec::new();
    let mut seen = HashSet::new();
    let system_drive = env::var("SystemDrive")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let mut system_drive_bytes = 0u64;
    let mut candidates = known_locations();
    candidates.sort_by_key(|item| item.path.components().count());
    for candidate in candidates {
        let Ok(metadata) = fs::symlink_metadata(&candidate.path) else {
            continue;
        };
        if is_link_or_reparse(&metadata) || !metadata.is_dir() {
            continue;
        }
        let Ok(canonical) = fs::canonicalize(&candidate.path) else {
            continue;
        };
        if !seen.insert(canonical.clone())
            || scanned_roots.iter().any(|root| canonical.starts_with(root))
        {
            continue;
        }
        scanned_roots.push(canonical);
        let (bytes, incomplete, largest_children) = measure_path(&candidate.path);
        if system_drive.as_deref().is_some_and(|drive| {
            candidate
                .path
                .to_string_lossy()
                .to_ascii_lowercase()
                .starts_with(&format!("{}\\", drive.to_ascii_lowercase()))
        }) {
            system_drive_bytes = system_drive_bytes.saturating_add(bytes);
        }
        let index = agents.iter().position(|item| item.id == candidate.id);
        let item = if let Some(index) = index {
            &mut agents[index]
        } else {
            agents.push(AgentStorageItem {
                id: candidate.id,
                label: candidate.label,
                total_bytes: 0,
                locations: Vec::new(),
            });
            agents.last_mut().unwrap()
        };
        item.total_bytes = item.total_bytes.saturating_add(bytes);
        item.locations.push(AgentStorageLocation {
            kind: candidate.kind,
            path: candidate.path.to_string_lossy().to_string(),
            bytes,
            incomplete,
            largest_children,
        });
    }
    agents.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
    let caches = known_cache_locations()
        .into_iter()
        .filter_map(|candidate| {
            let metadata = fs::symlink_metadata(&candidate.path).ok()?;
            if is_link_or_reparse(&metadata) || !metadata.is_dir() {
                return None;
            }
            let (bytes, incomplete, _) = measure_path(&candidate.path);
            Some(AgentCacheItem {
                key: candidate.key,
                agent_label: candidate.agent_label,
                path: candidate.path.to_string_lossy().to_string(),
                bytes,
                incomplete,
            })
        })
        .collect();
    AgentStorageOverview {
        total_bytes: agents.iter().map(|agent| agent.total_bytes).sum(),
        system_drive,
        system_drive_bytes,
        agents,
        caches,
    }
}

fn clear_cache_contents(path: &Path, result: &mut AgentCacheCleanupResult) -> Result<(), String> {
    let entries = fs::read_dir(path).map_err(|error| format!("读取缓存目录失败: {error}"))?;
    for entry in entries {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => {
                result.skipped_entries += 1;
                continue;
            }
        };
        let child = entry.path();
        let metadata = match fs::symlink_metadata(&child) {
            Ok(value) => value,
            Err(_) => {
                result.skipped_entries += 1;
                continue;
            }
        };
        if is_link_or_reparse(&metadata) {
            result.skipped_entries += 1;
        } else if metadata.is_dir() {
            let skipped_before = result.skipped_entries;
            if clear_cache_contents(&child, result).is_err() {
                result.skipped_entries += 1;
            }
            if fs::remove_dir(&child).is_err() && result.skipped_entries == skipped_before {
                result.skipped_entries += 1;
            }
        } else if metadata.is_file() {
            match fs::remove_file(&child) {
                Ok(()) => {
                    result.reclaimed_bytes = result.reclaimed_bytes.saturating_add(metadata.len())
                }
                Err(_) => result.skipped_entries += 1,
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_agent_storage() -> Result<AgentStorageOverview, String> {
    tauri::async_runtime::spawn_blocking(scan_agent_storage_sync)
        .await
        .map_err(|error| format!("Agent 空间扫描失败: {error}"))
}

#[tauri::command]
pub async fn clear_agent_cache(key: String) -> Result<AgentCacheCleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let candidate = known_cache_locations()
            .into_iter()
            .find(|item| item.key == key)
            .ok_or_else(|| "未知的 Agent 缓存目录".to_string())?;
        let metadata = fs::symlink_metadata(&candidate.path)
            .map_err(|error| format!("读取缓存目录失败: {error}"))?;
        if is_link_or_reparse(&metadata) || !metadata.is_dir() {
            return Err("缓存目录不是普通文件夹".to_string());
        }
        let mut result = AgentCacheCleanupResult {
            reclaimed_bytes: 0,
            skipped_entries: 0,
        };
        clear_cache_contents(&candidate.path, &mut result)?;
        Ok(result)
    })
    .await
    .map_err(|error| format!("清理 Agent 缓存失败: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_reports_largest_immediate_folders() {
        let root = env::temp_dir().join(format!(
            "lumina-agent-storage-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("large").join("nested")).unwrap();
        fs::create_dir_all(root.join("small")).unwrap();
        fs::write(root.join("large").join("nested").join("data"), [0u8; 10]).unwrap();
        fs::write(root.join("small").join("data"), [0u8; 2]).unwrap();

        let (bytes, incomplete, largest) = measure_path(&root);
        assert_eq!(bytes, 12);
        assert!(!incomplete);
        assert_eq!(largest[0].name, "large");
        assert_eq!(largest[0].bytes, 10);
        assert_eq!(largest[1].name, "small");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clearing_cache_only_removes_files_below_selected_directory() {
        let root = env::temp_dir().join(format!(
            "lumina-agent-cache-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = root.join("Cache");
        fs::create_dir_all(cache.join("nested")).unwrap();
        fs::write(root.join("settings.json"), "keep").unwrap();
        fs::write(cache.join("entry"), "12345").unwrap();
        fs::write(cache.join("nested").join("entry"), "123").unwrap();

        let mut result = AgentCacheCleanupResult {
            reclaimed_bytes: 0,
            skipped_entries: 0,
        };
        clear_cache_contents(&cache, &mut result).unwrap();
        assert_eq!(result.reclaimed_bytes, 8);
        assert_eq!(result.skipped_entries, 0);
        assert!(root.join("settings.json").exists());
        assert!(fs::read_dir(&cache).unwrap().next().is_none());
        let _ = fs::remove_dir_all(root);
    }
}
