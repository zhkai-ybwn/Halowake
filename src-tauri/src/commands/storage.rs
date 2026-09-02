use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const STORAGE_SETTINGS_FILE: &str = "storage-settings.json";
const DEFAULT_RETENTION_DAYS: u32 = 90;
const CLEANUP_INTERVAL_MILLIS: u64 = 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettings {
    pub auto_cleanup_enabled: bool,
    pub retention_days: u32,
    pub last_cleanup_at: Option<u64>,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            auto_cleanup_enabled: true,
            retention_days: DEFAULT_RETENTION_DAYS,
            last_cleanup_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageOverview {
    pub configuration_bytes: u64,
    pub data_bytes: u64,
    pub cache_bytes: u64,
    pub log_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupResult {
    pub performed: bool,
    pub reclaimed_bytes: u64,
    pub deleted_files: u64,
    pub deleted_records: u64,
    pub completed_at: Option<u64>,
}

fn storage_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| format!("解析应用配置目录失败: {error}"))?
        .join(STORAGE_SETTINGS_FILE))
}

fn normalize_settings(mut settings: StorageSettings) -> StorageSettings {
    settings.retention_days = settings.retention_days.clamp(1, 3_650);
    settings
}

fn read_storage_settings(app: &AppHandle) -> Result<StorageSettings, String> {
    let path = storage_settings_path(app)?;
    if !path.exists() {
        return Ok(StorageSettings::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| format!("读取存储设置失败 {}: {error}", path.display()))?;
    let settings = serde_json::from_str::<StorageSettings>(&content)
        .map_err(|error| format!("存储设置文件不是合法 JSON {}: {error}", path.display()))?;
    Ok(normalize_settings(settings))
}

pub(crate) fn current_retention_days(app: &AppHandle) -> u32 {
    read_storage_settings(app)
        .map(|settings| settings.retention_days)
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}

fn write_storage_settings(app: &AppHandle, settings: &StorageSettings) -> Result<(), String> {
    let path = storage_settings_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析存储设置目录: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("创建存储设置目录失败 {}: {error}", parent.display()))?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("序列化存储设置失败: {error}"))?;
    fs::write(&path, content)
        .map_err(|error| format!("写入存储设置失败 {}: {error}", path.display()))
}

#[tauri::command]
pub fn load_storage_settings(app: AppHandle) -> Result<StorageSettings, String> {
    read_storage_settings(&app)
}

#[tauri::command]
pub fn save_storage_settings(
    app: AppHandle,
    settings: StorageSettings,
) -> Result<StorageSettings, String> {
    let settings = normalize_settings(settings);
    write_storage_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn get_storage_overview(app: AppHandle) -> Result<StorageOverview, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("解析应用配置目录失败: {error}"))?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("解析应用数据目录失败: {error}"))?;
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("解析应用缓存目录失败: {error}"))?;
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|error| format!("解析应用日志目录失败: {error}"))?;

    // Only count Halowake-owned targets. Tauri may map config and data to the same
    // platform directory, so scanning their roots would double-count data.
    let configuration_bytes = ["ai-settings.json", STORAGE_SETTINGS_FILE]
        .iter()
        .map(|name| path_size(&config_dir.join(name)))
        .sum();
    let database_path = data_dir.join("lumina.db");
    let data_bytes = path_size(&database_path)
        + path_size(&data_dir.join("lumina.db-wal"))
        + path_size(&data_dir.join("lumina.db-shm"))
        + path_size(&data_dir.join("data"));
    let cache_bytes = path_size(&cache_dir.join("lumina"));
    let log_bytes = path_size(&log_dir);
    let total_bytes = configuration_bytes + data_bytes + cache_bytes + log_bytes;

    Ok(StorageOverview {
        configuration_bytes,
        data_bytes,
        cache_bytes,
        log_bytes,
        total_bytes,
    })
}

#[tauri::command]
pub fn run_storage_cleanup(app: AppHandle, force: bool) -> Result<StorageCleanupResult, String> {
    let mut settings = read_storage_settings(&app)?;
    let now = now_millis()?;

    if !force && !settings.auto_cleanup_enabled {
        return Ok(skipped_cleanup());
    }
    if !force
        && settings
            .last_cleanup_at
            .is_some_and(|last| now.saturating_sub(last) < CLEANUP_INTERVAL_MILLIS)
    {
        return Ok(skipped_cleanup());
    }

    let retention_millis = u64::from(settings.retention_days) * 24 * 60 * 60 * 1_000;
    let cutoff = now.saturating_sub(retention_millis);
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("解析应用缓存目录失败: {error}"))?
        .join("lumina");
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|error| format!("解析应用日志目录失败: {error}"))?;

    let (cache_bytes, cache_files) = cleanup_expired_files(&cache_dir, cutoff)?;
    let (log_bytes, log_files) = cleanup_expired_files(&log_dir, cutoff)?;
    let database = app.state::<crate::storage::AppDatabase>();
    let deleted_review_records = crate::review::repository::delete_expired_sessions(&database, now as i64)?;
    let deleted_git_records = crate::storage::history_repository::delete_expired_git_commit_history(&database, now as i64, cutoff as i64)?;
    let deleted_devdock_records = crate::storage::history_repository::delete_expired_devdock_run_history(&database, now as i64, cutoff as i64)?;
    let deleted_records = deleted_review_records + deleted_git_records + deleted_devdock_records;
    settings.last_cleanup_at = Some(now);
    write_storage_settings(&app, &settings)?;

    Ok(StorageCleanupResult {
        performed: true,
        reclaimed_bytes: cache_bytes + log_bytes,
        deleted_files: cache_files + log_files,
        deleted_records,
        completed_at: Some(now),
    })
}

fn skipped_cleanup() -> StorageCleanupResult {
    StorageCleanupResult {
        performed: false,
        reclaimed_bytes: 0,
        deleted_files: 0,
        deleted_records: 0,
        completed_at: None,
    }
}

fn now_millis() -> Result<u64, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("系统时间早于 Unix Epoch: {error}"))?
        .as_millis();
    u64::try_from(millis).map_err(|_| "系统时间超出支持范围。".to_string())
}

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| path_size(&entry.path()))
        .sum()
}

fn cleanup_expired_files(root: &Path, cutoff_millis: u64) -> Result<(u64, u64), String> {
    if !root.exists() {
        return Ok((0, 0));
    }
    cleanup_directory(root, cutoff_millis, true)
}

fn cleanup_directory(
    directory: &Path,
    cutoff_millis: u64,
    keep_root: bool,
) -> Result<(u64, u64), String> {
    let mut reclaimed_bytes = 0;
    let mut deleted_files = 0;
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("读取清理目录失败 {}: {error}", directory.display()))?;

    for entry in entries {
        let entry = entry.map_err(|error| format!("读取清理目录项失败: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("读取文件信息失败 {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let (bytes, files) = cleanup_directory(&path, cutoff_millis, false)?;
            reclaimed_bytes += bytes;
            deleted_files += files;
            continue;
        }

        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        if modified.is_some_and(|value| value < cutoff_millis) {
            fs::remove_file(&path)
                .map_err(|error| format!("删除过期文件失败 {}: {error}", path.display()))?;
            reclaimed_bytes += metadata.len();
            deleted_files += 1;
        }
    }

    if !keep_root && fs::read_dir(directory).map(|mut entries| entries.next().is_none()).unwrap_or(false) {
        let _ = fs::remove_dir(directory);
    }
    Ok((reclaimed_bytes, deleted_files))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_retention_days_to_supported_range() {
        let low = normalize_settings(StorageSettings {
            retention_days: 0,
            ..StorageSettings::default()
        });
        let high = normalize_settings(StorageSettings {
            retention_days: 99_999,
            ..StorageSettings::default()
        });
        assert_eq!(low.retention_days, 1);
        assert_eq!(high.retention_days, 3_650);
    }
}
