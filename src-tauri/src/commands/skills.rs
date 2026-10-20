use std::{
    collections::HashMap,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use ring::digest::{Context as DigestContext, SHA256};
use tauri::ipc::Channel;
use tauri::{Manager, State};
use rusqlite::params;

use crate::git::runner::{
    kill_process_tree, run_command_with_timeout, run_git, ProcessJobGuard,
    DEFAULT_GIT_COMMAND_TIMEOUT, GIT_CLONE_TIMEOUT,
};
use crate::commands::markdown::read_file_bytes_with_dlp_fallback;
use crate::storage::AppDatabase;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceIdentity {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_skill_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub agent_type: String,
    pub agent_label: String,
    pub associated_agents: Vec<String>,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_path: Option<String>,
    pub installed_at: Option<u64>,
    pub is_git_repo: bool,
    pub git_remote_url: Option<String>,
    pub git_branch: Option<String>,
    pub current_commit: Option<String>,
    pub is_dirty: bool,
    pub dirty: Option<bool>,
    pub can_uninstall: bool,
    pub externally_managed: bool,
    pub management: String,
    pub source: SkillSourceIdentity,
    pub has_update: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_error: Option<String>,
    pub latest_commit: Option<String>,
    pub latest_version: Option<String>,
    pub update_summary: Option<String>,
    pub commits_behind: usize,
    pub last_checked_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateResult {
    pub skill_id: String,
    pub success: bool,
    pub message: String,
    pub previous_commit: Option<String>,
    pub new_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

const MANAGED_SOURCE_FILE: &str = ".lumina-skill-source.json";
const OP_LOCK_PREFIX: &str = ".lumina-op-lock.json.";
const STAGE_DIR_PREFIX: &str = ".lumina-stage-";
const BACKUP_DIR_PREFIX: &str = ".lumina-backup-";
const USER_BACKUP_DIR_PREFIX: &str = ".lumina-user-backup-";
const LOCK_MAX_AGE_MS: u64 = 10 * 60 * 1000;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedSkillSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sub_skill_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    canonical_name: Option<String>,
    git_url: String,
    sub_path: String,
    commit: String,
    content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillOpLock {
    pub pid: u32,
    #[serde(default)]
    pub process_start_token: u64,
    pub canonical_name: String,
    pub op: String,
    pub started_at_ms: u64,
}

#[derive(Debug)]
struct SkillOpLockGuard {
    lock_path: PathBuf,
}

impl Drop for SkillOpLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[cfg(windows)]
fn query_process_start_token(pid: u32) -> Option<u64> {
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low_date_time: u32,
        high_date_time: u32,
    }
    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut std::ffi::c_void;
        fn GetProcessTimes(
            process: *mut std::ffi::c_void,
            creation_time: *mut FileTime,
            exit_time: *mut FileTime,
            kernel_time: *mut FileTime,
            user_time: *mut FileTime,
        ) -> i32;
        fn GetExitCodeProcess(process: *mut std::ffi::c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(object: *mut std::ffi::c_void) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut exit_code: u32 = 0;
        let alive = GetExitCodeProcess(handle, &mut exit_code) != 0 && exit_code == STILL_ACTIVE;
        let mut creation = FileTime::default();
        let mut exit = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();
        let has_times = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) != 0;
        let _ = CloseHandle(handle);
        if !alive || !has_times {
            return None;
        }
        Some(((creation.high_date_time as u64) << 32) | (creation.low_date_time as u64))
    }
}

#[cfg(not(windows))]
fn query_process_start_token(pid: u32) -> Option<u64> {
    if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
        if let Some(after_comm) = stat.rfind(')') {
            let fields: Vec<&str> = stat[after_comm + 1..].split_whitespace().collect();
            if let Some(starttime_str) = fields.get(19) {
                if let Ok(token) = starttime_str.parse::<u64>() {
                    return Some(token);
                }
            }
        }
    }
    let ok = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok { Some(1) } else { None }
}

pub(crate) fn is_lock_owner_alive(lock: &SkillOpLock) -> bool {
    let now = current_timestamp_millis();
    if now.saturating_sub(lock.started_at_ms) > LOCK_MAX_AGE_MS {
        return false;
    }
    let Some(actual_token) = query_process_start_token(lock.pid) else {
        return false;
    };
    if lock.process_start_token != 0 && actual_token != 0 {
        return actual_token == lock.process_start_token;
    }
    true
}

fn acquire_skill_op_lock(parent: &Path, canonical_name: &str, op: &str) -> Result<SkillOpLockGuard, String> {
    fs::create_dir_all(parent).map_err(|e| format!("创建技能父目录失败: {e}"))?;
    let lock_path = parent.join(format!("{OP_LOCK_PREFIX}{canonical_name}"));
    let my_pid = std::process::id();
    let my_token = query_process_start_token(my_pid).unwrap_or(1);

    for attempt in 0..2 {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                let payload = SkillOpLock {
                    pid: my_pid,
                    process_start_token: my_token,
                    canonical_name: canonical_name.to_string(),
                    op: op.to_string(),
                    started_at_ms: current_timestamp_millis(),
                };
                let bytes = serde_json::to_vec_pretty(&payload)
                    .map_err(|e| format!("序列化技能操作锁失败: {e}"))?;
                use std::io::Write;
                if let Err(e) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
                    let _ = fs::remove_file(&lock_path);
                    return Err(format!("写入技能操作锁失败: {e}"));
                }
                return Ok(SkillOpLockGuard { lock_path });
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut existing = fs::read_to_string(&lock_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<SkillOpLock>(&s).ok());
                if existing.is_none() {
                    // Wait briefly in case the lock creator is still flushing bytes
                    std::thread::sleep(Duration::from_millis(15));
                    existing = fs::read_to_string(&lock_path)
                        .ok()
                        .and_then(|s| serde_json::from_str::<SkillOpLock>(&s).ok());
                }
                if let Some(lock_info) = existing {
                    if is_lock_owner_alive(&lock_info) {
                        return Err(format!(
                            "技能「{}」正在被另一个进程 (PID {}) 执行 {} 操作，请稍后重试",
                            canonical_name, lock_info.pid, lock_info.op
                        ));
                    }
                }
                if attempt == 0 {
                    let _ = fs::remove_file(&lock_path);
                    continue;
                }
                return Err(format!("无法获取技能「{}」的操作锁", canonical_name));
            }
            Err(err) => {
                return Err(format!("创建技能操作锁失败: {err}"));
            }
        }
    }
    Err(format!("获取技能「{}」操作锁超时", canonical_name))
}

pub(crate) fn validate_canonical_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!("技能标准名称长度无效: '{name}'"));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("..") {
        return Err(format!("技能标准名称包含非法前后缀或路径片段: '{name}'"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!(
            "技能标准名称必须仅包含小写字母、数字和连字符: '{name}'"
        ));
    }
    Ok(())
}

pub(crate) fn normalize_canonical_skill_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("技能名称不能为空".to_string());
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.starts_with('.')
    {
        return Err(format!("技能名称包含非法路径字符: '{trimmed}'"));
    }
    let normalized = trimmed
        .to_ascii_lowercase()
        .replace(['_', ' '], "-")
        .trim_matches('-')
        .to_string();
    validate_canonical_skill_name(&normalized)?;
    Ok(normalized)
}

fn resolve_canonical_skill_name_from_dir(skill_dir: &Path, fallback_id: &str) -> Result<String, String> {
    let skill_md_path = skill_dir.join("SKILL.md");
    if let Some(content) = read_skill_markdown(&skill_md_path) {
        let (parsed_name, _, _) = parse_skill_md(&content);
        if let Some(frontmatter_name) = parsed_name.filter(|s| !s.trim().is_empty()) {
            return normalize_canonical_skill_name(&frontmatter_name);
        }
    }
    normalize_canonical_skill_name(fallback_id)
}

fn normalize_git_url_for_compare(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

/// Validates and normalizes a skill `sub_path`:
/// - Converts `\` to `/`
/// - Trims whitespace, leading `./`, and trailing `/`
/// - Rejects leading `/` or `\`, drive prefixes (`C:`), empty segments, `.`, and `..`
/// - Preserves original case (Git repository paths are case-sensitive)
pub(crate) fn normalize_skill_sub_path(sub: &str) -> Result<String, String> {
    let trimmed = sub.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains(':')
        || trimmed.contains('\0')
    {
        return Err(format!("子技能路径不能是绝对路径或包含盘符: '{trimmed}'"));
    }

    let replaced = trimmed.replace('\\', "/");
    let mut clean = replaced.as_str();
    while let Some(rest) = clean.strip_prefix("./") {
        clean = rest;
    }
    clean = clean.trim_end_matches('/');

    if clean.is_empty() || clean == "." || clean == ".." || clean.starts_with('/') {
        return Err(format!("子技能路径无效: '{trimmed}'"));
    }

    let mut segments = Vec::new();
    for seg in clean.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(format!("子技能路径包含非法目录片段: '{trimmed}'"));
        }
        segments.push(seg);
    }

    Ok(segments.join("/"))
}

fn normalize_sub_path_for_compare(sub: &str) -> Option<String> {
    normalize_skill_sub_path(sub).ok()
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ExistingTargetDisposition {
    Absent,
    SameSourceManaged(ManagedSkillSource),
    SameSourceGit,
}

fn check_existing_target_disposition(
    target_dir: &Path,
    incoming_git_url: &str,
    incoming_sub_path: &str,
) -> Result<ExistingTargetDisposition, String> {
    if !target_dir.exists() {
        return Ok(ExistingTargetDisposition::Absent);
    }
    let incoming_repo = normalize_git_url_for_compare(incoming_git_url);
    let incoming_sub = normalize_skill_sub_path(incoming_sub_path)?;

    if let Some(existing_source) = read_managed_source(target_dir) {
        let existing_repo = normalize_git_url_for_compare(&existing_source.git_url);
        let existing_sub = normalize_sub_path_for_compare(&existing_source.sub_path);
        if !incoming_repo.is_empty()
            && existing_repo == incoming_repo
            && existing_sub.as_deref() == Some(incoming_sub.as_str())
        {
            return Ok(ExistingTargetDisposition::SameSourceManaged(existing_source));
        }
        return Err(format!(
            "同名技能来源冲突：目录 {} 已安装来自 {} ({}) 的同名技能，拒绝使用 {} ({}) 覆盖",
            target_dir.display(),
            existing_source.git_url,
            if existing_source.sub_path.is_empty() { "根目录" } else { &existing_source.sub_path },
            incoming_git_url,
            if incoming_sub.is_empty() { "根目录" } else { &incoming_sub },
        ));
    }

    let git_dir = target_dir.join(".git");
    if git_dir.exists() {
        let existing_remote = read_git_remote_url_fast(&git_dir)
            .or_else(|| {
                run_git(&target_dir.to_string_lossy(), &["config", "--get", "remote.origin.url"])
                    .ok()
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_default();
        if !existing_remote.is_empty()
            && normalize_git_url_for_compare(&existing_remote) == incoming_repo
            && incoming_sub.is_empty()
        {
            return Ok(ExistingTargetDisposition::SameSourceGit);
        }
        return Err(format!(
            "同名技能来源冲突：目录 {} 已存在来自其他 Git 仓库 ({}) 的同名技能，拒绝覆盖",
            target_dir.display(),
            if existing_remote.is_empty() { "未知远端" } else { &existing_remote }
        ));
    }

    Err(format!(
        "同名技能来源冲突：目录 {} 已存在本地非受管同名技能，为保护您的本地文件拒绝覆盖",
        target_dir.display()
    ))
}

/// Simultaneously checks `catalog_id`, `canonical_name`, and source metadata (`git_url` + case-sensitive `sub_path`)
/// across `parent` to identify whether the skill is already installed (including under a legacy
/// `catalog_id`-named directory) or conflicts with a different source's skill of the same canonical name.
fn find_existing_installed_skill_in_parent(
    parent: &Path,
    catalog_id: &str,
    effective_id: &str,
    canonical_name: Option<&str>,
    incoming_git_url: &str,
    incoming_sub_path: &str,
) -> Result<Option<PathBuf>, String> {
    if !parent.exists() || !parent.is_dir() {
        return Ok(None);
    }

    let incoming_repo = normalize_git_url_for_compare(incoming_git_url);
    let incoming_sub = normalize_skill_sub_path(incoming_sub_path)?;
    let normalized_effective = normalize_canonical_skill_name(effective_id)
        .unwrap_or_else(|_| effective_id.trim().to_ascii_lowercase());
    let target_canonical = canonical_name
        .and_then(|c| normalize_canonical_skill_name(c).ok())
        .unwrap_or_else(|| normalized_effective.clone());

    // 1. Check direct target directories first (canonical_name, effective_id, and catalog_id when installing a non-subskill)
    let mut direct_candidates = Vec::new();
    if let Some(canon) = canonical_name.filter(|s| !s.trim().is_empty()) {
        direct_candidates.push(canon.to_string());
    }
    for id_candidate in [effective_id.trim(), normalized_effective.as_str()] {
        if !id_candidate.is_empty()
            && !direct_candidates.iter().any(|c| c.eq_ignore_ascii_case(id_candidate))
        {
            direct_candidates.push(id_candidate.to_string());
        }
    }
    // Only include `catalog_id` as a direct candidate folder if it is not a bundle parent with a distinct `effective_id`
    if !catalog_id.trim().is_empty()
        && catalog_id.trim().eq_ignore_ascii_case(effective_id.trim())
        && !direct_candidates
            .iter()
            .any(|c| c.eq_ignore_ascii_case(catalog_id.trim()))
    {
        direct_candidates.push(catalog_id.trim().to_string());
    }

    for candidate_folder in &direct_candidates {
        let candidate_dir = parent.join(candidate_folder);
        if !candidate_dir.exists() || !candidate_dir.is_dir() {
            continue;
        }
        match check_existing_target_disposition(&candidate_dir, incoming_git_url, &incoming_sub) {
            Ok(ExistingTargetDisposition::Absent) => {}
            Ok(ExistingTargetDisposition::SameSourceManaged(_))
            | Ok(ExistingTargetDisposition::SameSourceGit) => {
                return Ok(Some(candidate_dir));
            }
            Err(conflict_err) => {
                return Err(conflict_err);
            }
        }
    }

    // 2. Scan all non-hidden directories in `parent` to catch legacy `catalog_id`-named directories
    //    whose folder name differs from the incoming `canonical_name` or `effective_id`.
    let entries = fs::read_dir(parent).map_err(|e| format!("读取技能父目录失败: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(folder_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if folder_name.starts_with('.') || folder_name == "_shared" {
            continue;
        }
        if direct_candidates.iter().any(|c| c.eq_ignore_ascii_case(folder_name)) {
            continue;
        }

        let managed_source = read_managed_source(&path);
        let dir_canonical = resolve_canonical_skill_name_from_dir(&path, folder_name).ok();

        if let Some(ref src) = managed_source {
            let existing_repo = normalize_git_url_for_compare(&src.git_url);
            let existing_sub = normalize_sub_path_for_compare(&src.sub_path);

            // Strict same-source match: must have identical `git_url` AND identical case-sensitive `sub_path`
            if !incoming_repo.is_empty()
                && existing_repo == incoming_repo
                && existing_sub.as_deref() == Some(incoming_sub.as_str())
            {
                return Ok(Some(path));
            }

            // Cross-source or cross-subPath conflict against a directory with identical `canonical_name`
            if canonical_name.is_some() && dir_canonical.as_deref() == Some(target_canonical.as_str()) {
                return Err(format!(
                    "同名技能来源冲突：目录 {} 已安装来自 {} ({}) 的同名技能「{}」，拒绝使用 {} ({}) 覆盖",
                    path.display(),
                    src.git_url,
                    if src.sub_path.is_empty() { "根目录" } else { &src.sub_path },
                    target_canonical,
                    incoming_git_url,
                    if incoming_sub.is_empty() { "根目录" } else { &incoming_sub },
                ));
            }
        } else if path.join(".git").exists() {
            let existing_remote = read_git_remote_url_fast(&path.join(".git")).unwrap_or_default();
            if !existing_remote.is_empty() {
                if normalize_git_url_for_compare(&existing_remote) == incoming_repo
                    && incoming_sub.is_empty()
                {
                    return Ok(Some(path));
                }
                if canonical_name.is_some() && dir_canonical.as_deref() == Some(target_canonical.as_str()) {
                    return Err(format!(
                        "同名技能来源冲突：目录 {} 已存在来自其他 Git 仓库 ({}) 的同名技能「{}」，拒绝覆盖",
                        path.display(),
                        existing_remote,
                        target_canonical
                    ));
                }
            }
        } else if canonical_name.is_some()
            && path.join("SKILL.md").is_file()
            && dir_canonical.as_deref() == Some(target_canonical.as_str())
            && folder_name.eq_ignore_ascii_case(&target_canonical)
        {
            return Err(format!(
                "同名技能来源冲突：目录 {} 已存在本地非受管同名技能，为保护您的本地文件拒绝覆盖",
                path.display()
            ));
        }
    }

    Ok(None)
}

/// Scans `parent` for interrupted transactional skill operations (`.lumina-stage-<name>`,
/// `.lumina-backup-<name>`, `.lumina-op-lock.json.<name>`) and safely rolls back or cleans up
/// any operation whose owning process is no longer alive.
pub(crate) fn recover_interrupted_skill_ops_in_dir(parent: &Path) -> Result<usize, String> {
    if !parent.exists() || !parent.is_dir() {
        return Ok(0);
    }

    let entries: Vec<PathBuf> = fs::read_dir(parent)
        .map_err(|e| format!("读取技能目录失败: {e}"))?
        .filter_map(|e| e.ok().map(|item| item.path()))
        .collect();

    let mut candidate_names = std::collections::BTreeSet::new();
    for path in &entries {
        let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Some(name) = fname.strip_prefix(BACKUP_DIR_PREFIX) {
            if !name.is_empty() {
                candidate_names.insert(name.to_string());
            }
        } else if let Some(name) = fname.strip_prefix(STAGE_DIR_PREFIX) {
            if !name.is_empty() {
                candidate_names.insert(name.to_string());
            }
        } else if let Some(name) = fname.strip_prefix(OP_LOCK_PREFIX) {
            if !name.is_empty() {
                candidate_names.insert(name.to_string());
            }
        }
    }

    let mut actions = 0usize;
    for name in candidate_names {
        let lock_path = parent.join(format!("{OP_LOCK_PREFIX}{name}"));
        let stage_path = parent.join(format!("{STAGE_DIR_PREFIX}{name}"));
        let backup_path = parent.join(format!("{BACKUP_DIR_PREFIX}{name}"));
        let target_path = parent.join(&name);

        if lock_path.exists() {
            if let Ok(text) = fs::read_to_string(&lock_path) {
                if let Ok(lock_info) = serde_json::from_str::<SkillOpLock>(&text) {
                    if is_lock_owner_alive(&lock_info) {
                        // Active process still owns this operation; never interfere!
                        continue;
                    }
                }
            }
        }

        if backup_path.exists() && !target_path.exists() {
            // Interrupted after `target -> backup` and before `stage -> target`: restore backup!
            fs::rename(&backup_path, &target_path)
                .map_err(|e| format!("恢复技能备份 {} 失败: {e}", name))?;
            if stage_path.exists() {
                remove_dir_all_with_retry(&stage_path);
            }
            actions += 1;
        } else if backup_path.exists() && target_path.exists() {
            // Interrupted after `stage -> target`: verify target integrity
            let target_valid = target_path.join("SKILL.md").is_file()
                && read_managed_source(&target_path)
                    .map(|src| {
                        hash_skill_files(&target_path)
                            .map(|digest| digest == src.content_sha256)
                            .unwrap_or(false)
                    })
                    .unwrap_or(true);
            if target_valid {
                remove_dir_all_with_retry(&backup_path);
                if stage_path.exists() {
                    remove_dir_all_with_retry(&stage_path);
                }
                actions += 1;
            } else {
                remove_dir_all_with_retry(&target_path);
                fs::rename(&backup_path, &target_path)
                    .map_err(|e| format!("回滚损坏的技能目录 {} 失败: {e}", name))?;
                if stage_path.exists() {
                    remove_dir_all_with_retry(&stage_path);
                }
                actions += 1;
            }
        } else if stage_path.exists() {
            // Interrupted during staging before backup was created: remove stage, keep target untouched
            remove_dir_all_with_retry(&stage_path);
            actions += 1;
        }

        if lock_path.exists() {
            let _ = fs::remove_file(&lock_path);
            actions += 1;
        }
    }

    Ok(actions)
}

fn read_managed_source(path: &Path) -> Option<ManagedSkillSource> {
    let text = fs::read_to_string(path.join(MANAGED_SOURCE_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

fn read_skill_markdown(path: &Path) -> Option<String> {
    String::from_utf8(read_file_bytes_with_dlp_fallback(path).ok()?).ok()
}

fn hash_skill_files(root: &Path) -> Result<String, String> {
    fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|error| format!("读取技能目录失败: {error}"))? {
            let entry = entry.map_err(|error| format!("读取技能文件失败: {error}"))?;
            let ty = entry.file_type().map_err(|error| format!("检查技能文件失败: {error}"))?;
            if ty.is_symlink() { return Err("技能目录包含符号链接".to_string()); }
            if ty.is_dir() {
                if entry.file_name() != ".git" {
                    collect(&entry.path(), files)?;
                }
            } else if entry.file_name() != MANAGED_SOURCE_FILE {
                files.push(entry.path());
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    collect(root, &mut files)?;
    files.sort();
    let mut context = DigestContext::new(&SHA256);
    for file in files {
        let relative = file.strip_prefix(root).map_err(|error| format!("技能路径无效: {error}"))?;
        context.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        context.update(&[0]);
        context.update(&read_file_bytes_with_dlp_fallback(&file)?);
        context.update(&[0]);
    }
    Ok(context.finish().as_ref().iter().map(|byte| format!("{byte:02x}")).collect())
}

fn get_home_dir() -> Result<PathBuf, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|_| "无法定位当前用户目录".to_string())?;
    Ok(PathBuf::from(home))
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 判断 YAML 值是否为折叠/字面块标量引导符（如 >, >-, >+, |, |-, |+ 或空引导）
fn is_yaml_block_scalar_indicator(val: &str) -> bool {
    let clean = val.split('#').next().unwrap_or("").trim();
    if clean.is_empty() {
        return true;
    }
    clean.starts_with('>') || clean.starts_with('|')
}

/// 解析 SKILL.md 中的 YAML frontmatter
/// 返回: (name, description, version)
pub(crate) fn parse_skill_md(content: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut lines = content.lines();
    // 检查第一行是否为 ---
    if let Some(first_line) = lines.next() {
        if first_line.trim() != "---" {
            return (None, None, None);
        }
    } else {
        return (None, None, None);
    }

    let mut name = None;
    let mut description = None;
    let mut version = None;
    let mut in_description = false;
    let mut desc_buf = String::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }

        if in_description {
            if !line.starts_with(' ') && !line.starts_with('\t') && line.contains(':') {
                in_description = false;
                if !desc_buf.is_empty() {
                    description = Some(desc_buf.trim().to_string());
                    desc_buf.clear();
                }
            } else {
                if trimmed.is_empty() {
                    continue;
                }
                if !desc_buf.is_empty() {
                    desc_buf.push(' ');
                }
                desc_buf.push_str(trimmed);
                continue;
            }
        }

        let trimmed_lower = trimmed.to_lowercase();
        if trimmed_lower.starts_with("name:") {
            if let Some((_, rest)) = trimmed.split_once(':') {
                name = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        } else if trimmed_lower.starts_with("version:") {
            if let Some((_, rest)) = trimmed.split_once(':') {
                version = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        } else if trimmed_lower.starts_with("description:") {
            if let Some((_, rest)) = trimmed.split_once(':') {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if is_yaml_block_scalar_indicator(val) {
                    in_description = true;
                } else {
                    description = Some(val.to_string());
                }
            }
        }
    }

    if in_description && !desc_buf.is_empty() && description.is_none() {
        description = Some(desc_buf.trim().to_string());
    }

    // 终极防御校验：如果描述仅剩下 YAML 标量引导字符（如 >- 或 | 等），置为 None
    if let Some(ref d) = description {
        let dt = d.trim();
        if dt == ">-" || dt == ">" || dt == "|-" || dt == "|" || dt == ">+" || dt == "|+" {
            description = None;
        }
    }

    (name, description, version)
}

struct AgentTarget {
    agent_type: &'static str,
    agent_label: &'static str,
    dir_path: PathBuf,
}

fn is_uninstallable_root(path: &Path) -> bool {
    matches!(path.file_name().and_then(|name| name.to_str()), Some("skills" | "skills-cursor" | ".skills"))
}

fn skills_cli_lock_contains(lock_path: &Path, folder_name: &str) -> bool {
    let Ok(content) = fs::read_to_string(lock_path) else {
        return lock_path.exists();
    };
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(&content) else {
        return true;
    };
    lock.get("skills")
        .and_then(|skills| skills.as_object())
        .is_some_and(|skills| {
            skills.keys().any(|name| {
                let sanitized = name
                    .to_ascii_lowercase()
                    .chars()
                    .map(|character| {
                        if character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '.'
                            || character == '_'
                        {
                            character
                        } else {
                            '-'
                        }
                    })
                    .collect::<String>();
                let sanitized = sanitized
                    .split('-')
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("-");
                name == folder_name || sanitized.trim_matches(&['.', '-'][..]) == folder_name
            })
        })
}

fn is_skills_cli_managed(path: &Path) -> bool {
    let Some(skill_root) = path.parent() else {
        return false;
    };
    let Some(agents_dir) = skill_root.parent() else {
        return false;
    };
    if skill_root.file_name().and_then(|name| name.to_str()) != Some("skills")
        || agents_dir.file_name().and_then(|name| name.to_str()) != Some(".agents")
    {
        return false;
    }
    let Some(folder_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let mut locks = vec![agents_dir.join(".skill-lock.json")];
    if let Some(project_root) = agents_dir.parent() {
        locks.push(project_root.join("skills-lock.json"));
    }
    if get_home_dir()
        .ok()
        .is_some_and(|home| agents_dir == home.join(".agents"))
    {
        if let Some(xdg_state) = env::var_os("XDG_STATE_HOME") {
            locks.push(PathBuf::from(xdg_state).join("skills").join(".skill-lock.json"));
        }
    }
    locks
        .iter()
        .any(|lock| skills_cli_lock_contains(lock, folder_name))
}

fn is_link_or_reparse(path: &Path) -> Result<bool, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("读取目录信息失败 {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_no_links_in_tree(path: &Path) -> Result<(), String> {
    if is_link_or_reparse(path)? {
        return Err("技能目录包含链接或重解析点，无法安全卸载".to_string());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|error| format!("读取技能目录失败: {error}"))? {
            let entry = entry.map_err(|error| format!("读取技能目录项失败: {error}"))?;
            ensure_no_links_in_tree(&entry.path())?;
        }
    }
    Ok(())
}

fn validate_uninstall_target(path: &Path, targets: &[AgentTarget]) -> Result<(), String> {
    if !path.is_dir() || is_link_or_reparse(path)? {
        return Err("技能目录不存在或是链接".to_string());
    }
    if !path.join("SKILL.md").is_file() {
        return Err("目录中没有 SKILL.md，无法作为独立技能卸载".to_string());
    }
    let canonical = fs::canonicalize(path).map_err(|error| format!("解析技能目录失败: {error}"))?;
    let approved = targets.iter().filter(|target| is_uninstallable_root(&target.dir_path)).any(|target| {
        fs::canonicalize(&target.dir_path).ok().is_some_and(|root| canonical.parent() == Some(root.as_path()))
    });
    if !approved {
        return Err("技能目录不在允许卸载的位置".to_string());
    }
    if is_skills_cli_managed(path) {
        return Err("该技能由 skills CLI 管理，请使用原安装器卸载以同步锁文件".to_string());
    }
    ensure_no_links_in_tree(path)
}

fn collect_agent_targets(project_cwd: Option<&str>) -> Vec<AgentTarget> {
    let mut targets = Vec::new();
    let home = match get_home_dir() {
        Ok(h) => h,
        Err(_) => return targets,
    };

    // 0. 通用 Agents 生态 (~/.agents/skills 与 ~/.skills)
    let agents_candidates = [
        home.join(".agents").join("skills"),
        home.join(".skills"),
    ];
    for dir in agents_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "agents",
                agent_label: "Agents",
                dir_path: dir,
            });
        }
    }

    // 1. Codex CLI
    let codex_skills_dir = if let Ok(codex_home) = env::var("CODEX_HOME") {
        PathBuf::from(codex_home).join("skills")
    } else {
        home.join(".codex").join("skills")
    };
    targets.push(AgentTarget {
        agent_type: "codex",
        agent_label: "Codex",
        dir_path: codex_skills_dir,
    });

    // 2. Antigravity / Gemini CLI
    let gemini_skills_dir = home.join(".gemini").join("config").join("skills");
    targets.push(AgentTarget {
        agent_type: "antigravity",
        agent_label: "Antigravity",
        dir_path: gemini_skills_dir,
    });

    // 3. Cursor
    let cursor_candidates = [
        home.join(".cursor").join("skills-cursor"),
        home.join(".cursor").join("skills"),
        home.join(".cursor").join("rules"),
    ];
    for dir in cursor_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "cursor",
                agent_label: "Cursor",
                dir_path: dir,
            });
        }
    }

    // 4. Trae (字节跳动)
    let trae_candidates = [
        home.join(".trae").join("builtin_skills"),
        home.join(".trae").join("skills"),
        home.join(".trae-cn").join("skills"),
        home.join(".trae-aicc").join("skills"),
    ];
    for dir in trae_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "trae",
                agent_label: "Trae",
                dir_path: dir,
            });
        }
    }

    // 5. Claude Code
    let claude_candidates = [
        home.join(".claude").join("skills"),
        home.join(".claude").join("plugins"),
        home.join(".claude").join("commands"),
    ];
    for dir in claude_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "claude",
                agent_label: "Claude Code",
                dir_path: dir,
            });
        }
    }

    // 6. Qoder / 阿里灵码
    let qoder_candidates = [
        home.join(".qoder-cli").join("skills"),
        home.join(".qoder-cn").join("skills"),
        home.join(".qoder").join("skills"),
        home.join(".lingma").join("skills"),
    ];
    for dir in qoder_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "qoder",
                agent_label: "Qoder (灵码)",
                dir_path: dir,
            });
        }
    }

    // 7. OpenCode
    let opencode_candidates = [
        home.join(".opencode").join("skills"),
        home.join(".opencode").join("tools"),
        home.join(".local").join("share").join("opencode").join("skills"),
    ];
    for dir in opencode_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "opencode",
                agent_label: "OpenCode",
                dir_path: dir,
            });
        }
    }

    // 8. Windsurf (Codeium)
    let windsurf_candidates = [
        home.join(".windsurf").join("skills"),
        home.join(".codeium").join("windsurf").join("skills"),
    ];
    for dir in windsurf_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "windsurf",
                agent_label: "Windsurf",
                dir_path: dir,
            });
        }
    }

    // 9. Cline / Roo Code
    let cline_candidates = [
        home.join(".cline").join("skills"),
        home.join(".roomodes"),
    ];
    for dir in cline_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "cline",
                agent_label: "Cline",
                dir_path: dir,
            });
        }
    }

    // 10. 项目工作区目录
    if let Some(cwd) = project_cwd {
        let p = PathBuf::from(cwd);
        if p.exists() {
            let project_candidates = [
                p.join(".agents").join("skills"),
                p.join(".skills"),
                p.join(".claude").join("skills"),
                p.join(".gemini").join("skills"),
                p.join(".cursor").join("rules"),
                p.join(".trae").join("rules"),
            ];
            for dir in project_candidates {
                if dir.exists() {
                    targets.push(AgentTarget {
                        agent_type: "project",
                        agent_label: "项目专属",
                        dir_path: dir,
                    });
                }
            }
        }
    }

    targets
}

fn read_git_remote_url_fast(git_dir: &Path) -> Option<String> {
    let content = fs::read_to_string(git_dir.join("config")).ok()?;
    let mut in_remote_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_remote_origin = trimmed.eq_ignore_ascii_case("[remote \"origin\"]");
            continue;
        }
        if in_remote_origin {
            if let Some(rest) = trimmed.strip_prefix("url") {
                let rest = rest.trim();
                if let Some(url_part) = rest.strip_prefix('=') {
                    let url = url_part.trim();
                    if !url.is_empty() {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }
    None
}

fn read_commit_from_packed_refs(git_dir: &Path, target_ref: &str) -> Option<String> {
    let packed = fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    for line in packed.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('^') || trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let hash = parts.next()?;
        let ref_name = parts.next()?;
        if ref_name == target_ref {
            return if hash.len() >= 7 {
                Some(hash[..7].to_string())
            } else {
                Some(hash.to_string())
            };
        }
    }
    None
}

fn read_git_head_fast(git_dir: &Path) -> (Option<String>, Option<String>) {
    let head_content = match fs::read_to_string(git_dir.join("HEAD")) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let trimmed = head_content.trim();
    if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
        let branch = branch_ref.trim().to_string();
        let ref_file = git_dir.join("refs").join("heads").join(&branch);
        let commit = if let Ok(ref_content) = fs::read_to_string(ref_file) {
            let hash = ref_content.trim();
            if hash.len() >= 7 {
                Some(hash[..7].to_string())
            } else if !hash.is_empty() {
                Some(hash.to_string())
            } else {
                None
            }
        } else {
            read_commit_from_packed_refs(git_dir, &format!("refs/heads/{}", branch))
        };
        (Some(branch), commit)
    } else if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        (None, Some(trimmed[..7].to_string()))
    } else {
        (None, None)
    }
}

fn scan_single_skill_dir(
    entry_path: &Path,
    agent_type: &str,
    agent_label: &str,
) -> Option<AgentSkill> {
    if !entry_path.is_dir() {
        return None;
    }

    let folder_name = entry_path.file_name()?.to_string_lossy().to_string();

    // 跳过隐藏目录、系统预置目录与备份目录
    if folder_name.starts_with('.')
        || folder_name == "_shared"
        || folder_name == ".system"
        || folder_name == ".git"
        || folder_name == ".backup"
        || folder_name.starts_with(".tmp")
    {
        return None;
    }

    let skill_md_path = entry_path.join("SKILL.md");
    let has_skill_md = skill_md_path.exists();
    let git_dir = entry_path.join(".git");
    let managed_source = read_managed_source(entry_path);
    let has_managed_source = managed_source.is_some();
    let is_git_repo = git_dir.exists() || has_managed_source;

    // 既没有 SKILL.md 也不是 git 仓库，跳过
    if !has_skill_md && !is_git_repo {
        return None;
    }

    let mut name = folder_name.clone();
    let mut description = String::new();
    let mut version = None;

    if has_skill_md {
        if let Some(content) = read_skill_markdown(&skill_md_path) {
            let (parsed_name, parsed_desc, parsed_ver) = parse_skill_md(&content);
            if let Some(n) = parsed_name {
                if !n.trim().is_empty() {
                    name = n;
                }
            }
            if let Some(d) = parsed_desc {
                description = d;
            }
            version = parsed_ver;
        }
    }

    // Some installed SKILL.md files are unreadable to this process. The source
    // metadata remains available, so retain the skill's actual directory name
    // instead of showing the generated catalog ID.
    if name == folder_name {
        if let Some(source) = managed_source.as_ref() {
            if let Some(canon) = source.canonical_name.as_deref().filter(|s| !s.trim().is_empty()) {
                name = canon.to_string();
            } else if let Some(source_name) = source.sub_path.replace('\\', "/").rsplit('/').find(|part| !part.is_empty()) {
                name = source_name.to_string();
            }
        }
    }

    let canonical_name = managed_source
        .as_ref()
        .and_then(|s| s.canonical_name.as_deref())
        .and_then(|c| normalize_canonical_skill_name(c).ok())
        .or_else(|| normalize_canonical_skill_name(&name).ok())
        .or_else(|| normalize_canonical_skill_name(&folder_name).ok());

    let path_str = entry_path.to_string_lossy().to_string();
    let installed_at = fs::metadata(entry_path)
        .and_then(|metadata| metadata.created().or_else(|_| metadata.modified()))
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    let mut git_remote_url = None;
    let mut git_branch = None;
    let mut current_commit = None;
    let mut is_dirty = false;
    let mut dirty: Option<bool> = None;

    if git_dir.exists() {
        // Fast-path: read directly from .git/config and .git/HEAD
        git_remote_url = read_git_remote_url_fast(&git_dir);
        let (fast_branch, fast_commit) = read_git_head_fast(&git_dir);
        git_branch = fast_branch;
        current_commit = fast_commit;

        // Fallback to git CLI if fast-path did not resolve
        if git_remote_url.is_none() {
            if let Ok(url) = run_git(&path_str, &["config", "--get", "remote.origin.url"]) {
                let trimmed = url.trim().to_string();
                if !trimmed.is_empty() {
                    git_remote_url = Some(trimmed);
                }
            }
        }

        if git_branch.is_none() {
            if let Ok(branch) = run_git(&path_str, &["rev-parse", "--abbrev-ref", "HEAD"]) {
                let trimmed = branch.trim().to_string();
                if !trimmed.is_empty() {
                    git_branch = Some(trimmed);
                }
            }
        }

        if current_commit.is_none() {
            if let Ok(commit) = run_git(&path_str, &["rev-parse", "--short", "HEAD"]) {
                let trimmed = commit.trim().to_string();
                if !trimmed.is_empty() {
                    current_commit = Some(trimmed);
                }
            }
        }

        if let Ok(status) = run_git(&path_str, &["status", "--porcelain"]) {
            let git_dirty = !status.trim().is_empty();
            is_dirty = git_dirty;
            dirty = Some(git_dirty);
        }
    }

    let mut catalog_id = None;
    let mut sub_skill_id = None;
    let mut sub_path = None;
    let externally_managed = is_skills_cli_managed(entry_path);

    let source_identity = if let Some(source) = managed_source {
        catalog_id = source.catalog_id.filter(|s| !s.trim().is_empty());
        sub_skill_id = source.sub_skill_id.filter(|s| !s.trim().is_empty());
        let normalized_sub = normalize_skill_sub_path(&source.sub_path).unwrap_or_else(|_| source.sub_path.clone());
        sub_path = Some(normalized_sub.clone());
        git_remote_url = Some(source.git_url.clone());
        current_commit = Some(source.commit.chars().take(7).collect());
        let managed_dirty = hash_skill_files(entry_path)
            .map(|digest| digest != source.content_sha256)
            .unwrap_or(true);
        is_dirty = managed_dirty;
        dirty = Some(managed_dirty);
        SkillSourceIdentity {
            kind: "halowake_managed".to_string(),
            git_url: Some(source.git_url),
            sub_path: Some(normalized_sub),
            commit: Some(source.commit),
            content_sha256: Some(source.content_sha256),
        }
    } else if git_dir.exists() && git_remote_url.is_some() {
        sub_path = Some(String::new());
        SkillSourceIdentity {
            kind: "git_remote".to_string(),
            git_url: git_remote_url.clone(),
            sub_path: Some(String::new()),
            commit: current_commit.clone(),
            content_sha256: None,
        }
    } else {
        SkillSourceIdentity {
            kind: "unknown".to_string(),
            git_url: None,
            sub_path: None,
            commit: None,
            content_sha256: None,
        }
    };

    if catalog_id.is_none() && has_managed_source && !folder_name.eq_ignore_ascii_case(&name) {
        catalog_id = Some(folder_name.clone());
    }

    let management = if externally_managed {
        "external".to_string()
    } else if has_managed_source {
        "halowake".to_string()
    } else if git_dir.exists() && git_remote_url.is_some() {
        "git".to_string()
    } else {
        "unmanaged".to_string()
    };

    let id = format!("{}:{}", agent_type, folder_name);

    Some(AgentSkill {
        id,
        catalog_id,
        sub_skill_id,
        canonical_name,
        name,
        description,
        version,
        agent_type: agent_type.to_string(),
        agent_label: agent_label.to_string(),
        associated_agents: vec![agent_type.to_string()],
        path: path_str,
        sub_path,
        installed_at,
        is_git_repo,
        git_remote_url,
        git_branch,
        current_commit,
        is_dirty,
        dirty,
        can_uninstall: false,
        externally_managed,
        management,
        source: source_identity,
        has_update: false,
        update_status: None,
        update_error: None,
        latest_commit: None,
        latest_version: None,
        update_summary: None,
        commits_behind: 0,
        last_checked_at: None,
    })
}

fn scan_agent_skills_sync(project_cwd: Option<&str>) -> Vec<AgentSkill> {
    scan_agent_targets(collect_agent_targets(project_cwd))
}

fn scan_agent_targets(targets: Vec<AgentTarget>) -> Vec<AgentSkill> {
    let mut result: Vec<AgentSkill> = Vec::new();
    let mut path_to_index: HashMap<PathBuf, usize> = HashMap::new();

    for target in targets {
        if !target.dir_path.exists() {
            continue;
        }

        // Automatically self-heal any interrupted skill install/update before scanning
        let _ = recover_interrupted_skill_ops_in_dir(&target.dir_path);

        let entries = match fs::read_dir(&target.dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

            if let Some(&existing_idx) = path_to_index.get(&canonical) {
                // 已存在该物理目录，避免重复条目，同时关联当前 Agent
                if !result[existing_idx]
                    .associated_agents
                    .contains(&target.agent_type.to_string())
                {
                    result[existing_idx]
                        .associated_agents
                        .push(target.agent_type.to_string());
                    if !result[existing_idx].agent_label.contains(target.agent_label) {
                        result[existing_idx].agent_label = format!(
                            "{} / {}",
                            result[existing_idx].agent_label, target.agent_label
                        );
                    }
                }
                continue;
            }

            if let Some(mut skill) =
                scan_single_skill_dir(&path, target.agent_type, target.agent_label)
            {
                skill.externally_managed = is_skills_cli_managed(&path);
                if skill.externally_managed {
                    skill.management = "external".to_string();
                }
                skill.can_uninstall = is_uninstallable_root(&target.dir_path)
                    && path.join("SKILL.md").is_file()
                    && !is_link_or_reparse(&path).unwrap_or(true)
                    && !skill.externally_managed;
                let idx = result.len();
                result.push(skill);
                path_to_index.insert(canonical, idx);
            }
        }
    }

    result
}

#[tauri::command]
pub async fn scan_agent_skills(project_cwd: Option<String>) -> Result<Vec<AgentSkill>, String> {
    tokio::task::spawn_blocking(move || scan_agent_skills_sync(project_cwd.as_deref()))
        .await
        .map_err(|e| format!("扫描技能失败: {}", e))
}

fn parse_version_parts(v: &str) -> Vec<u64> {
    let clean = v.trim().trim_start_matches(|c: char| c == 'v' || c == 'V');
    let core = clean.split(['-', '+']).next().unwrap_or(clean);
    core.split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

pub(crate) fn is_newer_semver(latest: &str, current: &str) -> bool {
    let l_parts = parse_version_parts(latest);
    let c_parts = parse_version_parts(current);
    if l_parts.is_empty() {
        return false;
    }
    if c_parts.is_empty() {
        return true;
    }
    let max_len = l_parts.len().max(c_parts.len());
    for i in 0..max_len {
        let l = *l_parts.get(i).unwrap_or(&0);
        let c = *c_parts.get(i).unwrap_or(&0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    false
}

fn find_registry_entry_for_skill(
    database: &AppDatabase,
    name: &str,
    folder_name: &str,
) -> Result<Option<(Option<String>, Option<String>, Option<String>)>, String> {
    let conn = database.connect()?;
    let mut stmt = conn
        .prepare(
            "SELECT payload_json FROM skill_registry_entries
             WHERE name = ?1 OR id = ?1 OR id = ?2 OR id LIKE ?3 OR id LIKE ?4
             LIMIT 1",
        )
        .map_err(|e| format!("查询技能注册表失败: {e}"))?;

    let colon_suffix = format!("%:{}", folder_name);
    let hyphen_suffix = format!("%-{}", folder_name);
    let mut rows = stmt
        .query(params![name, folder_name, colon_suffix, hyphen_suffix])
        .map_err(|e| format!("查询注册表条目失败: {e}"))?;

    if let Some(row) = rows.next().map_err(|e| format!("读取注册表行失败: {e}"))? {
        let payload_str: String = row.get(0).map_err(|e| format!("解析注册表列失败: {e}"))?;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&payload_str) {
            let ver = val.get("version").and_then(|v| v.as_str()).map(ToString::to_string);
            let git_url = val.get("gitUrl").and_then(|v| v.as_str()).map(ToString::to_string);
            let sub_path = val.get("subPath").and_then(|v| v.as_str()).map(ToString::to_string);
            return Ok(Some((ver, git_url, sub_path)));
        }
    }
    Ok(None)
}

#[allow(dead_code)]
fn quick_npm_view_version(pkg: &str) -> Option<String> {
    let mut cmd = Command::new(if cfg!(windows) { "npm.cmd" } else { "npm" });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args(["view", pkg, "version"]);
    let output = run_command_with_timeout(cmd, DEFAULT_GIT_COMMAND_TIMEOUT, "npm view").ok()?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+') {
            return Some(text);
        }
    }
    None
}

#[allow(dead_code)]
fn resolve_unmanaged_skill_source(
    _skill_path: &Path,
    skill_name: &str,
    folder_name: &str,
    database: Option<&AppDatabase>,
) -> Option<(String, Option<String>)> {
    if skill_name.eq_ignore_ascii_case("impeccable") || folder_name.eq_ignore_ascii_case("impeccable") {
        return Some(("https://github.com/pbakaus/impeccable.git".to_string(), Some(".agents/skills/impeccable".to_string())));
    }

    if let Some(db) = database {
        if let Ok(Some((_, Some(git_url), sub_path))) = find_registry_entry_for_skill(db, skill_name, folder_name) {
            return Some((git_url, sub_path));
        }
    }

    None
}

fn check_agent_skills_updates_sync(
    project_cwd: Option<&str>,
    _database: Option<&AppDatabase>,
) -> Vec<AgentSkill> {
    check_agent_skills_updates_with_head_resolver(
        scan_agent_skills_sync(project_cwd),
        remote_head_commit,
    )
}

fn check_agent_skills_updates_with_head_resolver<F>(
    mut skills: Vec<AgentSkill>,
    mut resolve_remote_head: F,
) -> Vec<AgentSkill>
where
    F: FnMut(&str) -> Result<String, String>,
{
    let now = current_timestamp_millis();
    let home = get_home_dir().ok();
    let mut remote_head_cache: HashMap<String, Result<String, String>> = HashMap::new();

    for skill in &mut skills {
        skill.last_checked_at = Some(now);
        skill.update_error = None;

        if skill.externally_managed || skill.management == "external" {
            skill.has_update = false;
            skill.update_status = Some("unsupported".to_string());
            skill.update_summary = Some("由外部安装器管理，请使用原安装器检查更新".to_string());
            continue;
        }

        // 1. 优先检查受管技能源 (.lumina-skill-source.json)
        if let Some(source) = read_managed_source(Path::new(&skill.path)) {
            let repo_key = normalize_git_url_for_compare(&source.git_url);
            let head_result = remote_head_cache
                .entry(repo_key)
                .or_insert_with(|| resolve_remote_head(&source.git_url))
                .clone();

            match head_result {
                Ok(remote_commit) => {
                    if remote_commit != source.commit {
                        skill.has_update = true;
                        skill.update_status = Some("update_available".to_string());
                        skill.commits_behind = 1;
                        skill.latest_commit = Some(remote_commit.chars().take(7).collect());
                        skill.update_summary = Some(if source.sub_path.trim().is_empty() {
                            "技能仓库有新提交".to_string()
                        } else {
                            "来源仓库有新提交（子技能目录未必变更）".to_string()
                        });
                    } else {
                        skill.has_update = false;
                        skill.update_status = Some("up_to_date".to_string());
                    }
                }
                Err(err) => {
                    skill.has_update = false;
                    skill.update_status = Some("error".to_string());
                    skill.update_error = Some(err);
                    skill.update_summary = Some("无法连接来源仓库检查更新".to_string());
                }
            }
            continue;
        }

        // 2. 检查 Git 仓库
        if skill.is_git_repo && skill.git_remote_url.is_some() {
            let fetch_res = run_git(&skill.path, &["fetch", "--quiet"]);

            // 优先检查 tracking branch
            let mut upstream_ref = None;
            if let Ok(u) = run_git(&skill.path, &["rev-parse", "@{u}"]) {
                let trimmed = u.trim().to_string();
                if !trimmed.is_empty() {
                    upstream_ref = Some(trimmed);
                }
            }

            // 若没有默认 tracking branch，尝试以当前分支名字拼装 origin/<branch>
            if upstream_ref.is_none() {
                if let Some(ref b) = skill.git_branch {
                    let remote_b = format!("origin/{}", b);
                    if let Ok(sha) = run_git(&skill.path, &["rev-parse", &remote_b]) {
                        let trimmed = sha.trim().to_string();
                        if !trimmed.is_empty() {
                            upstream_ref = Some(remote_b);
                        }
                    }
                }
            }

            if let Some(ref target_upstream) = upstream_ref {
                let rev_range = format!("HEAD..{}", target_upstream);
                if let Ok(count_str) = run_git(&skill.path, &["rev-list", &rev_range, "--count"]) {
                    if let Ok(count) = count_str.trim().parse::<usize>() {
                        if count > 0 {
                            skill.has_update = true;
                            skill.update_status = Some("update_available".to_string());
                            skill.commits_behind = count;

                            if let Ok(log) = run_git(
                                &skill.path,
                                &["log", "-1", "--pretty=format:%h %s", target_upstream],
                            ) {
                                skill.update_summary = Some(log.trim().to_string());
                            } else {
                                skill.update_summary = Some("技能仓库有新提交".to_string());
                            }

                            if let Ok(sha) =
                                run_git(&skill.path, &["rev-parse", "--short", target_upstream])
                            {
                                skill.latest_commit = Some(sha.trim().to_string());
                            }
                        } else if let Err(fetch_err) = fetch_res {
                            skill.has_update = false;
                            skill.update_status = Some("error".to_string());
                            skill.update_error = Some(fetch_err);
                        } else {
                            skill.has_update = false;
                            skill.update_status = Some("up_to_date".to_string());
                        }
                    }
                } else if let Err(fetch_err) = fetch_res {
                    skill.update_status = Some("error".to_string());
                    skill.update_error = Some(fetch_err);
                } else {
                    skill.update_status = Some("unknown".to_string());
                }
            } else if let Err(fetch_err) = fetch_res {
                skill.update_status = Some("error".to_string());
                skill.update_error = Some(fetch_err);
            } else {
                skill.update_status = Some("unknown".to_string());
            }
            continue;
        }

        // 3. 检查独立/非受管目录技能的本地显式更新缓存（不通过名称猜测远端仓库或篡改来源）
        let skill_path = Path::new(&skill.path);
        let folder_name = skill_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(&skill.name);

        let mut found_latest_version = None;
        let mut candidates = vec![
            skill_path.join(".update-check.json"),
            skill_path.join("update-check.json"),
        ];
        if let Some(ref h) = home {
            candidates.push(h.join(format!(".{}", folder_name)).join("update-check.json"));
            candidates.push(h.join(format!(".{}", skill.name)).join("update-check.json"));
        }
        for cache_file in candidates {
            if cache_file.is_file() {
                if let Ok(content) = fs::read_to_string(&cache_file) {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(latest) = json_val
                            .get("latestVersion")
                            .or_else(|| json_val.get("latest_version"))
                            .or_else(|| json_val.get("version"))
                            .and_then(|v| v.as_str())
                        {
                            found_latest_version = Some(latest.to_string());
                            break;
                        }
                    }
                }
            }
        }

        if let Some(latest_ver) = found_latest_version {
            let cur_ver = skill.version.as_deref().unwrap_or("0.0.0");
            if is_newer_semver(&latest_ver, cur_ver) {
                skill.has_update = true;
                skill.update_status = Some("update_available".to_string());
                skill.commits_behind = 1;
                skill.latest_version = Some(latest_ver.clone());
                skill.update_summary = Some(format!(
                    "发现新版本 {} (当前 {})",
                    latest_ver,
                    skill.version.as_deref().unwrap_or("未标记")
                ));
            } else {
                skill.has_update = false;
                skill.update_status = Some("up_to_date".to_string());
            }
        } else {
            skill.has_update = false;
            skill.update_status = Some("unsupported".to_string());
        }
    }

    skills
}

#[tauri::command]
pub async fn check_agent_skills_updates(
    app: tauri::AppHandle,
    project_cwd: Option<String>,
) -> Result<Vec<AgentSkill>, String> {
    let database = app.try_state::<AppDatabase>().map(|db| db.inner().clone());
    tokio::task::spawn_blocking(move || {
        check_agent_skills_updates_sync(project_cwd.as_deref(), database.as_ref())
    })
    .await
    .map_err(|e| format!("检查技能更新失败: {}", e))
}

fn update_agent_skill_sync(
    skill_path: &str,
    force: bool,
    _database: Option<&AppDatabase>,
) -> Result<SkillUpdateResult, String> {
    let p = Path::new(skill_path);
    if let Some(parent) = p.parent() {
        let _ = recover_interrupted_skill_ops_in_dir(parent);
    }
    if !p.exists() || !p.is_dir() {
        return Err(format!("技能目录不存在: {}", skill_path));
    }
    if is_skills_cli_managed(p) {
        return Err("该技能由 skills CLI 管理，请使用原安装器更新".to_string());
    }

    let folder_name = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "skill".to_string());

    if let Some(source) = read_managed_source(p) {
        validate_skill_git_url(&source.git_url)?;
        let parent = p.parent().ok_or_else(|| "技能目录无效".to_string())?;
        let _lock = acquire_skill_op_lock(parent, &folder_name, "update")?;

        let dirty = hash_skill_files(p)
            .map(|digest| digest != source.content_sha256)
            .unwrap_or(true);
        if dirty && !force {
            return Err("技能文件已在本地修改；请确认后使用强制更新".to_string());
        }

        let stage_dir = parent.join(format!("{STAGE_DIR_PREFIX}{folder_name}"));
        let backup_dir = parent.join(format!("{BACKUP_DIR_PREFIX}{folder_name}"));
        if stage_dir.exists() {
            remove_dir_all_with_retry(&stage_dir);
        }
        let silent_progress: Channel<InstallSkillProgress> = Channel::new(|_| Ok(()));
        deploy_skill_files(
            &source.git_url,
            if source.sub_path.is_empty() { None } else { Some(source.sub_path.as_str()) },
            &stage_dir,
            &silent_progress,
            None,
            None,
        )?;
        // Preserve identity metadata across updates
        if let Some(mut new_source) = read_managed_source(&stage_dir) {
            if new_source.catalog_id.is_none() {
                new_source.catalog_id = source.catalog_id.clone();
            }
            if new_source.sub_skill_id.is_none() {
                new_source.sub_skill_id = source.sub_skill_id.clone();
            }
            if new_source.canonical_name.is_none() {
                new_source.canonical_name = source
                    .canonical_name
                    .clone()
                    .or_else(|| resolve_canonical_skill_name_from_dir(&stage_dir, &folder_name).ok());
            }
            if let Ok(bytes) = serde_json::to_vec_pretty(&new_source) {
                let _ = fs::write(stage_dir.join(MANAGED_SOURCE_FILE), bytes);
            }
        }
        let new_commit = read_managed_source(&stage_dir)
            .map(|value| value.commit.chars().take(7).collect());
        if backup_dir.exists() {
            remove_dir_all_with_retry(&backup_dir);
        }
        if let Err(error) = fs::rename(p, &backup_dir) {
            remove_dir_all_with_retry(&stage_dir);
            return Err(format!("备份当前技能失败: {error}"));
        }
        if let Err(error) = fs::rename(&stage_dir, p) {
            let _ = fs::rename(&backup_dir, p);
            remove_dir_all_with_retry(&stage_dir);
            return Err(format!("替换技能失败: {error}"));
        }

        let mut preserved_backup_path = None;
        if dirty && force {
            let user_backup_dir = parent.join(format!(
                "{USER_BACKUP_DIR_PREFIX}{folder_name}-{}",
                current_timestamp_millis()
            ));
            if fs::rename(&backup_dir, &user_backup_dir).is_ok() {
                preserved_backup_path = Some(user_backup_dir.to_string_lossy().to_string());
            } else {
                preserved_backup_path = Some(backup_dir.to_string_lossy().to_string());
            }
        } else {
            remove_dir_all_with_retry(&backup_dir);
        }

        let message = if let Some(ref b_path) = preserved_backup_path {
            format!("技能已强制更新至最新版本（原本地修改已备份至 {}）", b_path)
        } else {
            "技能已更新到来源仓库的最新版本".to_string()
        };

        return Ok(SkillUpdateResult {
            skill_id: folder_name,
            success: true,
            message,
            previous_commit: Some(source.commit.chars().take(7).collect()),
            new_commit,
            backup_path: preserved_backup_path,
        });
    }

    if p.join(".git").exists() {
        let parent = p.parent().ok_or_else(|| "技能目录无效".to_string())?;
        let _lock = acquire_skill_op_lock(parent, &folder_name, "update")?;

        let remote_url = read_git_remote_url_fast(&p.join(".git"))
            .or_else(|| {
                run_git(skill_path, &["config", "--get", "remote.origin.url"])
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });
        if remote_url.is_none() {
            return Err("当前 Git 技能目录未配置确认的远端仓库地址，无法自动更新".to_string());
        }

        let previous_commit = run_git(skill_path, &["rev-parse", "--short", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string());

        let status_output = run_git(skill_path, &["status", "--porcelain"])
            .map_err(|e| format!("无法检查 Git 工作区状态，已拒绝更新: {e}"))?;
        let dirty = !status_output.trim().is_empty();

        if dirty && !force {
            return Err("检测到本地文件已修改，为防止覆盖您的改动，请在确认后重试或勾选强制更新。".to_string());
        }

        let mut preserved_backup_path = None;
        if dirty && force {
            // Check git stash result strictly; abort if stash fails before any reset/pull
            run_git(
                skill_path,
                &["stash", "push", "--include-untracked", "-m", "halowake-auto-backup-before-update"],
            )
            .map_err(|stash_err| {
                format!("保存本地修改到 Git Stash 失败，已中止强制更新以保护本地文件: {stash_err}")
            })?;
            preserved_backup_path = Some("git stash@{0} (halowake-auto-backup-before-update)".to_string());
        }

        let pull_result = run_git(skill_path, &["pull", "--ff-only"]);

        return match pull_result {
            Ok(_) => {
                let new_commit = run_git(skill_path, &["rev-parse", "--short", "HEAD"])
                    .ok()
                    .map(|s| s.trim().to_string());
                Ok(SkillUpdateResult {
                    skill_id: folder_name,
                    success: true,
                    message: "更新成功".to_string(),
                    previous_commit,
                    new_commit,
                    backup_path: preserved_backup_path,
                })
            }
            Err(err) => {
                if force {
                    run_git(skill_path, &["fetch", "origin"])
                        .map_err(|fetch_err| format!("拉取远端失败: {fetch_err}"))?;
                    if run_git(skill_path, &["reset", "--hard", "@{u}"]).is_ok() {
                        let new_commit = run_git(skill_path, &["rev-parse", "--short", "HEAD"])
                            .ok()
                            .map(|s| s.trim().to_string());
                        return Ok(SkillUpdateResult {
                            skill_id: folder_name,
                            success: true,
                            message: "强制同步重置至最新版本成功".to_string(),
                            previous_commit,
                            new_commit,
                            backup_path: preserved_backup_path,
                        });
                    }
                }
                Err(format!("拉取更新失败: {}", err))
            }
        };
    }

    Err(format!(
        "当前技能「{}」为本地未受管技能（缺少受管来源元数据与 Git 远端），为防止覆盖您的本地文件，不支持自动更新；如需使用在线版本，请先备份或重命名当前目录后从在线目录安装",
        folder_name
    ))
}

#[tauri::command]
pub async fn update_agent_skill(
    app: tauri::AppHandle,
    skill_path: String,
    force: bool,
) -> Result<SkillUpdateResult, String> {
    let database = app.try_state::<AppDatabase>().map(|db| db.inner().clone());
    tokio::task::spawn_blocking(move || update_agent_skill_sync(&skill_path, force, database.as_ref()))
        .await
        .map_err(|e| format!("执行技能更新失败: {}", e))?
}

#[tauri::command]
pub async fn open_skill_folder(path: String) -> Result<(), String> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err(format!("目录不存在: {}", path));
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
        .map_err(|e| format!("打开目录失败 ({}): {}", path, e))?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSkillArgs {
    pub skill_id: String,
    pub git_url: String,
    pub target_mode: Option<String>, // "project" | "specific_agent" | "global"
    pub scope: Option<String>,       // fallback for backwards compatibility
    pub target_agents: Vec<String>,
    pub project_cwd: Option<String>,
    pub add_to_gitignore: bool,
    pub sub_skill_id: Option<String>, // 单独装配的子技能 ID
    pub sub_path: Option<String>,     // 仓库中子技能相对路径 (如 "skills/canvas")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of_operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTargetOutcome {
    pub target_agent: String,
    pub target_path: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSkillResult {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub message: String,
    pub installed_paths: Vec<String>,
    #[serde(default)]
    pub skipped_paths: Vec<String>,
    #[serde(default)]
    pub target_outcomes: Vec<InstallTargetOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstallHistoryEntry {
    pub id: i64,
    pub skill_id: String,
    pub display_name: String,
    pub sub_skill_id: Option<String>,
    pub git_url: String,
    pub sub_path: Option<String>,
    pub target_mode: String,
    pub target_agents: Vec<String>,
    pub project_cwd: Option<String>,
    pub status: String,
    pub message: String,
    pub installed_paths: Vec<String>,
    pub existing_paths: Vec<String>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_outcomes: Vec<InstallTargetOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of_operation_id: Option<String>,
}

fn save_install_history(database: &AppDatabase, args: &InstallSkillArgs, result: &Result<InstallSkillResult, String>, completed_paths: &[String]) -> Result<(), String> {
    let connection = database.connect()?;
    let (status, message, paths): (&str, &str, &[String]) = match result {
        Ok(value) if value.status.as_deref() == Some("partial") => ("partial", value.message.as_str(), &value.installed_paths),
        Ok(value) if value.status.as_deref() == Some("failed") => ("failed", value.message.as_str(), &value.installed_paths),
        Ok(value) if value.installed_paths.is_empty() => ("skipped", value.message.as_str(), &value.installed_paths),
        Ok(value) => ("success", value.message.as_str(), &value.installed_paths),
        Err(error) if !completed_paths.is_empty() => ("partial", error.as_str(), completed_paths),
        Err(error) => ("failed", error.as_str(), &[]),
    };
    let agents = serde_json::to_string(&args.target_agents).map_err(|error| format!("序列化安装目标失败: {error}"))?;
    let paths = serde_json::to_string(paths).map_err(|error| format!("序列化安装路径失败: {error}"))?;
    let target_outcomes_json = match result {
        Ok(value) => serde_json::to_string(&value.target_outcomes).unwrap_or_else(|_| "[]".to_string()),
        Err(_) => "[]".to_string(),
    };
    let display_name = completed_paths.first()
        .or_else(|| result.as_ref().ok().and_then(|r| r.installed_paths.first().or_else(|| r.skipped_paths.first())))
        .and_then(|path| read_skill_markdown(&Path::new(path).join("SKILL.md")))
        .and_then(|content| parse_skill_md(&content).0)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| args.sub_skill_id.clone().unwrap_or_else(|| args.skill_id.clone()));
    connection.execute(
        "INSERT INTO skill_install_history (
            skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, target_agents_json,
            project_cwd, status, message, installed_paths_json, created_at,
            operation_id, item_key, target_outcomes_json, retry_of_operation_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            &args.skill_id, display_name, args.sub_skill_id.as_deref(), &args.git_url, args.sub_path.as_deref(),
            args.target_mode.as_deref().or(args.scope.as_deref()).unwrap_or("project"), agents,
            args.project_cwd.as_deref(), status, message, paths, current_timestamp_millis() as i64,
            args.operation_id.as_deref(), args.item_key.as_deref(), target_outcomes_json, args.retry_of_operation_id.as_deref()
        ],
    ).map_err(|error| format!("写入技能安装记录失败: {error}"))?;
    Ok(())
}

fn load_install_history(database: &AppDatabase) -> Result<Vec<SkillInstallHistoryEntry>, String> {
    let connection = database.connect()?;
    let mut statement = connection.prepare(
        "SELECT id, skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, target_agents_json,
                project_cwd, status, message, installed_paths_json, created_at,
                operation_id, item_key, target_outcomes_json, retry_of_operation_id
         FROM skill_install_history ORDER BY created_at DESC, id DESC"
    ).map_err(|error| format!("准备读取技能安装记录失败: {error}"))?;
    let rows = statement.query_map([], |row| Ok((
        row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?,
        row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, Option<String>>(8)?,
        row.get::<_, String>(9)?, row.get::<_, String>(10)?, row.get::<_, String>(11)?, row.get::<_, i64>(12)?,
        row.get::<_, Option<String>>(13)?, row.get::<_, Option<String>>(14)?,
        row.get::<_, Option<String>>(15)?, row.get::<_, Option<String>>(16)?,
    ))).map_err(|error| format!("查询技能安装记录失败: {error}"))?;
    let mut history = Vec::new();
    for row in rows {
        let (id, skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, agents_json,
            project_cwd, status, message, paths_json, created_at,
            operation_id, item_key, target_outcomes_json, retry_of_operation_id) = row.map_err(|error| format!("读取技能安装记录失败: {error}"))?;
        let target_agents: Vec<String> = serde_json::from_str(&agents_json).map_err(|error| format!("解析安装目标失败: {error}"))?;
        let installed_paths: Vec<String> = serde_json::from_str(&paths_json).map_err(|error| format!("解析安装路径失败: {error}"))?;
        let existing_paths = installed_paths.iter().filter(|path| Path::new(path).is_dir()).cloned().collect();
        let target_outcomes: Vec<InstallTargetOutcome> = target_outcomes_json
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();
        history.push(SkillInstallHistoryEntry {
            id, skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, target_agents,
            project_cwd, status, message, installed_paths, existing_paths,
            created_at: created_at.max(0) as u64,
            operation_id, item_key, target_outcomes, retry_of_operation_id,
        });
    }
    Ok(history)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSkillProgress {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_key: Option<String>,
    pub phase: &'static str,
    pub message: String,
    pub percent: Option<u8>,
}

fn send_install_progress(
    progress: &Channel<InstallSkillProgress>,
    operation_id: Option<&str>,
    item_key: Option<&str>,
    phase: &'static str,
    message: impl Into<String>,
    percent: Option<u8>,
) {
    let _ = progress.send(InstallSkillProgress {
        operation_id: operation_id.map(str::to_string),
        item_key: item_key.map(str::to_string),
        phase,
        message: message.into(),
        percent,
    });
}

static INSTALL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn install_temp_path(parent: &Path, prefix: &str) -> PathBuf {
    let sequence = INSTALL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{}-{}-{}-{}",
        prefix,
        std::process::id(),
        current_timestamp_millis(),
        sequence
    ))
}

fn validate_skill_git_url(git_url: &str) -> Result<(), String> {
    let repo = git_url
        .trim()
        .strip_prefix("https://github.com/")
        .ok_or_else(|| "仅支持 GitHub HTTPS 技能仓库地址".to_string())?
        .trim_end_matches(".git");
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty() || *part == "." || *part == ".."
                || !part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        })
    {
        return Err("GitHub 仓库地址格式无效".to_string());
    }
    Ok(())
}

fn remote_head_commit(git_url: &str) -> Result<String, String> {
    validate_skill_git_url(git_url)?;
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args(["-c", "credential.interactive=never", "ls-remote", git_url, "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = run_command_with_timeout(cmd, DEFAULT_GIT_COMMAND_TIMEOUT, "git ls-remote")
        .map_err(|error| format!("检查技能远端失败: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "检查技能远端失败".to_string()
        } else {
            format!("检查技能远端失败: {stderr}")
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let sha = text.split_whitespace().next().unwrap_or("");
    if sha.len() < 7 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("远端未返回有效提交号".to_string());
    }
    Ok(sha.to_string())
}

fn clone_percent(line: &str) -> Option<u8> {
    let stage_start = if line.contains("Receiving objects:") { 10 }
        else if line.contains("Resolving deltas:") { 80 }
        else if line.contains("Updating files:") { 95 }
        else { return None };
    let stage_weight = match stage_start { 10 => 70, 80 => 15, _ => 3 };
    let number = line.split('%').next()?.rsplit(|character: char| !character.is_ascii_digit()).next()?.parse::<u8>().ok()?;
    Some(stage_start + (number.min(100) as u16 * stage_weight / 100) as u8)
}

fn is_transient_git_network_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    ["unexpected eof", "tls connect error", "ssl_error_syscall", "connection reset", "curl 56", "curl 35"]
        .iter().any(|signal| lower.contains(signal))
}

fn git_error_summary(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    text.split(['\r', '\n'])
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with("fatal:") || line.starts_with("error:"))
        .unwrap_or_else(|| text.trim())
        .to_string()
}

fn run_git_clone_attempt(
    git_url: &str,
    target_dir: &Path,
    sub_path: Option<&str>,
    progress: &Channel<InstallSkillProgress>,
    operation_id: Option<&str>,
    item_key: Option<&str>,
) -> Result<(), String> {
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.args(["-c", "credential.interactive=never", "clone", "--progress", "--depth", "1"]);
    if sub_path.is_some() {
        cmd.args(["--filter=blob:none", "--sparse"]);
    }
    cmd.arg(git_url)
        .arg(target_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|error| format!("无法启动 git 命令: {error}"))?;

    let job = {
        let j = ProcessJobGuard::new();
        if let Some(ref guard) = j {
            guard.assign_child(&child);
        }
        j
    };

    let mut stderr = child.stderr.take().ok_or_else(|| "无法读取 Git 下载进度".to_string())?;
    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match stderr.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    if tx.send(Ok(buffer[..count].to_vec())).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(format!("读取 Git 下载进度失败: {error}")));
                    break;
                }
            }
        }
    });

    let start = Instant::now();
    let mut tail = Vec::new();
    let mut line = Vec::new();
    let mut last_percent = None;
    loop {
        if start.elapsed() >= GIT_CLONE_TIMEOUT {
            kill_process_tree(&mut child, &job);
            let _ = child.wait();
            return Err(format!(
                "Git 下载超时 ({}s)，请检查网络或代理设置",
                GIT_CLONE_TIMEOUT.as_secs()
            ));
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(chunk)) => {
                tail.extend_from_slice(&chunk);
                if tail.len() > 16_384 {
                    tail.drain(..tail.len() - 16_384);
                }
                for byte in &chunk {
                    if *byte == b'\r' || *byte == b'\n' {
                        if !line.is_empty() {
                            let text = String::from_utf8_lossy(&line);
                            if let Some(percent) = clone_percent(&text) {
                                if last_percent != Some(percent) {
                                    send_install_progress(
                                        progress,
                                        operation_id,
                                        item_key,
                                        "downloading",
                                        text.trim().to_string(),
                                        Some(percent),
                                    );
                                    last_percent = Some(percent);
                                }
                            }
                            line.clear();
                        }
                    } else if line.len() < 2048 {
                        line.push(*byte);
                    }
                }
            }
            Ok(Err(read_err)) => {
                kill_process_tree(&mut child, &job);
                let _ = child.wait();
                return Err(read_err);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() >= GIT_CLONE_TIMEOUT {
                    kill_process_tree(&mut child, &job);
                    let _ = child.wait();
                    return Err(format!(
                        "Git 下载超时 ({}s)，请检查网络或代理设置",
                        GIT_CLONE_TIMEOUT.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(30));
            }
            Err(e) => {
                kill_process_tree(&mut child, &job);
                let _ = child.wait();
                return Err(format!("等待 Git 下载失败: {e}"));
            }
        }
    };

    if !status.success() {
        let message = git_error_summary(&tail);
        return Err(if message.is_empty() { "git clone 失败，请检查网络或仓库地址".to_string() } else { message });
    }
    Ok(())
}

fn execute_git_clone_silent(
    git_url: &str,
    target_dir: &Path,
    sub_path: Option<&str>,
    progress: &Channel<InstallSkillProgress>,
    operation_id: Option<&str>,
    item_key: Option<&str>,
) -> Result<(), String> {
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    send_install_progress(progress, operation_id, item_key, "connecting", "正在连接 GitHub…", None);
    let mut result = run_git_clone_attempt(git_url, target_dir, sub_path, progress, operation_id, item_key);
    if let Err(error) = &result {
        if is_transient_git_network_error(error) {
            send_install_progress(progress, operation_id, item_key, "retrying", "连接中断，正在重试下载…", None);
            if target_dir.exists() {
                fs::remove_dir_all(target_dir).map_err(|cleanup_error| format!("重试前清理临时目录失败: {cleanup_error}"))?;
            }
            std::thread::sleep(Duration::from_millis(700));
            result = run_git_clone_attempt(git_url, target_dir, sub_path, progress, operation_id, item_key);
        }
    }
    result.map_err(|error| {
        if is_transient_git_network_error(&error) {
            format!("连接 GitHub 时 TLS/网络连接中断。请检查网络或代理后重试。\n{error}")
        } else {
            error
        }
    })?;
    if let Some(sub) = sub_path {
        send_install_progress(progress, operation_id, item_key, "extracting", "正在定位并提取子技能…", Some(96));
        let mut sparse = Command::new("git");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            sparse.creation_flags(CREATE_NO_WINDOW);
        }
        sparse
            .arg("-C").arg(target_dir)
            .args(["sparse-checkout", "set", "--", sub]);
        let output = run_command_with_timeout(sparse, DEFAULT_GIT_COMMAND_TIMEOUT, "git sparse-checkout")
            .map_err(|e| format!("无法展开技能目录: {}", e))?;
        if !output.status.success() {
            return Err(format!("无法展开技能目录 {}: {}", sub, String::from_utf8_lossy(&output.stderr)));
        }
    }
    Ok(())
}

fn append_to_gitignore_if_needed(project_cwd: &str) -> Result<(), String> {
    let p = Path::new(project_cwd);
    if !p.exists() {
        return Ok(());
    }
    let gitignore_path = p.join(".gitignore");
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).unwrap_or_default()
    } else {
        String::new()
    };

    let already_ignored = content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == ".agents/skills"
            || trimmed == ".agents/skills/"
            || trimmed == "/.agents/skills/"
            || trimmed == ".agents"
            || trimmed == ".agents/"
    });

    if !already_ignored {
        let mut new_content = content;
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("\n# AI Agent Skills (Halowake)\n.agents/skills/\n");
        fs::write(&gitignore_path, new_content).map_err(|e| format!("更新 .gitignore 失败: {}", e))?;
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "技能目录包含符号链接"));
        }
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn remove_dir_all_with_retry(path: &Path) {
    for _ in 0..5 {
        if fs::remove_dir_all(path).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(60));
    }
}

fn find_skill_dir_in_repo(repo_dir: &Path, expected_name: Option<&str>) -> Option<String> {
    let output = run_git(&repo_dir.to_string_lossy(), &["ls-tree", "-r", "--name-only", "HEAD"]).ok()?;
    let skill_md_paths: Vec<&str> = output
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with("SKILL.md"))
        .collect();

    if skill_md_paths.is_empty() {
        return None;
    }

    if let Some(name) = expected_name.filter(|n| !n.trim().is_empty()) {
        let name_lower = name.to_ascii_lowercase();
        let preferred_prefixes = [".agents/skills/", "skills/", ".agent/skills/", ".claude/skills/", ".gemini/skills/"];
        for prefix in preferred_prefixes {
            let candidate = format!("{}{}/skill.md", prefix, name_lower);
            if let Some(matched) = skill_md_paths.iter().find(|p| p.to_ascii_lowercase() == candidate) {
                return Path::new(matched).parent().and_then(|p| p.to_str()).map(|s| s.replace('\\', "/"));
            }
        }

        if let Some(matched) = skill_md_paths.iter().find(|p| {
            let lower = p.to_ascii_lowercase();
            lower.ends_with(&format!("/{}/skill.md", name_lower)) || lower == format!("{}/skill.md", name_lower)
        }) {
            return Path::new(matched).parent().and_then(|p| p.to_str()).map(|s| s.replace('\\', "/"));
        }
    }

    if skill_md_paths.len() == 1 {
        let single = skill_md_paths[0];
        if single.eq_ignore_ascii_case("SKILL.md") {
            return Some("".to_string());
        }
        return Path::new(single).parent().and_then(|p| p.to_str()).map(|s| s.replace('\\', "/"));
    }

    for prefix in [".agents/skills/", "skills/"] {
        if let Some(matched) = skill_md_paths.iter().find(|p| p.starts_with(prefix)) {
            return Path::new(matched).parent().and_then(|p| p.to_str()).map(|s| s.replace('\\', "/"));
        }
    }

    None
}

fn deploy_skill_files(
    git_url: &str,
    sub_path: Option<&str>,
    target_dir: &Path,
    progress: &Channel<InstallSkillProgress>,
    operation_id: Option<&str>,
    item_key: Option<&str>,
) -> Result<(), String> {
    let parent = target_dir.parent().ok_or_else(|| "安装目标目录无效".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("创建安装目录失败: {}", e))?;
    let staging_dir = install_temp_path(parent, "lumina-skill-stage");
    let result = if let Some(sub) = sub_path.filter(|s| !s.trim().is_empty()) {
        let clean_sub = normalize_skill_sub_path(sub)?;
        let temp_dir = install_temp_path(&std::env::temp_dir(), "lumina-subskill");
        let copy_result = (|| -> Result<(), String> {
            execute_git_clone_silent(git_url, &temp_dir, Some(&clean_sub), progress, operation_id, item_key)?;
            let canonical_repo = fs::canonicalize(&temp_dir)
                .map_err(|e| format!("无法检查仓库目录: {}", e))?;

            let mut resolved_sub = clean_sub.clone();
            let mut canonical_src = temp_dir.join(&clean_sub).canonicalize().ok();
            let mut has_skill_md = canonical_src.as_ref()
                .map(|p| p.join("SKILL.md").is_file())
                .unwrap_or(false);

            if !has_skill_md {
                // 如果指定的子目录没有找到 SKILL.md，通过 git 仓库树动态嗅探真正的技能目录位置
                let skill_id_hint = clean_sub.rsplit('/').next();
                if let Some(discovered) = find_skill_dir_in_repo(&temp_dir, skill_id_hint) {
                    let mut sparse = Command::new("git");
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        sparse.creation_flags(CREATE_NO_WINDOW);
                    }
                    let _ = sparse.arg("-C").arg(&temp_dir).args(["sparse-checkout", "set", "--", &discovered]).output();
                    if let Ok(canon) = temp_dir.join(&discovered).canonicalize() {
                        if canon.join("SKILL.md").is_file() {
                            canonical_src = Some(canon);
                            resolved_sub = normalize_skill_sub_path(&discovered).unwrap_or(discovered);
                            has_skill_md = true;
                        }
                    }
                }
            }

            let canonical_src = canonical_src.ok_or_else(|| format!("子目录 {} 中没有 SKILL.md", clean_sub))?;
            if !canonical_src.starts_with(&canonical_repo) || !canonical_src.is_dir() {
                return Err("子技能路径不在仓库目录内".to_string());
            }
            if !has_skill_md || !canonical_src.join("SKILL.md").is_file() {
                return Err(format!("子目录 {} 中没有 SKILL.md", resolved_sub));
            }
            send_install_progress(progress, operation_id, item_key, "copying", "正在复制技能文件…", Some(98));
            copy_dir_all(&canonical_src, &staging_dir)
                .map_err(|e| format!("复制子技能文件失败: {}", e))?;
            let commit = run_git(&temp_dir.to_string_lossy(), &["rev-parse", "HEAD"])
                .map_err(|e| format!("读取技能来源提交失败: {}", e))?
                .trim().to_string();
            let canonical_name = resolve_canonical_skill_name_from_dir(&staging_dir, "skill").ok();
            let source = ManagedSkillSource {
                catalog_id: None,
                sub_skill_id: None,
                canonical_name,
                git_url: git_url.to_string(),
                sub_path: resolved_sub,
                commit,
                content_sha256: hash_skill_files(&staging_dir)?,
            };
            let metadata = serde_json::to_vec_pretty(&source)
                .map_err(|e| format!("生成技能来源信息失败: {}", e))?;
            fs::write(staging_dir.join(MANAGED_SOURCE_FILE), metadata)
                .map_err(|e| format!("保存技能来源信息失败: {}", e))
        })();
        remove_dir_all_with_retry(&temp_dir);
        copy_result
    } else {
        execute_git_clone_silent(git_url, &staging_dir, None, progress, operation_id, item_key).and_then(|_| {
            if !staging_dir.join("SKILL.md").is_file() {
                return Err("仓库根目录没有 SKILL.md，请选择具体技能目录".to_string());
            }
            // Validate no symlinks exist inside the cloned skill directory and compute hash (excluding .git)
            let content_sha256 = hash_skill_files(&staging_dir)?;
            let commit = run_git(&staging_dir.to_string_lossy(), &["rev-parse", "HEAD"])
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let canonical_name = resolve_canonical_skill_name_from_dir(&staging_dir, "skill").ok();
            let source = ManagedSkillSource {
                catalog_id: None,
                sub_skill_id: None,
                canonical_name,
                git_url: git_url.to_string(),
                sub_path: String::new(),
                commit,
                content_sha256,
            };
            let metadata = serde_json::to_vec_pretty(&source)
                .map_err(|e| format!("生成技能来源信息失败: {}", e))?;
            fs::write(staging_dir.join(MANAGED_SOURCE_FILE), metadata)
                .map_err(|e| format!("保存技能来源信息失败: {}", e))?;
            let git_exclude = staging_dir.join(".git").join("info").join("exclude");
            if let Some(info_dir) = git_exclude.parent() {
                if info_dir.is_dir() {
                    let existing = fs::read_to_string(&git_exclude).unwrap_or_default();
                    if !existing.lines().any(|line| line.trim() == MANAGED_SOURCE_FILE) {
                        let appended = if existing.ends_with('\n') || existing.is_empty() {
                            format!("{existing}{MANAGED_SOURCE_FILE}\n")
                        } else {
                            format!("{existing}\n{MANAGED_SOURCE_FILE}\n")
                        };
                        let _ = fs::write(&git_exclude, appended);
                    }
                }
            }
            Ok(())
        })
    };
    if let Err(error) = result {
        remove_dir_all_with_retry(&staging_dir);
        return Err(error);
    }
    send_install_progress(progress, operation_id, item_key, "saving", "正在写入目标目录…", Some(99));
    fs::rename(&staging_dir, target_dir).map_err(|e| {
        remove_dir_all_with_retry(&staging_dir);
        format!("完成技能安装失败: {}", e)
    })
}

/// Atomically commits a prepared skill directory (`prepared_dir`) into `parent/<canonical_name>`
/// under a cross-process exclusive lock:
/// - Resolves `canonical_name` from `SKILL.md` frontmatter `name` (or `fallback_id`)
/// - Records `catalog_id` and `canonical_name` in `.lumina-skill-source.json`
/// - Rejects cross-source same-name conflicts
/// - Protects user-modified files when `allow_replace_same_source && !force_replace_dirty`
/// - Uses `.lumina-stage-<name>` and `.lumina-backup-<name>` for crash-safe replacement
pub(crate) fn commit_prepared_skill_to_parent(
    prepared_dir: &Path,
    parent: &Path,
    catalog_id: &str,
    fallback_id: &str,
    git_url: &str,
    sub_path: &str,
    allow_replace_same_source: bool,
    force_replace_dirty: bool,
) -> Result<(PathBuf, bool), String> {
    let normalized_incoming_sub = match normalize_skill_sub_path(sub_path) {
        Ok(sub) => sub,
        Err(err) => {
            remove_dir_all_with_retry(prepared_dir);
            return Err(err);
        }
    };
    if !prepared_dir.join("SKILL.md").is_file() {
        remove_dir_all_with_retry(prepared_dir);
        return Err("待安装技能目录缺少 SKILL.md".to_string());
    }
    let content_sha256 = match hash_skill_files(prepared_dir) {
        Ok(digest) => digest,
        Err(err) => {
            remove_dir_all_with_retry(prepared_dir);
            return Err(err);
        }
    };
    let canonical_name = match resolve_canonical_skill_name_from_dir(prepared_dir, fallback_id) {
        Ok(name) => name,
        Err(err) => {
            remove_dir_all_with_retry(prepared_dir);
            return Err(err);
        }
    };

    let resolved_sub = match read_managed_source(prepared_dir)
        .map(|s| normalize_skill_sub_path(&s.sub_path))
        .unwrap_or(Ok(normalized_incoming_sub))
    {
        Ok(sub) => sub,
        Err(err) => {
            remove_dir_all_with_retry(prepared_dir);
            return Err(err);
        }
    };

    let inferred_sub_skill_id = if !fallback_id.trim().is_empty()
        && !fallback_id.trim().eq_ignore_ascii_case(catalog_id.trim())
    {
        Some(fallback_id.trim().to_string())
    } else {
        None
    };

    if let Some(mut source) = read_managed_source(prepared_dir) {
        source.catalog_id = Some(catalog_id.to_string());
        if source.sub_skill_id.is_none() {
            source.sub_skill_id = inferred_sub_skill_id;
        }
        source.canonical_name = Some(canonical_name.clone());
        source.sub_path = resolved_sub.clone();
        source.content_sha256 = content_sha256;
        if let Ok(bytes) = serde_json::to_vec_pretty(&source) {
            let _ = fs::write(prepared_dir.join(MANAGED_SOURCE_FILE), bytes);
        }
    }

    let _ = recover_interrupted_skill_ops_in_dir(parent);
    let _canonical_lock = match acquire_skill_op_lock(
        parent,
        &canonical_name,
        if allow_replace_same_source { "update" } else { "install" },
    ) {
        Ok(guard) => guard,
        Err(err) => {
            remove_dir_all_with_retry(prepared_dir);
            return Err(err);
        }
    };

    let matched_existing_dir = match find_existing_installed_skill_in_parent(
        parent,
        catalog_id,
        fallback_id,
        Some(&canonical_name),
        git_url,
        &resolved_sub,
    ) {
        Ok(existing) => existing,
        Err(conflict_err) => {
            remove_dir_all_with_retry(prepared_dir);
            return Err(conflict_err);
        }
    };

    let final_target_dir = matched_existing_dir.unwrap_or_else(|| parent.join(&canonical_name));
    let target_folder_name = final_target_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&canonical_name)
        .to_string();

    let _legacy_lock = if !target_folder_name.eq_ignore_ascii_case(&canonical_name) {
        match acquire_skill_op_lock(
            parent,
            &target_folder_name,
            if allow_replace_same_source { "update" } else { "install" },
        ) {
            Ok(guard) => Some(guard),
            Err(err) => {
                remove_dir_all_with_retry(prepared_dir);
                return Err(err);
            }
        }
    } else {
        None
    };

    if final_target_dir.exists() {
        if !allow_replace_same_source {
            // Re-verify disposition in case final_target_dir was not matched via same-source
            match check_existing_target_disposition(&final_target_dir, git_url, &resolved_sub) {
                Ok(_) => {
                    remove_dir_all_with_retry(prepared_dir);
                    return Ok((final_target_dir, false));
                }
                Err(conflict_err) => {
                    remove_dir_all_with_retry(prepared_dir);
                    return Err(conflict_err);
                }
            }
        }

        if let Some(existing_src) = read_managed_source(&final_target_dir) {
            let dirty = hash_skill_files(&final_target_dir)
                .map(|digest| digest != existing_src.content_sha256)
                .unwrap_or(true);
            if dirty && !force_replace_dirty {
                remove_dir_all_with_retry(prepared_dir);
                return Err("技能文件已在本地修改；请确认后使用强制更新".to_string());
            }
        } else {
            match check_existing_target_disposition(&final_target_dir, git_url, &resolved_sub) {
                Ok(_) => {}
                Err(conflict_err) => {
                    remove_dir_all_with_retry(prepared_dir);
                    return Err(conflict_err);
                }
            }
        }
    }

    let stage_dir = parent.join(format!("{STAGE_DIR_PREFIX}{target_folder_name}"));
    let backup_dir = parent.join(format!("{BACKUP_DIR_PREFIX}{target_folder_name}"));
    if stage_dir.exists() {
        remove_dir_all_with_retry(&stage_dir);
    }
    if prepared_dir != stage_dir {
        if let Err(err) = fs::rename(prepared_dir, &stage_dir) {
            remove_dir_all_with_retry(prepared_dir);
            return Err(format!("准备技能暂存目录失败: {err}"));
        }
    }

    if final_target_dir.exists() {
        if backup_dir.exists() {
            remove_dir_all_with_retry(&backup_dir);
        }
        if let Err(err) = fs::rename(&final_target_dir, &backup_dir) {
            remove_dir_all_with_retry(&stage_dir);
            return Err(format!("备份现有技能目录失败: {err}"));
        }
        if let Err(err) = fs::rename(&stage_dir, &final_target_dir) {
            let _ = fs::rename(&backup_dir, &final_target_dir);
            remove_dir_all_with_retry(&stage_dir);
            return Err(format!("替换技能目标目录失败: {err}"));
        }
        remove_dir_all_with_retry(&backup_dir);
    } else if let Err(err) = fs::rename(&stage_dir, &final_target_dir) {
        remove_dir_all_with_retry(&stage_dir);
        return Err(format!("完成技能安装失败: {err}"));
    }

    Ok((final_target_dir, true))
}

fn install_skill_into_parent(
    parent_dir: &Path,
    catalog_id: &str,
    effective_id: &str,
    git_url: &str,
    sub_path: Option<&str>,
    progress: &Channel<InstallSkillProgress>,
    operation_id: Option<&str>,
    item_key: Option<&str>,
) -> Result<(PathBuf, bool), String> {
    fs::create_dir_all(parent_dir).map_err(|e| format!("创建安装父目录失败: {e}"))?;
    let _ = recover_interrupted_skill_ops_in_dir(parent_dir);

    // Pre-check if the skill is already installed under canonical_name, catalog_id, or source metadata
    if let Some(existing_dir) = find_existing_installed_skill_in_parent(
        parent_dir,
        catalog_id,
        effective_id,
        None,
        git_url,
        sub_path.unwrap_or(""),
    )? {
        return Ok((existing_dir, false));
    }

    let prepared_dir = install_temp_path(parent_dir, "lumina-skill-prep");
    deploy_skill_files(git_url, sub_path, &prepared_dir, progress, operation_id, item_key)?;
    commit_prepared_skill_to_parent(
        &prepared_dir,
        parent_dir,
        catalog_id,
        effective_id,
        git_url,
        sub_path.unwrap_or(""),
        false,
        false,
    )
}

fn install_agent_skill_sync(args: InstallSkillArgs, progress: &Channel<InstallSkillProgress>, installed_paths: &mut Vec<String>) -> Result<InstallSkillResult, String> {
    if args.git_url.trim().is_empty() {
        return Err("Git 仓库地址不能为空".to_string());
    }
    validate_skill_git_url(&args.git_url)?;

    let clean_id = args.skill_id.trim();
    if clean_id.is_empty() {
        return Err("技能 ID 不能为空".to_string());
    }

    // 若指定了子技能 ID，优先以子技能 ID 作为回退名称；否则以主技能 ID 作为回退名称，最终目录名由 SKILL.md canonical name 决定
    let effective_id = match args.sub_skill_id.as_deref() {
        Some(sid) if !sid.trim().is_empty() => sid.trim(),
        _ => clean_id,
    };
    if !effective_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("技能 ID 只能包含字母、数字、横线和下划线".to_string());
    }

    if let Some(sub) = args.sub_path.as_deref().filter(|s| !s.trim().is_empty()) {
        normalize_skill_sub_path(sub)?;
    }

    let mode = args
        .target_mode
        .as_deref()
        .or(args.scope.as_deref())
        .unwrap_or("project");

    let op_id = args.operation_id.as_deref();
    let it_key = args.item_key.as_deref();
    let item_key_base = args
        .item_key
        .as_deref()
        .unwrap_or(args.sub_skill_id.as_deref().unwrap_or(&args.skill_id));

    let mut skipped_paths = Vec::new();
    let mut target_outcomes = Vec::new();

    if mode == "project" {
        let cwd = match args.project_cwd.as_deref() {
            Some(c) if !c.trim().is_empty() => c.trim(),
            _ => return Err("未指定当前项目工作区目录，请先打开或选择项目".to_string()),
        };

        let parent_dir = Path::new(cwd).join(".agents").join("skills");
        let (target_dir, newly_installed) = install_skill_into_parent(
            &parent_dir,
            clean_id,
            effective_id,
            &args.git_url,
            args.sub_path.as_deref(),
            progress,
            op_id,
            it_key,
        )?;
        let path_str = target_dir.to_string_lossy().to_string();
        if !newly_installed {
            skipped_paths.push(path_str.clone());
            target_outcomes.push(InstallTargetOutcome {
                target_agent: "project".to_string(),
                target_path: path_str,
                status: "skipped".to_string(),
                error: None,
                item_key: Some(format!("{item_key_base}::project")),
            });
            return Ok(InstallSkillResult {
                success: true,
                status: Some("skipped".to_string()),
                message: format!("项目目录中已存在该技能: {}", target_dir.display()),
                installed_paths: Vec::new(),
                skipped_paths,
                target_outcomes,
            });
        }
        installed_paths.push(path_str.clone());
        target_outcomes.push(InstallTargetOutcome {
            target_agent: "project".to_string(),
            target_path: path_str,
            status: "installed".to_string(),
            error: None,
            item_key: Some(format!("{item_key_base}::project")),
        });

        if args.add_to_gitignore {
            let _ = append_to_gitignore_if_needed(cwd);
        }
    } else if mode == "global" {
        let home = get_home_dir()?;
        let parent_dir = home.join(".agents").join("skills");
        let (target_dir, newly_installed) = install_skill_into_parent(
            &parent_dir,
            clean_id,
            effective_id,
            &args.git_url,
            args.sub_path.as_deref(),
            progress,
            op_id,
            it_key,
        )?;
        let path_str = target_dir.to_string_lossy().to_string();
        if !newly_installed {
            skipped_paths.push(path_str.clone());
            target_outcomes.push(InstallTargetOutcome {
                target_agent: "global".to_string(),
                target_path: path_str,
                status: "skipped".to_string(),
                error: None,
                item_key: Some(format!("{item_key_base}::global")),
            });
            return Ok(InstallSkillResult {
                success: true,
                status: Some("skipped".to_string()),
                message: "全局通用技能目录中已存在该技能".to_string(),
                installed_paths: Vec::new(),
                skipped_paths,
                target_outcomes,
            });
        }
        installed_paths.push(path_str.clone());
        target_outcomes.push(InstallTargetOutcome {
            target_agent: "global".to_string(),
            target_path: path_str,
            status: "installed".to_string(),
            error: None,
            item_key: Some(format!("{item_key_base}::global")),
        });
    } else {
        // Specific Agent
        let home = get_home_dir()?;
        let agents = if args.target_agents.is_empty() {
            vec!["antigravity".to_string()]
        } else {
            args.target_agents.clone()
        };

        let mut failed_errors = Vec::new();

        for agent in agents {
            let parent_dir = match agent.as_str() {
                "antigravity" => home.join(".gemini").join("config").join("skills"),
                "claude" => home.join(".claude").join("skills"),
                "codex" => env::var("CODEX_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| home.join(".codex"))
                    .join("skills"),
                "cursor" => home.join(".cursor").join("skills"),
                "trae" => home.join(".trae").join("skills"),
                _ => home.join(".agents").join("skills"),
            };

            match install_skill_into_parent(
                &parent_dir,
                clean_id,
                effective_id,
                &args.git_url,
                args.sub_path.as_deref(),
                progress,
                op_id,
                it_key,
            ) {
                Ok((target_dir, true)) => {
                    let path_str = target_dir.to_string_lossy().to_string();
                    installed_paths.push(path_str.clone());
                    target_outcomes.push(InstallTargetOutcome {
                        target_agent: agent.clone(),
                        target_path: path_str,
                        status: "installed".to_string(),
                        error: None,
                        item_key: Some(format!("{item_key_base}::{agent}")),
                    });
                }
                Ok((target_dir, false)) => {
                    let path_str = target_dir.to_string_lossy().to_string();
                    skipped_paths.push(path_str.clone());
                    target_outcomes.push(InstallTargetOutcome {
                        target_agent: agent.clone(),
                        target_path: path_str,
                        status: "skipped".to_string(),
                        error: None,
                        item_key: Some(format!("{item_key_base}::{agent}")),
                    });
                }
                Err(err) => {
                    failed_errors.push(format!("{agent}: {err}"));
                    target_outcomes.push(InstallTargetOutcome {
                        target_agent: agent.clone(),
                        target_path: parent_dir.to_string_lossy().to_string(),
                        status: "failed".to_string(),
                        error: Some(err),
                        item_key: Some(format!("{item_key_base}::{agent}")),
                    });
                }
            }
        }

        if !failed_errors.is_empty() {
            let is_partial = !installed_paths.is_empty() || !skipped_paths.is_empty();
            let status_str = if is_partial { "partial" } else { "failed" };
            return Ok(InstallSkillResult {
                success: false,
                status: Some(status_str.to_string()),
                message: if is_partial {
                    format!(
                        "部分目标安装完成（新增 {} 个，已跳过 {} 个，失败 {} 个）：{}",
                        installed_paths.len(),
                        skipped_paths.len(),
                        failed_errors.len(),
                        failed_errors.join("；")
                    )
                } else {
                    format!("目标安装失败：{}", failed_errors.join("；"))
                },
                installed_paths: installed_paths.clone(),
                skipped_paths,
                target_outcomes,
            });
        }

        if installed_paths.is_empty() {
            return Ok(InstallSkillResult {
                success: true,
                status: Some("skipped".to_string()),
                message: "指定的目标目录中均已存在该技能".to_string(),
                installed_paths: Vec::new(),
                skipped_paths,
                target_outcomes,
            });
        }
    }

    send_install_progress(progress, op_id, it_key, "complete", "技能安装完成", Some(100));
    Ok(InstallSkillResult {
        success: true,
        status: Some("installed".to_string()),
        message: format!("成功安装技能至 {} 个目标路径", installed_paths.len()),
        installed_paths: installed_paths.clone(),
        skipped_paths,
        target_outcomes,
    })
}

#[tauri::command]
pub async fn install_agent_skill(database: State<'_, AppDatabase>, args: InstallSkillArgs, on_progress: Channel<InstallSkillProgress>) -> Result<InstallSkillResult, String> {
    let database = database.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut completed_paths = Vec::new();
        let result = install_agent_skill_sync(args.clone(), &on_progress, &mut completed_paths)
            .map_err(|error| if completed_paths.is_empty() { error } else { format!("{error}\n已有 {} 个目标目录安装完成，请查看安装记录。", completed_paths.len()) });
        if let Err(error) = save_install_history(&database, &args, &result, &completed_paths) {
            log::error!("{error}");
        }
        result
    })
        .await
        .map_err(|e| format!("安装任务异常: {}", e))?
}

#[tauri::command]
pub async fn list_skill_install_history(database: State<'_, AppDatabase>) -> Result<Vec<SkillInstallHistoryEntry>, String> {
    let database = database.inner().clone();
    tokio::task::spawn_blocking(move || load_install_history(&database))
        .await
        .map_err(|error| format!("读取技能安装记录任务失败: {error}"))?
}

fn uninstall_agent_skill_with_targets(
    target: &Path,
    roots: &[AgentTarget],
) -> Result<bool, String> {
    validate_uninstall_target(target, roots)?;
    let parent = target.parent().ok_or_else(|| "技能目录无效".to_string())?;
    let folder_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| "技能目录名称无效".to_string())?
        .to_string();

    let _ = recover_interrupted_skill_ops_in_dir(parent);
    let canonical_name = read_managed_source(target)
        .and_then(|s| s.canonical_name)
        .or_else(|| resolve_canonical_skill_name_from_dir(target, &folder_name).ok());

    let _folder_lock = acquire_skill_op_lock(parent, &folder_name, "uninstall")?;
    let _canon_lock = match canonical_name.as_deref() {
        Some(canon) if !canon.eq_ignore_ascii_case(&folder_name) => {
            Some(acquire_skill_op_lock(parent, canon, "uninstall")?)
        }
        _ => None,
    };

    // Re-validate inside the exclusive lock to prevent TOCTOU race with concurrent update/install
    validate_uninstall_target(target, roots)?;
    fs::remove_dir_all(target).map_err(|e| format!("删除技能目录失败: {}", e))?;
    Ok(true)
}

#[tauri::command]
pub async fn uninstall_agent_skill(skill_path: String, project_cwd: Option<String>) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let target = Path::new(&skill_path);
        let roots = collect_agent_targets(project_cwd.as_deref());
        uninstall_agent_skill_with_targets(target, &roots)
    })
    .await
    .map_err(|e| format!("卸载任务异常: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;

    #[test]
    fn scan_finds_same_skill_in_codex_and_antigravity() {
        let root = env::temp_dir().join(format!("lumina-skill-scan-{}-{}", std::process::id(), current_timestamp_millis()));
        let folder = "corpus-vercel-labs-native-skill-data-native-ui-4089298a";
        let codex = root.join(".codex").join("skills");
        let antigravity = root.join(".gemini").join("config").join("skills");
        for parent in [&codex, &antigravity] {
            let skill = parent.join(folder);
            fs::create_dir_all(&skill).unwrap();
            fs::write(skill.join("SKILL.md"), "---\nname: native-ui\ndescription: Native UI\n---\n").unwrap();
        }
        let skills = scan_agent_targets(vec![
            AgentTarget { agent_type: "codex", agent_label: "Codex", dir_path: codex },
            AgentTarget { agent_type: "antigravity", agent_label: "Antigravity", dir_path: antigravity },
        ]);
        assert_eq!(skills.len(), 2);
        assert!(skills.iter().all(|skill| skill.name == "native-ui" && skill.can_uninstall));
        assert!(skills.iter().any(|skill| skill.agent_type == "codex"));
        assert!(skills.iter().any(|skill| skill.agent_type == "antigravity"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scan_uses_managed_source_name_when_skill_file_is_unreadable() {
        let root = env::temp_dir().join(format!("lumina-skill-name-{}-{}", std::process::id(), current_timestamp_millis()));
        let skill_dir = root.join("corpus-vercel-labs-native-skill-data-native-ui-4089298a");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), [0xff, 0xfe, 0x00]).unwrap();
        let source = ManagedSkillSource {
            catalog_id: None,
            sub_skill_id: None,
            canonical_name: None,
            git_url: "https://github.com/vercel-labs/native".to_string(),
            sub_path: "skill-data/native-ui".to_string(),
            commit: "96943d4a680f27ea5bfa367c99db6cce58368d49".to_string(),
            content_sha256: "missing".to_string(),
        };
        fs::write(skill_dir.join(MANAGED_SOURCE_FILE), serde_json::to_vec(&source).unwrap()).unwrap();

        let skill = scan_single_skill_dir(&skill_dir, "codex", "Codex").unwrap();
        assert_eq!(skill.name, "native-ui");
        assert_eq!(skill.agent_type, "codex");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_history_keeps_source_and_detects_removed_target() {
        let root = env::temp_dir().join(format!("lumina-skill-history-{}-{}", std::process::id(), current_timestamp_millis()));
        fs::create_dir_all(&root).unwrap();
        let database = AppDatabase::from_path(root.join("history.db"));
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        let skill_path = root.join("native-ui");
        fs::create_dir_all(&skill_path).unwrap();
        fs::write(skill_path.join("SKILL.md"), "---\nname: native-ui\n---\n").unwrap();
        let args = InstallSkillArgs {
            skill_id: "catalog-native-ui".to_string(), git_url: "https://github.com/owner/repo".to_string(),
            target_mode: Some("specific_agent".to_string()), scope: None,
            target_agents: vec!["codex".to_string()], project_cwd: None, add_to_gitignore: false,
            sub_skill_id: None, sub_path: Some("skills/native-ui".to_string()),
            operation_id: Some("op-123".to_string()),
            item_key: Some("skill-1::codex".to_string()),
            retry_of_operation_id: None,
        };
        let path = skill_path.to_string_lossy().to_string();
        save_install_history(&database, &args, &Ok(InstallSkillResult {
            success: true,
            status: Some("success".to_string()),
            message: "安装成功".to_string(),
            installed_paths: vec![path.clone()],
            skipped_paths: Vec::new(),
            target_outcomes: Vec::new(),
        }), &[path.clone()]).unwrap();
        let entries = load_install_history(&database).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display_name, "native-ui");
        assert_eq!(entries[0].target_agents, ["codex"]);
        assert_eq!(entries[0].existing_paths, [path.clone()]);
        assert_eq!(entries[0].operation_id.as_deref(), Some("op-123"));
        assert_eq!(entries[0].item_key.as_deref(), Some("skill-1::codex"));
        save_install_history(&database, &args, &Err("第二个目标失败".to_string()), &[path.clone()]).unwrap();
        let entries = load_install_history(&database).unwrap();
        assert_eq!(entries[0].status, "partial");
        assert_eq!(entries[0].installed_paths, [path]);
        fs::remove_dir_all(&skill_path).unwrap();
        assert!(load_install_history(&database).unwrap()[0].existing_paths.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clone_progress_and_tls_errors_are_recognized() {
        assert_eq!(clone_percent("Receiving objects:  50% (10/20)"), Some(45));
        assert_eq!(clone_percent("Resolving deltas: 100% (2/2)"), Some(95));
        assert!(is_transient_git_network_error("TLS connect error: unexpected eof while reading"));
        assert_eq!(git_error_summary(b"Cloning into 'tmp'...\rReceiving objects: 10%\rfatal: unable to access URL: TLS connect error\n"), "fatal: unable to access URL: TLS connect error");
    }

    #[test]
    fn install_skill_result_serde_contract_alignment() {
        let result = InstallSkillResult {
            success: false,
            status: Some("partial".to_string()),
            message: "部分目标安装完成（新增 1 个，失败 1 个）：claude: 权限不足".to_string(),
            installed_paths: vec!["/home/user/.codex/skills/native-ui".to_string()],
            skipped_paths: Vec::new(),
            target_outcomes: vec![
                InstallTargetOutcome {
                    item_key: Some("catalog-native-ui::codex".to_string()),
                    target_agent: "codex".to_string(),
                    target_path: "/home/user/.codex/skills/native-ui".to_string(),
                    status: "installed".to_string(),
                    error: None,
                },
                InstallTargetOutcome {
                    item_key: Some("catalog-native-ui::claude".to_string()),
                    target_agent: "claude".to_string(),
                    target_path: "/home/user/.claude/skills".to_string(),
                    status: "failed".to_string(),
                    error: Some("权限不足".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"partial\""));
        assert!(json.contains("\"targetOutcomes\":["));
        assert!(json.contains("\"targetAgent\":\"codex\""));
        assert!(json.contains("\"status\":\"installed\""));
        assert!(json.contains("\"targetAgent\":\"claude\""));
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"error\":\"权限不足\""));

        let parsed: InstallSkillResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status.as_deref(), Some("partial"));
        assert_eq!(parsed.target_outcomes.len(), 2);
        assert_eq!(parsed.target_outcomes[0].status, "installed");
        assert_eq!(parsed.target_outcomes[1].status, "failed");
    }

    #[test]
    fn uninstall_only_accepts_direct_skill_children_of_approved_roots() {
        let root = std::env::temp_dir().join(format!(
            "lumina-uninstall-guard-{}-{}",
            std::process::id(), current_timestamp_millis()
        ));
        let skills_root = root.join(".agents").join("skills");
        let skill = skills_root.join("example");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: example\n---\n").unwrap();
        let targets = vec![AgentTarget {
            agent_type: "project",
            agent_label: "Project",
            dir_path: skills_root.clone(),
        }];
        assert!(validate_uninstall_target(&skill, &targets).is_ok());
        assert!(validate_uninstall_target(&skills_root, &targets).is_err());

        let unrelated = root.join("other-skills").join("example");
        fs::create_dir_all(&unrelated).unwrap();
        fs::write(unrelated.join("SKILL.md"), "skill").unwrap();
        assert!(validate_uninstall_target(&unrelated, &targets).is_err());

        fs::remove_file(skill.join("SKILL.md")).unwrap();
        assert!(validate_uninstall_target(&skill, &targets).is_err());
        fs::write(skill.join("SKILL.md"), "skill").unwrap();
        fs::write(
            root.join("skills-lock.json"),
            r#"{"version":1,"skills":{"example":{"source":"owner/repo"}}}"#,
        )
        .unwrap();
        assert!(is_skills_cli_managed(&skill));
        assert!(validate_uninstall_target(&skill, &targets).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn managed_copy_retains_source_and_detects_local_edits() {
        let root = std::env::temp_dir().join(format!(
            "lumina-managed-skill-test-{}-{}",
            std::process::id(), current_timestamp_millis()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("SKILL.md"), "---\nname: Example\ndescription: Example skill\n---\n").unwrap();
        let source = ManagedSkillSource {
            catalog_id: None,
            sub_skill_id: None,
            canonical_name: None,
            git_url: "https://github.com/owner/repo".to_string(),
            sub_path: "skills/example".to_string(),
            commit: "1234567890abcdef".to_string(),
            content_sha256: hash_skill_files(&root).unwrap(),
        };
        fs::write(root.join(MANAGED_SOURCE_FILE), serde_json::to_vec(&source).unwrap()).unwrap();
        let skill = scan_single_skill_dir(&root, "codex", "Codex").unwrap();
        assert!(skill.is_git_repo);
        assert_eq!(skill.current_commit.as_deref(), Some("1234567"));
        assert!(!skill.is_dirty);
        assert_eq!(skill.dirty, Some(false));
        assert_eq!(skill.management, "halowake");
        fs::write(root.join("SKILL.md"), "changed").unwrap();
        let modified = scan_single_skill_dir(&root, "codex", "Codex").unwrap();
        assert!(modified.is_dirty);
        assert_eq!(modified.dirty, Some(true));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_parse_skill_md() {
        let content = r#"---
name: impeccable
description: Use when the user wants to design or improve UI.
version: 4.0.4
---

# Impeccable
Some instructions here...
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("impeccable"));
        assert_eq!(
            desc.as_deref(),
            Some("Use when the user wants to design or improve UI.")
        );
        assert_eq!(ver.as_deref(), Some("4.0.4"));
    }

    #[test]
    fn test_parse_skill_md_multiline_description() {
        let content = r#"---
name: test-skill
description: >
  Line 1 of description
  Line 2 of description
version: 1.0.0
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("test-skill"));
        assert_eq!(
            desc.as_deref(),
            Some("Line 1 of description Line 2 of description")
        );
        assert_eq!(ver.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn test_parse_skill_md_cursor_block_folded_scalar() {
        let content = r#"---
name: autopilot
description: >-
  Keep a PR merge-ready by triaging comments, resolving clear conflicts, and
  fixing CI in a loop.
metadata:
  category: git
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("autopilot"));
        assert_eq!(
            desc.as_deref(),
            Some("Keep a PR merge-ready by triaging comments, resolving clear conflicts, and fixing CI in a loop.")
        );
        assert_eq!(ver, None);
    }

    #[test]
    fn test_parse_skill_md_cursor_single_block_scalar_before_end() {
        let content = r#"---
name: create-rule
description: >-
  Create Cursor rules for persistent AI guidance.
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("create-rule"));
        assert_eq!(
            desc.as_deref(),
            Some("Create Cursor rules for persistent AI guidance.")
        );
        assert_eq!(ver, None);
    }

    #[test]
    fn test_parse_skill_md_strips_stray_block_tokens() {
        let content = r#"---
name: empty-desc-skill
description: >-
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("empty-desc-skill"));
        assert_eq!(desc, None);
        assert_eq!(ver, None);
    }

    #[test]
    fn test_is_newer_semver() {
        assert!(is_newer_semver("4.5.0", "4.0.4"));
        assert!(is_newer_semver("4.1.0", "4.0.4"));
        assert!(is_newer_semver("v4.5.0", "4.0.4"));
        assert!(is_newer_semver("5.0.0", "4.99.99"));
        assert!(!is_newer_semver("4.0.4", "4.0.4"));
        assert!(!is_newer_semver("4.0.3", "4.0.4"));
        assert!(!is_newer_semver("4.0.4", "4.5.0"));
        assert!(!is_newer_semver("", "1.0.0"));
        assert!(is_newer_semver("1.0.0", ""));
    }

    #[test]
    fn test_detects_update_from_ecosystem_cache() {
        let temp_root = std::env::temp_dir().join(format!(
            "lumina-skill-update-test-{}-{}",
            std::process::id(), current_timestamp_millis()
        ));
        let skill_dir = temp_root.join(".skills").join("my-tool");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: my-tool\nversion: 1.0.0\ndescription: A tool\n---\n").unwrap();
        fs::write(skill_dir.join("update-check.json"), r#"{"latestVersion":"1.2.0"}"#).unwrap();

        let skills = check_agent_skills_updates_sync(Some(temp_root.to_str().unwrap()), None);
        let my_tool = skills.iter().find(|s| s.name == "my-tool");
        assert!(my_tool.is_some());
        let s = my_tool.unwrap();
        assert!(s.has_update);
        assert_eq!(s.update_status.as_deref(), Some("update_available"));
        assert_eq!(s.latest_version.as_deref(), Some("1.2.0"));
        assert!(s.update_summary.as_ref().unwrap().contains("1.2.0"));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_resolve_unmanaged_skill_source_for_impeccable() {
        let dummy_path = Path::new("dummy/impeccable");
        let res = resolve_unmanaged_skill_source(dummy_path, "impeccable", "impeccable", None);
        assert!(res.is_some());
        let (git_url, sub_path) = res.unwrap();
        assert_eq!(git_url, "https://github.com/pbakaus/impeccable.git");
        assert_eq!(sub_path.as_deref(), Some(".agents/skills/impeccable"));
    }

    fn create_prepared_skill_dir(
        parent: &Path,
        label: &str,
        skill_md: &str,
        git_url: &str,
        sub_path: &str,
        commit: &str,
    ) -> PathBuf {
        let prep = install_temp_path(parent, label);
        fs::create_dir_all(&prep).unwrap();
        fs::write(prep.join("SKILL.md"), skill_md).unwrap();
        let sha = hash_skill_files(&prep).unwrap();
        let source = ManagedSkillSource {
            catalog_id: None,
            sub_skill_id: None,
            canonical_name: None,
            git_url: git_url.to_string(),
            sub_path: sub_path.to_string(),
            commit: commit.to_string(),
            content_sha256: sha,
        };
        fs::write(
            prep.join(MANAGED_SOURCE_FILE),
            serde_json::to_vec_pretty(&source).unwrap(),
        )
        .unwrap();
        prep
    }

    #[test]
    fn phase1b_dual_identity_install_uses_canonical_name_and_preserves_catalog_id() {
        let root = env::temp_dir().join(format!(
            "lumina-phase1b-dual-id-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_parent = root.join(".agents").join("skills");
        fs::create_dir_all(&skills_parent).unwrap();

        let catalog_id = "corpus-vercel-labs-native-skill-data-native-ui-4089298a";
        let prep = create_prepared_skill_dir(
            &skills_parent,
            "prep-native-ui",
            "---\nname: native-ui\ndescription: Native UI helper\n---\n# Native UI\n",
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
            "11111112222222",
        );

        let (installed_dir, newly_installed) = commit_prepared_skill_to_parent(
            &prep,
            &skills_parent,
            catalog_id,
            catalog_id,
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
            false,
            false,
        )
        .unwrap();

        assert!(newly_installed);
        assert_eq!(installed_dir.file_name().unwrap().to_str().unwrap(), "native-ui");
        assert!(installed_dir.join("SKILL.md").is_file());

        let saved_source = read_managed_source(&installed_dir).unwrap();
        assert_eq!(saved_source.catalog_id.as_deref(), Some(catalog_id));
        assert_eq!(saved_source.canonical_name.as_deref(), Some("native-ui"));

        // Legacy `.lumina-skill-source.json` without catalogId/canonicalName must also deserialize cleanly
        let legacy_json = r#"{"gitUrl":"https://github.com/vercel-labs/native","subPath":"skill-data/native-ui","commit":"abcdef1","contentSha256":"deadbeef"}"#;
        let legacy_parsed: ManagedSkillSource = serde_json::from_str(legacy_json).unwrap();
        assert!(legacy_parsed.catalog_id.is_none());
        assert!(legacy_parsed.canonical_name.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn phase1b_cross_source_same_name_conflict_is_rejected_without_touching_existing_files() {
        let root = env::temp_dir().join(format!(
            "lumina-phase1b-conflict-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_parent = root.join(".agents").join("skills");
        fs::create_dir_all(&skills_parent).unwrap();

        let prep_a = create_prepared_skill_dir(
            &skills_parent,
            "prep-a",
            "---\nname: code-review\ndescription: Owner A Code Review\n---\nOwner A Content",
            "https://github.com/owner-a/skills",
            "skills/code-review",
            "aaaaaaa1111111",
        );
        let (target_a, installed_a) = commit_prepared_skill_to_parent(
            &prep_a,
            &skills_parent,
            "owner-a-code-review",
            "code-review",
            "https://github.com/owner-a/skills",
            "skills/code-review",
            false,
            false,
        )
        .unwrap();
        assert!(installed_a);

        // Attempt to install a different repository's skill with the same canonical name `code-review`
        let prep_b = create_prepared_skill_dir(
            &skills_parent,
            "prep-b",
            "---\nname: code-review\ndescription: Owner B Code Review\n---\nOwner B Content",
            "https://github.com/owner-b/other-skills",
            "skills/code-review",
            "bbbbbbb2222222",
        );
        let conflict_res = commit_prepared_skill_to_parent(
            &prep_b,
            &skills_parent,
            "owner-b-code-review",
            "code-review",
            "https://github.com/owner-b/other-skills",
            "skills/code-review",
            false,
            false,
        );
        assert!(conflict_res.is_err());
        let err_msg = conflict_res.unwrap_err();
        assert!(
            err_msg.contains("同名技能来源冲突"),
            "Expected conflict error, got: {err_msg}"
        );

        // Existing Owner A files must remain completely intact
        let existing_md = fs::read_to_string(target_a.join("SKILL.md")).unwrap();
        assert!(existing_md.contains("Owner A Content"));
        assert!(!prep_b.exists(), "Rejected prepared dir must be cleaned up");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn phase1b_concurrent_lock_contention_and_pid_reuse_defense() {
        let root = env::temp_dir().join(format!(
            "lumina-phase1b-lock-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        fs::create_dir_all(&root).unwrap();

        // 1. Live lock blocks concurrent lock acquisition on the same canonical skill name
        let guard = acquire_skill_op_lock(&root, "react-doctor", "install").unwrap();
        let second = acquire_skill_op_lock(&root, "react-doctor", "update");
        assert!(second.is_err());
        assert!(second.unwrap_err().contains("正在被另一个进程"));
        drop(guard);

        // 2. Simulate OS PID reuse: lock has our current live PID and a fresh timestamp,
        //    but a different `process_start_token` from a dead process that used to have this PID.
        let lock_path = root.join(format!("{OP_LOCK_PREFIX}react-doctor"));
        let fake_reused_pid_lock = SkillOpLock {
            pid: std::process::id(),
            process_start_token: 42, // Does not match current process creation FILETIME
            canonical_name: "react-doctor".to_string(),
            op: "install".to_string(),
            started_at_ms: current_timestamp_millis(),
        };
        fs::write(
            &lock_path,
            serde_json::to_vec_pretty(&fake_reused_pid_lock).unwrap(),
        )
        .unwrap();

        assert!(
            !is_lock_owner_alive(&fake_reused_pid_lock),
            "Lock with mismatched process_start_token must be detected as stale despite live PID"
        );

        // Acquiring the lock must succeed by reclaiming the stale PID-reused lock
        let reclaimed = acquire_skill_op_lock(&root, "react-doctor", "install");
        assert!(reclaimed.is_ok(), "Should reclaim stale PID-reused lock");
        drop(reclaimed);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn phase1b_crash_recovery_all_four_states_and_10x_idempotency_on_real_fs() {
        let root = env::temp_dir().join(format!(
            "lumina-phase1b-recovery-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        fs::create_dir_all(&root).unwrap();

        // Case 1: Interrupted during staging before backup (stage exists, target V1 exists)
        let c1_target = root.join("skill-c1");
        let c1_stage = root.join(format!("{STAGE_DIR_PREFIX}skill-c1"));
        fs::create_dir_all(&c1_target).unwrap();
        fs::write(c1_target.join("SKILL.md"), "---\nname: skill-c1\n---\nV1").unwrap();
        fs::create_dir_all(&c1_stage).unwrap();
        fs::write(c1_stage.join("SKILL.md"), "---\nname: skill-c1\n---\nV2-partial").unwrap();

        // Case 2: Interrupted after target -> backup, before stage -> target (target missing!)
        let c2_backup = root.join(format!("{BACKUP_DIR_PREFIX}skill-c2"));
        let c2_stage = root.join(format!("{STAGE_DIR_PREFIX}skill-c2"));
        fs::create_dir_all(&c2_backup).unwrap();
        fs::write(c2_backup.join("SKILL.md"), "---\nname: skill-c2\n---\nV1-intact").unwrap();
        fs::create_dir_all(&c2_stage).unwrap();
        fs::write(c2_stage.join("SKILL.md"), "---\nname: skill-c2\n---\nV2-uncommitted").unwrap();

        // Case 3: Interrupted after stage -> target succeeded, before backup cleanup
        let c3_target = root.join("skill-c3");
        let c3_backup = root.join(format!("{BACKUP_DIR_PREFIX}skill-c3"));
        fs::create_dir_all(&c3_target).unwrap();
        fs::write(c3_target.join("SKILL.md"), "---\nname: skill-c3\n---\nV2-complete").unwrap();
        fs::create_dir_all(&c3_backup).unwrap();
        fs::write(c3_backup.join("SKILL.md"), "---\nname: skill-c3\n---\nV1-old").unwrap();

        // Case 4: Active lock held by current live process (must NOT be touched by recovery!)
        let active_guard = acquire_skill_op_lock(&root, "skill-c4", "update").unwrap();
        let c4_stage = root.join(format!("{STAGE_DIR_PREFIX}skill-c4"));
        fs::create_dir_all(&c4_stage).unwrap();
        fs::write(c4_stage.join("SKILL.md"), "---\nname: skill-c4\n---\nin-flight").unwrap();

        for pass in 0..10 {
            let actions = recover_interrupted_skill_ops_in_dir(&root).unwrap();
            if pass == 0 {
                assert!(actions >= 3);
            } else {
                assert_eq!(actions, 0, "Pass {pass} must be a 0-op idempotent pass");
            }
        }

        // Verify Case 1: V1 preserved, stage removed
        assert!(fs::read_to_string(c1_target.join("SKILL.md")).unwrap().contains("V1"));
        assert!(!c1_stage.exists());

        // Verify Case 2: backup restored to target `skill-c2`, stage removed
        let c2_target = root.join("skill-c2");
        assert!(c2_target.exists());
        assert!(fs::read_to_string(c2_target.join("SKILL.md")).unwrap().contains("V1-intact"));
        assert!(!c2_backup.exists());
        assert!(!c2_stage.exists());

        // Verify Case 3: V2 target kept, old backup removed
        assert!(fs::read_to_string(c3_target.join("SKILL.md")).unwrap().contains("V2-complete"));
        assert!(!c3_backup.exists());

        // Verify Case 4: active in-flight stage dir is untouched while lock guard is alive
        assert!(c4_stage.exists());
        drop(active_guard);

        // Once active_guard is dropped, recovery cleans up Case 4 stage dir too
        recover_interrupted_skill_ops_in_dir(&root).unwrap();
        assert!(!c4_stage.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn phase1b_user_modified_files_protected_against_unforced_overwrite() {
        let root = env::temp_dir().join(format!(
            "lumina-phase1b-dirty-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_parent = root.join(".agents").join("skills");
        fs::create_dir_all(&skills_parent).unwrap();

        let prep_v1 = create_prepared_skill_dir(
            &skills_parent,
            "prep-v1",
            "---\nname: custom-linter\n---\nOriginal V1",
            "https://github.com/acme/linter",
            "skills/custom-linter",
            "1111111",
        );
        let (installed_dir, _) = commit_prepared_skill_to_parent(
            &prep_v1,
            &skills_parent,
            "acme-custom-linter",
            "custom-linter",
            "https://github.com/acme/linter",
            "skills/custom-linter",
            false,
            false,
        )
        .unwrap();

        // Simulate user modifying SKILL.md locally
        fs::write(
            installed_dir.join("SKILL.md"),
            "---\nname: custom-linter\n---\nUser Customized Rules",
        )
        .unwrap();

        // Attempt same-source update with force_replace_dirty = false
        let prep_v2 = create_prepared_skill_dir(
            &skills_parent,
            "prep-v2",
            "---\nname: custom-linter\n---\nUpstream V2",
            "https://github.com/acme/linter",
            "skills/custom-linter",
            "2222222",
        );
        let res = commit_prepared_skill_to_parent(
            &prep_v2,
            &skills_parent,
            "acme-custom-linter",
            "custom-linter",
            "https://github.com/acme/linter",
            "skills/custom-linter",
            true,
            false,
        );
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("技能文件已在本地修改"));

        // Verify user's local modification is still intact
        let current_md = fs::read_to_string(installed_dir.join("SKILL.md")).unwrap();
        assert!(current_md.contains("User Customized Rules"));

        // Now update with force_replace_dirty = true
        let prep_v2_forced = create_prepared_skill_dir(
            &skills_parent,
            "prep-v2-forced",
            "---\nname: custom-linter\n---\nUpstream V2",
            "https://github.com/acme/linter",
            "skills/custom-linter",
            "2222222",
        );
        let forced_res = commit_prepared_skill_to_parent(
            &prep_v2_forced,
            &skills_parent,
            "acme-custom-linter",
            "custom-linter",
            "https://github.com/acme/linter",
            "skills/custom-linter",
            true,
            true,
        );
        assert!(forced_res.is_ok());
        assert!(fs::read_to_string(installed_dir.join("SKILL.md")).unwrap().contains("Upstream V2"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn phase1b_rejects_path_traversal_and_invalid_canonical_names() {
        assert!(normalize_canonical_skill_name("../evil").is_err());
        assert!(normalize_canonical_skill_name("foo/bar").is_err());
        assert!(normalize_canonical_skill_name("foo\\bar").is_err());
        assert!(normalize_canonical_skill_name(".hidden").is_err());
        assert!(normalize_canonical_skill_name("").is_err());
        assert_eq!(
            normalize_canonical_skill_name("React_Doctor").unwrap(),
            "react-doctor"
        );
    }

    #[test]
    fn phase1b_legacy_catalog_id_directory_lifecycle_install_dedup_upgrade_uninstall() {
        let root = env::temp_dir().join(format!(
            "lumina-phase1b-legacy-compat-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_parent = root.join(".agents").join("skills");
        fs::create_dir_all(&skills_parent).unwrap();

        let catalog_id = "corpus-vercel-labs-native-skill-data-native-ui-4089298a";
        let legacy_dir = skills_parent.join(catalog_id);
        fs::create_dir_all(&legacy_dir).unwrap();
        let v1_md = "---\nname: native-ui\ndescription: Native UI v1\n---\n# Native UI v1";
        fs::write(legacy_dir.join("SKILL.md"), v1_md).unwrap();
        let v1_digest = hash_skill_files(&legacy_dir).unwrap();
        // Legacy `.lumina-skill-source.json` written by older Halowake (no catalog_id or canonical_name)
        let legacy_source = ManagedSkillSource {
            catalog_id: None,
            sub_skill_id: None,
            canonical_name: None,
            git_url: "https://github.com/vercel-labs/native".to_string(),
            sub_path: "skill-data/native-ui".to_string(),
            commit: "1111111a680f27ea5bfa367c99db6cce58368d49".to_string(),
            content_sha256: v1_digest,
        };
        fs::write(
            legacy_dir.join(MANAGED_SOURCE_FILE),
            serde_json::to_vec_pretty(&legacy_source).unwrap(),
        )
        .unwrap();

        // 1. Scan recognizes canonical name ("native-ui") AND legacy catalog_id
        let scanned = scan_single_skill_dir(&legacy_dir, "codex", "Codex").unwrap();
        assert_eq!(scanned.name, "native-ui");
        assert_eq!(scanned.catalog_id.as_deref(), Some(catalog_id));
        assert!(!scanned.is_dirty);

        // 2. Pre-install check recognizes the legacy directory before cloning
        let precheck = find_existing_installed_skill_in_parent(
            &skills_parent,
            catalog_id,
            "native-ui",
            None,
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
        )
        .unwrap();
        assert_eq!(precheck.as_deref(), Some(legacy_dir.as_path()));

        // 3. Re-installing the same skill does NOT create a duplicate `native-ui` directory
        let prep_dup = create_prepared_skill_dir(
            &skills_parent,
            "prep-dup",
            v1_md,
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
            "1111111a680f27ea5bfa367c99db6cce58368d49",
        );
        let (dup_target, newly_installed) = commit_prepared_skill_to_parent(
            &prep_dup,
            &skills_parent,
            catalog_id,
            "native-ui",
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
            false,
            false,
        )
        .unwrap();
        assert!(!newly_installed, "duplicate install must be skipped");
        assert_eq!(dup_target, legacy_dir, "must return existing legacy directory");
        assert!(
            !skills_parent.join("native-ui").exists(),
            "must NOT create duplicate canonical_name directory when legacy catalog_id directory is installed"
        );

        // 4. Cross-source conflict against legacy directory with same canonical_name is rejected
        let prep_conflict = create_prepared_skill_dir(
            &skills_parent,
            "prep-conflict",
            "---\nname: native-ui\ndescription: Fake Native UI\n---\n# Evil",
            "https://github.com/evil-org/fake-native",
            "skills/native-ui",
            "9999999",
        );
        let conflict_res = commit_prepared_skill_to_parent(
            &prep_conflict,
            &skills_parent,
            "evil-native-ui",
            "native-ui",
            "https://github.com/evil-org/fake-native",
            "skills/native-ui",
            false,
            false,
        );
        assert!(conflict_res.is_err());
        assert!(conflict_res.unwrap_err().contains("同名技能来源冲突"));
        assert!(!skills_parent.join("native-ui").exists());

        // 5. In-place upgrade updates the legacy directory without migrating directory name
        let v2_md = "---\nname: native-ui\ndescription: Native UI v2\n---\n# Native UI v2 Updated";
        let prep_v2 = create_prepared_skill_dir(
            &skills_parent,
            "prep-v2",
            v2_md,
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
            "2222222b680f27ea5bfa367c99db6cce58368d49",
        );
        let (upgraded_dir, replaced) = commit_prepared_skill_to_parent(
            &prep_v2,
            &skills_parent,
            catalog_id,
            "native-ui",
            "https://github.com/vercel-labs/native",
            "skill-data/native-ui",
            true,
            false,
        )
        .unwrap();
        assert!(replaced);
        assert_eq!(
            upgraded_dir, legacy_dir,
            "upgrade must keep legacy directory in-place without migrating user path"
        );
        assert!(!skills_parent.join("native-ui").exists());
        assert!(
            fs::read_to_string(legacy_dir.join("SKILL.md"))
                .unwrap()
                .contains("Native UI v2 Updated")
        );
        let upgraded_src = read_managed_source(&legacy_dir).unwrap();
        assert_eq!(upgraded_src.catalog_id.as_deref(), Some(catalog_id));
        assert_eq!(upgraded_src.canonical_name.as_deref(), Some("native-ui"));

        // 6. Uninstall legacy directory succeeds cleanly
        let roots = vec![AgentTarget {
            agent_type: "project",
            agent_label: "Project",
            dir_path: skills_parent.clone(),
        }];
        validate_uninstall_target(&legacy_dir, &roots).unwrap();
        fs::remove_dir_all(&legacy_dir).unwrap();
        assert!(!legacy_dir.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn t02_bundle_subskills_coexist_and_deduplicate_independently() {
        let root = env::temp_dir().join(format!(
            "lumina-t02-bundle-subskills-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_parent = root.join(".agents").join("skills");
        fs::create_dir_all(&skills_parent).unwrap();

        let bundle_id = "acme-ui-bundle";
        let repo_url = "https://github.com/acme/ui-bundle";

        // Install Sub-skill A
        let prep_a = create_prepared_skill_dir(
            &skills_parent,
            "prep-sub-a",
            "---\nname: design-tokens\ndescription: Sub-skill A\n---\nTokens",
            repo_url,
            "skills/design-tokens",
            "aaaa1111",
        );
        let (dir_a, installed_a) = commit_prepared_skill_to_parent(
            &prep_a,
            &skills_parent,
            bundle_id,
            "design-tokens",
            repo_url,
            "skills/design-tokens",
            false,
            false,
        )
        .unwrap();
        assert!(installed_a);
        assert_eq!(dir_a.file_name().unwrap().to_str().unwrap(), "design-tokens");

        // Install Sub-skill B from the same bundle_id and repo_url
        let prep_b = create_prepared_skill_dir(
            &skills_parent,
            "prep-sub-b",
            "---\nname: layout-grid\ndescription: Sub-skill B\n---\nGrid",
            repo_url,
            "skills/layout-grid",
            "aaaa1111",
        );
        let (dir_b, installed_b) = commit_prepared_skill_to_parent(
            &prep_b,
            &skills_parent,
            bundle_id,
            "layout-grid",
            repo_url,
            "skills/layout-grid",
            false,
            false,
        )
        .unwrap();
        assert!(
            installed_b,
            "Sub-skill B must NOT be deduplicated against Sub-skill A despite sharing catalog_id and git_url"
        );
        assert_eq!(dir_b.file_name().unwrap().to_str().unwrap(), "layout-grid");

        // Re-installing Sub-skill A skips only A
        let prep_a_dup = create_prepared_skill_dir(
            &skills_parent,
            "prep-sub-a-dup",
            "---\nname: design-tokens\ndescription: Sub-skill A\n---\nTokens",
            repo_url,
            "skills/design-tokens",
            "aaaa1111",
        );
        let (dir_a_dup, installed_a_dup) = commit_prepared_skill_to_parent(
            &prep_a_dup,
            &skills_parent,
            bundle_id,
            "design-tokens",
            repo_url,
            "skills/design-tokens",
            false,
            false,
        )
        .unwrap();
        assert!(!installed_a_dup);
        assert_eq!(dir_a_dup, dir_a);

        // Scan returns both sub-skills with distinct sub_path and sub_skill_id
        let scanned = scan_agent_targets(vec![AgentTarget {
            agent_type: "project",
            agent_label: "项目专属",
            dir_path: skills_parent.clone(),
        }]);
        assert_eq!(scanned.len(), 2);
        let scanned_a = scanned.iter().find(|s| s.name == "design-tokens").unwrap();
        let scanned_b = scanned.iter().find(|s| s.name == "layout-grid").unwrap();
        assert_eq!(scanned_a.sub_path.as_deref(), Some("skills/design-tokens"));
        assert_eq!(scanned_a.sub_skill_id.as_deref(), Some("design-tokens"));
        assert_eq!(scanned_b.sub_path.as_deref(), Some("skills/layout-grid"));
        assert_eq!(scanned_b.sub_skill_id.as_deref(), Some("layout-grid"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn t02_sub_path_normalization_preserves_case_and_rejects_traversal() {
        assert_eq!(
            normalize_skill_sub_path(" ./skills\\ReactDoctor/ ").unwrap(),
            "skills/ReactDoctor"
        );
        assert_eq!(normalize_skill_sub_path("").unwrap(), "");
        assert!(normalize_skill_sub_path("../evil").is_err());
        assert!(normalize_skill_sub_path("skills/../../etc/passwd").is_err());
        assert!(normalize_skill_sub_path("/abs/path").is_err());
        assert!(normalize_skill_sub_path("\\abs\\path").is_err());
        assert!(normalize_skill_sub_path("C:/skills/foo").is_err());
    }

    #[test]
    fn t03_unmanaged_skill_rejects_auto_update_and_preserves_custom_files() {
        let root = env::temp_dir().join(format!(
            "lumina-t03-unmanaged-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skill_dir = root.join(".agents").join("skills").join("impeccable");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: impeccable\ndescription: User local skill\n---\nUser custom content",
        )
        .unwrap();
        fs::write(skill_dir.join("user-notes.txt"), "Do not delete me").unwrap();

        let scanned = scan_single_skill_dir(&skill_dir, "codex", "Codex").unwrap();
        assert_eq!(scanned.management, "unmanaged");
        assert_eq!(scanned.dirty, None);
        assert_eq!(scanned.source.kind, "unknown");

        // Both ordinary and forced auto-update must reject unmanaged local directory
        let res_normal = update_agent_skill_sync(skill_dir.to_str().unwrap(), false, None);
        assert!(res_normal.is_err());
        assert!(res_normal.unwrap_err().contains("本地未受管技能"));

        let res_forced = update_agent_skill_sync(skill_dir.to_str().unwrap(), true, None);
        assert!(res_forced.is_err());
        assert!(res_forced.unwrap_err().contains("本地未受管技能"));

        // Verify user files are completely untouched
        assert_eq!(
            fs::read_to_string(skill_dir.join("user-notes.txt")).unwrap(),
            "Do not delete me"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn t03_uninstall_and_update_share_exclusive_operation_lock() {
        let root = env::temp_dir().join(format!(
            "lumina-t03-uninstall-lock-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_root = root.join(".agents").join("skills");
        let skill_dir = skills_root.join("locked-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: locked-skill\n---\n").unwrap();

        let targets = vec![AgentTarget {
            agent_type: "project",
            agent_label: "Project",
            dir_path: skills_root.clone(),
        }];

        // Hold an active update lock on `locked-skill`
        let update_guard = acquire_skill_op_lock(&skills_root, "locked-skill", "update").unwrap();

        let blocked = uninstall_agent_skill_with_targets(&skill_dir, &targets);
        assert!(blocked.is_err());
        assert!(blocked.unwrap_err().contains("正在被另一个进程"));
        assert!(skill_dir.exists(), "Skill dir must remain intact while locked");

        drop(update_guard);

        let unblocked = uninstall_agent_skill_with_targets(&skill_dir, &targets);
        assert!(unblocked.is_ok());
        assert!(!skill_dir.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn t03_check_updates_deduplicates_repo_and_distinguishes_subskill_and_offline() {
        let root = env::temp_dir().join(format!(
            "lumina-t03-check-updates-{}-{}",
            std::process::id(),
            current_timestamp_millis()
        ));
        let skills_root = root.join(".agents").join("skills");
        fs::create_dir_all(&skills_root).unwrap();

        let prep_sub1 = create_prepared_skill_dir(
            &skills_root,
            "prep-s1",
            "---\nname: sub-one\n---\nOne",
            "https://github.com/acme/mono",
            "skills/sub-one",
            "1111111111111111",
        );
        let ( _dir1, _ ) = commit_prepared_skill_to_parent(
            &prep_sub1,
            &skills_root,
            "acme-mono",
            "sub-one",
            "https://github.com/acme/mono",
            "skills/sub-one",
            false,
            false,
        )
        .unwrap();

        let prep_sub2 = create_prepared_skill_dir(
            &skills_root,
            "prep-s2",
            "---\nname: sub-two\n---\nTwo",
            "https://github.com/acme/mono",
            "skills/sub-two",
            "1111111111111111",
        );
        let ( _dir2, _ ) = commit_prepared_skill_to_parent(
            &prep_sub2,
            &skills_root,
            "acme-mono",
            "sub-two",
            "https://github.com/acme/mono",
            "skills/sub-two",
            false,
            false,
        )
        .unwrap();

        let scanned = scan_agent_targets(vec![AgentTarget {
            agent_type: "project",
            agent_label: "Project",
            dir_path: skills_root.clone(),
        }]);
        assert_eq!(scanned.len(), 2);

        let mut call_count = 0usize;
        let checked = check_agent_skills_updates_with_head_resolver(scanned.clone(), |_url| {
            call_count += 1;
            Ok("2222222222222222".to_string())
        });
        assert_eq!(call_count, 1, "Same repository must only be queried once per update check");
        assert!(checked.iter().all(|s| {
            s.has_update
                && s.update_status.as_deref() == Some("update_available")
                && s.update_summary.as_deref() == Some("来源仓库有新提交（子技能目录未必变更）")
        }));

        // Simulate offline/timeout failure
        let offline = check_agent_skills_updates_with_head_resolver(scanned, |_url| {
            Err("执行命令超时 (25s): git ls-remote".to_string())
        });
        assert!(offline.iter().all(|s| {
            !s.has_update
                && s.update_status.as_deref() == Some("error")
                && s.update_error.as_deref().unwrap_or_default().contains("超时")
        }));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn t03_command_timeout_kills_hanging_process() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("ping");
            c.args(["-n", "6", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("5");
            c
        };
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let res = run_command_with_timeout(cmd, Duration::from_millis(80), "hanging-test-cmd");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("执行命令超时"));
    }
}
