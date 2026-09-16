use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use rusqlite::{Connection, OpenFlags};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexReportQuery {
    pub from: String,
    pub to: String,
    pub providers: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CodexProjectInfo {
    pub name: String,
    pub cwd: String,
    pub session_count: usize,
    pub last_active_at: Option<String>,
    pub provider: Option<String>,
    pub providers: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CodexReportSession {
    pub id: String,
    pub provider: String,
    pub started_at: String,
    pub ended_at: String,
    pub cwd: Option<String>,
    pub project_name: String,
    pub user_messages: Vec<String>,
    pub assistant_messages: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledToolInfo {
    pub provider: String,
    pub name: String,
    pub is_installed: bool,
    pub session_count: usize,
}

#[derive(Debug, Default)]
struct SessionDraft {
    id: String,
    provider: String,
    started_at: String,
    ended_at: String,
    cwd: Option<String>,
    project_name: String,
    user_messages: Vec<String>,
    assistant_messages: Vec<String>,
}

fn get_home_dir() -> Result<PathBuf, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|_| "无法定位当前用户目录".to_string())?;
    Ok(PathBuf::from(home))
}

fn get_appdata_dir() -> Option<PathBuf> {
    env::var("APPDATA").ok().map(PathBuf::from)
}

fn get_local_appdata_dir() -> Option<PathBuf> {
    env::var("LOCALAPPDATA").ok().map(PathBuf::from)
}

#[tauri::command]
pub async fn detect_installed_ai_tools() -> Result<Vec<InstalledToolInfo>, String> {
    tokio::task::spawn_blocking(detect_installed_ai_tools_sync)
        .await
        .map_err(|e| format!("执行检测任务失败: {}", e))?
}

fn detect_installed_ai_tools_sync() -> Result<Vec<InstalledToolInfo>, String> {
    let home = get_home_dir()?;
    let mut tools = Vec::new();

    // 1. Codex
    let codex_dir = home.join(".codex").join("sessions");
    let codex_installed = codex_dir.exists();
    let mut codex_count = 0;
    if codex_installed {
        let mut codex_files = Vec::new();
        let _ = collect_jsonl_files(&codex_dir, &mut codex_files);
        codex_count = codex_files.len();
    }
    tools.push(InstalledToolInfo {
        provider: "codex".to_string(),
        name: "Codex CLI".to_string(),
        is_installed: codex_installed,
        session_count: codex_count,
    });

    // 2. Claude Code
    let claude_dir = home.join(".claude");
    let claude_projects = claude_dir.join("projects");
    let claude_installed = claude_dir.exists();
    let mut claude_count = 0;
    if claude_projects.exists() {
        let mut claude_files = Vec::new();
        let _ = collect_jsonl_files(&claude_projects, &mut claude_files);
        claude_count = claude_files.len();
    }
    tools.push(InstalledToolInfo {
        provider: "claude".to_string(),
        name: "Claude Code".to_string(),
        is_installed: claude_installed,
        session_count: claude_count,
    });

    // 3. Antigravity
    let agy_dir = home.join(".gemini").join("antigravity");
    let agy_installed = agy_dir.exists();
    let mut agy_count = 0;
    if agy_installed {
        let brain_dir = agy_dir.join("brain");
        if brain_dir.exists() {
            if let Ok(entries) = fs::read_dir(brain_dir) {
                agy_count = entries.filter_map(Result::ok).filter(|e| e.path().is_dir()).count();
            }
        }
    }
    tools.push(InstalledToolInfo {
        provider: "antigravity".to_string(),
        name: "Antigravity".to_string(),
        is_installed: agy_installed,
        session_count: agy_count,
    });

    // 4. OpenCode
    let mut opencode_installed = false;
    let mut opencode_count = 0;
    let opencode_candidates = [
        home.join(".local").join("share").join("opencode").join("storage"),
        home.join(".opencode").join("storage"),
        get_appdata_dir().unwrap_or_default().join("opencode").join("storage"),
        get_local_appdata_dir().unwrap_or_default().join("opencode").join("storage"),
    ];
    for dir in &opencode_candidates {
        if dir.exists() {
            opencode_installed = true;
            if let Ok(entries) = fs::read_dir(dir) {
                opencode_count = entries.filter_map(Result::ok).count();
            }
            break;
        }
    }
    tools.push(InstalledToolInfo {
        provider: "opencode".to_string(),
        name: "OpenCode".to_string(),
        is_installed: opencode_installed,
        session_count: opencode_count,
    });

    Ok(tools)
}

#[derive(Debug)]
struct ProjectAccumulator {
    name: String,
    cwd: String,
    session_count: usize,
    last_active_at: Option<String>,
    providers: Vec<String>,
}

fn record_project_activity(
    project_map: &mut HashMap<String, ProjectAccumulator>,
    cwd_raw: &str,
    name_hint: Option<&str>,
    timestamp: Option<String>,
    provider: &str,
    count: usize,
) {
    let normalized = normalize_project_path(cwd_raw);
    if normalized.is_empty() {
        return;
    }
    let key = normalized.to_lowercase();
    let name = if let Some(hint) = name_hint {
        if !hint.is_empty() && hint != "Outside of Project" {
            hint.to_string()
        } else {
            get_project_name(&normalized)
        }
    } else {
        get_project_name(&normalized)
    };

    let entry = project_map.entry(key).or_insert_with(|| ProjectAccumulator {
        name,
        cwd: normalized.clone(),
        session_count: 0,
        last_active_at: None,
        providers: Vec::new(),
    });

    entry.session_count += count;
    if !entry.providers.iter().any(|p| p == provider) {
        entry.providers.push(provider.to_string());
    }
    if let Some(ts) = timestamp {
        if entry.last_active_at.as_ref().map_or(true, |existing| &ts > existing) {
            entry.last_active_at = Some(ts);
        }
    }
}

#[tauri::command]
pub async fn load_codex_projects() -> Result<Vec<CodexProjectInfo>, String> {
    tokio::task::spawn_blocking(load_codex_projects_sync)
        .await
        .map_err(|e| format!("执行加载项目任务失败: {}", e))?
}

fn load_codex_projects_sync() -> Result<Vec<CodexProjectInfo>, String> {
    let home = get_home_dir()?;
    let mut project_map: HashMap<String, ProjectAccumulator> = HashMap::new();

    // 1. Codex Projects
    let codex_dir = home.join(".codex").join("sessions");
    if codex_dir.exists() {
        let mut session_files = Vec::new();
        let _ = collect_jsonl_files(&codex_dir, &mut session_files);
        for path in &session_files {
            if let Some((cwd, timestamp)) = extract_codex_session_meta(path) {
                record_project_activity(&mut project_map, &cwd, None, timestamp, "codex", 1);
            }
        }
    }

    // 2. Claude Code Projects
    let claude_projects_dir = home.join(".claude").join("projects");
    if claude_projects_dir.exists() {
        if let Ok(entries) = fs::read_dir(&claude_projects_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                    let cwd = decode_claude_project_path(&folder_name);
                    let mut session_files = Vec::new();
                    let _ = collect_jsonl_files(&path, &mut session_files);
                    let count = session_files.len();
                    if count > 0 {
                        record_project_activity(&mut project_map, &cwd, None, None, "claude", count);
                    }
                }
            }
        }
    }

    // 3. Antigravity Projects
    let (_project_id_map, project_mappings, _conv_to_project, project_stats) = load_antigravity_metadata(&home);
    for (name, cwd) in &project_mappings {
        let (count, last_active) = if let Some((cnt, ts)) = project_stats.get(cwd) {
            (*cnt, ts.clone())
        } else {
            (1, None)
        };
        record_project_activity(&mut project_map, cwd, Some(name), last_active, "antigravity", count.max(1));
    }

    let mut projects: Vec<CodexProjectInfo> = project_map
        .into_values()
        .map(|acc| {
            let provider = if acc.providers.len() == 1 {
                acc.providers.first().cloned()
            } else if acc.providers.is_empty() {
                None
            } else {
                Some("multi".to_string())
            };
            CodexProjectInfo {
                name: acc.name,
                cwd: acc.cwd,
                session_count: acc.session_count,
                last_active_at: acc.last_active_at,
                provider,
                providers: acc.providers,
            }
        })
        .collect();

    projects.sort_by(|a, b| {
        b.last_active_at
            .cmp(&a.last_active_at)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(projects)
}

fn days_from_civil(year: i64, month: u64, day: u64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

pub fn parse_iso_timestamp(iso: &str) -> Option<u64> {
    if iso.len() < 19 {
        return None;
    }
    let year: i64 = iso.get(0..4)?.parse().ok()?;
    let month: u64 = iso.get(5..7)?.parse().ok()?;
    let day: u64 = iso.get(8..10)?.parse().ok()?;
    let hour: u64 = iso.get(11..13)?.parse().ok()?;
    let min: u64 = iso.get(14..16)?.parse().ok()?;
    let sec: u64 = iso.get(17..19)?.parse().ok()?;

    if month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }

    let days = days_from_civil(year, month, day);
    if days < 0 {
        return None;
    }
    let mut total_secs = (days as u64) * 86400 + hour * 3600 + min * 60 + sec;

    if let Some(tz_part) = iso.get(19..) {
        if let Some(plus_idx) = tz_part.find('+') {
            if let Some(offset_str) = tz_part.get(plus_idx + 1..) {
                if offset_str.len() >= 5 {
                    let oh: u64 = offset_str.get(0..2).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let om: u64 = offset_str.get(3..5).and_then(|s| s.parse().ok()).unwrap_or(0);
                    total_secs = total_secs.saturating_sub(oh * 3600 + om * 60);
                }
            }
        } else if let Some(minus_idx) = tz_part.find('-') {
            if let Some(offset_str) = tz_part.get(minus_idx + 1..) {
                if offset_str.len() >= 5 {
                    let oh: u64 = offset_str.get(0..2).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let om: u64 = offset_str.get(3..5).and_then(|s| s.parse().ok()).unwrap_or(0);
                    total_secs = total_secs.saturating_add(oh * 3600 + om * 60);
                }
            }
        }
    }
    Some(total_secs)
}

fn is_file_modified_since(path: &Path, min_epoch: u64) -> bool {
    if min_epoch == 0 {
        return true;
    }
    match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(mtime) => {
            let secs = mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(u64::MAX);
            secs >= min_epoch
        }
        Err(_) => true,
    }
}

fn fast_extract_timestamp(line: &str) -> Option<&str> {
    for prefix in &["\"created_at\":\"", "\"timestamp\":\""] {
        if let Some(pos) = line.find(prefix) {
            let start = pos + prefix.len();
            if let Some(end) = line[start..].find('"') {
                return Some(&line[start..start + end]);
            }
        }
    }
    None
}

fn parse_items_parallel<T, F>(items: Vec<T>, parser: F) -> Vec<CodexReportSession>
where
    T: Send + Sync,
    F: Fn(&T) -> Option<CodexReportSession> + Send + Sync,
{
    if items.is_empty() {
        return Vec::new();
    }
    let num_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
    if items.len() <= 2 || num_threads <= 1 {
        return items.iter().filter_map(&parser).collect();
    }

    let chunk_size = (items.len() + num_threads - 1) / num_threads;
    std::thread::scope(|s| {
        let handles: Vec<_> = items
            .chunks(chunk_size)
            .map(|chunk| {
                s.spawn(|| {
                    chunk.iter().filter_map(&parser).collect::<Vec<_>>()
                })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(sub) = handle.join() {
                results.extend(sub);
            }
        }
        results
    })
}

#[tauri::command]
pub async fn load_codex_report_sessions(query: CodexReportQuery) -> Result<Vec<CodexReportSession>, String> {
    tokio::task::spawn_blocking(move || load_codex_report_sessions_sync(query))
        .await
        .map_err(|e| format!("执行加载会话任务失败: {}", e))?
}

fn load_codex_report_sessions_sync(query: CodexReportQuery) -> Result<Vec<CodexReportSession>, String> {
    let home = get_home_dir()?;
    let selected_providers = query.providers.clone().unwrap_or_else(|| {
        vec![
            "codex".to_string(),
            "claude".to_string(),
            "antigravity".to_string(),
            "opencode".to_string(),
        ]
    });

    let mut from_str = query.from;
    let mut to_str = query.to;
    if from_str.len() == 10 {
        from_str = format!("{}T00:00:00.000Z", from_str);
    }
    if to_str.len() == 10 {
        to_str = format!("{}T23:59:59.999Z", to_str);
    }

    let to_date = if to_str.len() >= 10 { &to_str[..10] } else { &to_str };
    // Add 5 minutes grace period before query.from for mtime check
    let min_epoch = parse_iso_timestamp(&from_str).map(|s| s.saturating_sub(300)).unwrap_or(0);

    let mut all_sessions = Vec::new();

    // 1. Codex (with date directory pruning and mtime filter)
    if selected_providers.iter().any(|p| p == "codex" || p == "all") {
        let codex_dir = home.join(".codex").join("sessions");
        if codex_dir.exists() {
            let mut session_files = Vec::new();
            collect_codex_jsonl_files_pruned(&codex_dir, to_date, &mut session_files);
            let sessions = parse_items_parallel(session_files, |path| {
                parse_codex_session(path, &from_str, &to_str)
            });
            all_sessions.extend(sessions);
        }
    }

    // 2. Claude Code
    if selected_providers.iter().any(|p| p == "claude" || p == "all") {
        let claude_projects_dir = home.join(".claude").join("projects");
        if claude_projects_dir.exists() {
            let mut claude_files = Vec::new();
            let _ = collect_jsonl_files(&claude_projects_dir, &mut claude_files);
            let filtered_claude: Vec<PathBuf> = claude_files
                .into_iter()
                .filter(|p| is_file_modified_since(p, min_epoch))
                .collect();
            let sessions = parse_items_parallel(filtered_claude, |path| {
                parse_claude_session(path, &from_str, &to_str)
            });
            all_sessions.extend(sessions);
        }
    }

    // 3. Antigravity (with mtime check & fast streaming reader)
    if selected_providers.iter().any(|p| p == "antigravity" || p == "all") {
        let brain_dir = home.join(".gemini").join("antigravity").join("brain");
        if brain_dir.exists() {
            let (_project_id_map, project_mappings, conv_to_project, _project_stats) = load_antigravity_metadata(&home);
            let mut agy_candidates = Vec::new();
            if let Ok(entries) = fs::read_dir(&brain_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let conv_dir = entry.path();
                    if conv_dir.is_dir() {
                        let conv_id = conv_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                        let transcript_file = conv_dir.join(".system_generated").join("logs").join("transcript.jsonl");
                        if transcript_file.exists() && is_file_modified_since(&transcript_file, min_epoch) {
                            agy_candidates.push((transcript_file, conv_id));
                        }
                    }
                }
            }
            let sessions = parse_items_parallel(agy_candidates, |(transcript_file, conv_id)| {
                parse_antigravity_transcript(
                    transcript_file,
                    conv_id,
                    &conv_to_project,
                    &project_mappings,
                    &from_str,
                    &to_str,
                )
            });
            all_sessions.extend(sessions);
        }
    }

    // 4. OpenCode
    if selected_providers.iter().any(|p| p == "opencode" || p == "all") {
        let opencode_candidates = [
            home.join(".local").join("share").join("opencode").join("storage"),
            home.join(".opencode").join("storage"),
            get_appdata_dir().unwrap_or_default().join("opencode").join("storage"),
            get_local_appdata_dir().unwrap_or_default().join("opencode").join("storage"),
        ];
        let mut opencode_files = Vec::new();
        for storage_dir in &opencode_candidates {
            if storage_dir.exists() {
                if let Ok(entries) = fs::read_dir(storage_dir) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.extension().is_some_and(|ext| ext == "json") {
                            if is_file_modified_since(&path, min_epoch) {
                                opencode_files.push(path);
                            }
                        }
                    }
                }
            }
        }
        let sessions = parse_items_parallel(opencode_files, |path| {
            parse_opencode_session(path, &from_str, &to_str)
        });
        all_sessions.extend(sessions);
    }

    all_sessions.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    Ok(all_sessions)
}

/// Prune Codex directories based on YYYY/MM/DD structure for future dates only
fn collect_codex_jsonl_files_pruned(codex_dir: &Path, to_date: &str, files: &mut Vec<PathBuf>) {
    let to_year = if to_date.len() >= 4 { &to_date[..4] } else { "9999" };
    let to_ym = if to_date.len() >= 7 { &to_date[..7] } else { "9999-12" };

    let Ok(year_entries) = fs::read_dir(codex_dir) else { return };
    for year_entry in year_entries.filter_map(Result::ok) {
        let year_path = year_entry.path();
        if !year_path.is_dir() { continue; }
        let Some(year_str) = year_path.file_name().and_then(|n| n.to_str()) else { continue; };
        if year_str > to_year { continue; }

        let Ok(month_entries) = fs::read_dir(&year_path) else { continue };
        for month_entry in month_entries.filter_map(Result::ok) {
            let month_path = month_entry.path();
            if !month_path.is_dir() { continue; }
            let Some(month_str) = month_path.file_name().and_then(|n| n.to_str()) else { continue; };
            let current_ym = format!("{}-{}", year_str, month_str);
            if current_ym.as_str() > to_ym { continue; }

            let Ok(day_entries) = fs::read_dir(&month_path) else { continue };
            for day_entry in day_entries.filter_map(Result::ok) {
                let day_path = day_entry.path();
                if !day_path.is_dir() { continue; }
                let Some(day_str) = day_path.file_name().and_then(|n| n.to_str()) else { continue; };
                let current_ymd = format!("{}-{}-{}", year_str, month_str, day_str);
                if current_ymd.as_str() > to_date { continue; }

                if let Ok(entries) = fs::read_dir(&day_path) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.extension().is_some_and(|ext| ext == "jsonl") {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }
}

fn collect_jsonl_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("读取目录失败 {}: {}", directory.display(), error))?
    {
        let path = entry
            .map_err(|error| format!("读取条目失败: {}", error))?
            .path();
        if path.is_dir() {
            let _ = collect_jsonl_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }
    Ok(())
}

fn extract_codex_session_meta(path: &Path) -> Option<(String, Option<String>)> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(15) {
        let line = line.ok()?;
        if !line.contains("\"session_meta\"") {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<Value>(&line) {
            if entry.get("type").and_then(Value::as_str) == Some("session_meta") {
                let payload = entry.get("payload");
                let cwd = payload
                    .and_then(|p| p.get("cwd"))
                    .and_then(Value::as_str)?
                    .trim();
                if cwd.is_empty() {
                    return None;
                }
                let timestamp = entry.get("timestamp").and_then(Value::as_str).map(str::to_string);
                return Some((cwd.to_string(), timestamp));
            }
        }
    }
    None
}

fn parse_codex_session(path: &Path, from: &str, to: &str) -> Option<CodexReportSession> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let default_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            if s.len() > 36 {
                s[s.len() - 36..].to_string()
            } else {
                s.to_string()
            }
        })
        .unwrap_or_default();

    let mut draft = SessionDraft {
        id: default_id,
        provider: "codex".to_string(),
        ..Default::default()
    };

    for line_res in reader.lines() {
        let Ok(line) = line_res else { continue; };
        // Fast string pre-filter to avoid unnecessary serde deserialization
        let is_meta = line.contains("\"session_meta\"");
        if !is_meta
            && !line.contains("\"user\"")
            && !line.contains("\"user_message\"")
            && !line.contains("\"final_answer\"")
            && !line.contains("\"assistant\"")
        {
            continue;
        }

        if !is_meta {
            if let Some(ts) = fast_extract_timestamp(&line) {
                if !is_in_range(ts, from, to) {
                    continue;
                }
            }
        }

        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let timestamp = entry.get("timestamp").and_then(Value::as_str).unwrap_or_default();
        let entry_type = entry.get("type").and_then(Value::as_str).unwrap_or_default();
        let payload = entry.get("payload").unwrap_or(&Value::Null);

        if entry_type == "session_meta" {
            let sid = payload
                .get("session_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str);
            if let Some(id) = sid {
                if !id.is_empty() {
                    draft.id = id.to_string();
                }
            }
            if let Some(cwd) = payload.get("cwd").and_then(Value::as_str) {
                if !cwd.is_empty() {
                    let norm_cwd = normalize_project_path(cwd);
                    draft.project_name = get_project_name(&norm_cwd);
                    draft.cwd = Some(norm_cwd);
                }
            }
            continue;
        }

        if !is_in_range(timestamp, from, to) {
            continue;
        }

        let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or_default();
        let role = payload
            .get("role")
            .or_else(|| entry.get("role"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        // 1. User messages
        if (entry_type == "event_msg" && payload_type == "user_message") || role == "user" {
            let text = payload
                .get("message")
                .and_then(extract_text_value)
                .or_else(|| payload.get("content").and_then(extract_text_value))
                .or_else(|| extract_text_value(payload));
            if let Some(cleaned) = text.and_then(|s| clean_user_message(&s)) {
                update_bounds(&mut draft, timestamp);
                draft.user_messages.push(cleaned);
            }
        }
        // 2. Assistant messages
        else if role == "assistant" {
            let phase = payload.get("phase").and_then(Value::as_str);
            if phase == Some("final_answer") || phase.is_none() {
                let text = payload
                    .get("content")
                    .and_then(extract_text_value)
                    .or_else(|| payload.get("message").and_then(extract_text_value))
                    .or_else(|| extract_text_value(payload));
                if let Some(cleaned) = text.and_then(|s| clean_result_message(&s)) {
                    update_bounds(&mut draft, timestamp);
                    draft.assistant_messages.push(cleaned);
                }
            }
        }
    }

    finalize_draft(draft, from, to)
}

fn parse_claude_session(path: &Path, from: &str, to: &str) -> Option<CodexReportSession> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let session_id = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    let parent_name = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or_default();
    let inferred_cwd = decode_claude_project_path(parent_name);

    let mut draft = SessionDraft {
        id: session_id,
        provider: "claude".to_string(),
        cwd: if inferred_cwd.is_empty() { None } else { Some(inferred_cwd) },
        ..Default::default()
    };

    for line_res in reader.lines() {
        let Ok(line) = line_res else { continue; };
        if !line.contains("\"user\"") && !line.contains("\"assistant\"") {
            continue;
        }

        if let Some(ts) = fast_extract_timestamp(&line) {
            if !is_in_range(ts, from, to) {
                continue;
            }
        }

        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let timestamp = entry.get("timestamp")
            .or_else(|| entry.get("created_at"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        if !timestamp.is_empty() && !is_in_range(timestamp, from, to) {
            continue;
        }

        if !timestamp.is_empty() {
            update_bounds(&mut draft, timestamp);
        }

        let msg_type = entry.get("type").and_then(Value::as_str).unwrap_or_default();
        let role = entry.get("role").or_else(|| entry.pointer("/message/role")).and_then(Value::as_str).unwrap_or_default();

        if msg_type == "user" || role == "user" {
            let text = entry.get("message")
                .or_else(|| entry.get("content"))
                .or_else(|| entry.pointer("/message/content"))
                .and_then(extract_text_value);
            if let Some(t) = text.and_then(|s| clean_user_message(&s)) {
                draft.user_messages.push(t);
            }
        } else if msg_type == "assistant" || role == "assistant" {
            let text = entry.get("message")
                .or_else(|| entry.get("content"))
                .or_else(|| entry.pointer("/message/content"))
                .and_then(extract_text_value);
            if let Some(t) = text.and_then(|s| clean_result_message(&s)) {
                draft.assistant_messages.push(t);
            }
        }
    }

    finalize_draft(draft, from, to)
}

fn parse_antigravity_transcript(
    path: &Path,
    conv_id: &str,
    conv_project_map: &HashMap<String, (String, String)>,
    project_mappings: &[(String, String)],
    from: &str,
    to: &str,
) -> Option<CodexReportSession> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut draft = SessionDraft {
        id: conv_id.to_string(),
        provider: "antigravity".to_string(),
        ..Default::default()
    };

    let (mut inferred_project_name, mut inferred_cwd) = if let Some((pname, pcwd)) = conv_project_map.get(conv_id) {
        (Some(pname.clone()), Some(pcwd.clone()))
    } else {
        (None, None)
    };

    for line_res in reader.lines() {
        let Ok(line) = line_res else { continue; };

        // Fast skip lines that are not user input or planner response
        if !line.contains("\"USER_INPUT\"") && !line.contains("\"PLANNER_RESPONSE\"") {
            continue;
        }

        if let Some(ts) = fast_extract_timestamp(&line) {
            if !is_in_range(ts, from, to) {
                continue;
            }
        }

        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let timestamp = entry.get("created_at")
            .or_else(|| entry.get("timestamp"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        if !timestamp.is_empty() && !is_in_range(timestamp, from, to) {
            continue;
        }

        if !timestamp.is_empty() {
            update_bounds(&mut draft, timestamp);
        }

        let entry_type = entry.get("type").and_then(Value::as_str).unwrap_or_default();

        if entry_type == "USER_INPUT" {
            if let Some(content_str) = entry.get("content").and_then(Value::as_str) {
                let user_request = extract_agy_user_request(content_str);
                if let Some(cleaned) = clean_user_message(&user_request) {
                    draft.user_messages.push(cleaned);
                }
            }
        } else if entry_type == "PLANNER_RESPONSE" {
            if inferred_cwd.is_none() {
                if let Some(tool_calls) = entry.get("tool_calls").and_then(Value::as_array) {
                    for call in tool_calls {
                        if let Some(args) = call.get("args") {
                            let candidate_path = args.get("SearchDirectory")
                                .or_else(|| args.get("Cwd"))
                                .or_else(|| args.get("SearchPath"))
                                .or_else(|| args.get("TargetFile"))
                                .or_else(|| args.get("AbsolutePath"))
                                .and_then(Value::as_str);

                            if let Some(raw_path) = candidate_path {
                                let path_clean = raw_path.trim_matches('"').trim();
                                if !path_clean.is_empty() {
                                    let norm = normalize_project_path(path_clean);

                                    // Check if norm matches or is inside any known project
                                    for (pname, pcwd) in project_mappings {
                                        let pcwd_lower = pcwd.to_lowercase();
                                        let norm_lower = norm.to_lowercase();
                                        if norm_lower == pcwd_lower
                                            || norm_lower.starts_with(&format!("{}\\", pcwd_lower))
                                            || norm_lower.starts_with(&format!("{}/", pcwd_lower))
                                        {
                                            inferred_project_name = Some(pname.clone());
                                            inferred_cwd = Some(pcwd.clone());
                                            break;
                                        }
                                    }

                                    // If not matched to configured projects, extract directory
                                    if inferred_cwd.is_none() {
                                        let dir_path = if Path::new(&norm).is_file() {
                                            Path::new(&norm).parent().and_then(|p| p.to_str()).unwrap_or(&norm).to_string()
                                        } else {
                                            norm.clone()
                                        };
                                        inferred_project_name = Some(get_project_name(&dir_path));
                                        inferred_cwd = Some(dir_path);
                                    }

                                    if inferred_cwd.is_some() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(content_str) = entry.get("content").and_then(Value::as_str) {
                if !content_str.trim().is_empty() {
                    if let Some(cleaned) = clean_result_message(content_str) {
                        draft.assistant_messages.push(cleaned);
                    }
                }
            }
        }
    }

    draft.cwd = inferred_cwd;
    draft.project_name = inferred_project_name.unwrap_or_else(|| {
        draft.cwd.as_deref().map(get_project_name).unwrap_or_else(|| "Antigravity Project".to_string())
    });

    finalize_draft(draft, from, to)
}

fn parse_opencode_session(path: &Path, from: &str, to: &str) -> Option<CodexReportSession> {
    let content = fs::read_to_string(path).ok()?;
    let val = serde_json::from_str::<Value>(&content).ok()?;

    let session_id = val.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let cwd = val.get("cwd").or_else(|| val.get("projectPath")).and_then(Value::as_str).map(str::to_string);

    let mut draft = SessionDraft {
        id: session_id,
        provider: "opencode".to_string(),
        cwd,
        ..Default::default()
    };

    if let Some(messages) = val.get("messages").and_then(Value::as_array) {
        for msg in messages {
            let role = msg.get("role").and_then(Value::as_str).unwrap_or_default();
            let timestamp = msg.get("timestamp").and_then(Value::as_str).unwrap_or_default();
            let text = msg.get("content").and_then(extract_text_value);

            if !timestamp.is_empty() && !is_in_range(timestamp, from, to) {
                continue;
            }

            if role == "user" {
                if let Some(t) = text.and_then(|s| clean_user_message(&s)) {
                    update_bounds(&mut draft, timestamp);
                    draft.user_messages.push(t);
                }
            } else if role == "assistant" {
                if let Some(t) = text.and_then(|s| clean_result_message(&s)) {
                    update_bounds(&mut draft, timestamp);
                    draft.assistant_messages.push(t);
                }
            }
        }
    }

    finalize_draft(draft, from, to)
}

fn finalize_draft(mut draft: SessionDraft, from: &str, to: &str) -> Option<CodexReportSession> {
    if draft.user_messages.is_empty() && draft.assistant_messages.is_empty() {
        return None;
    }
    if draft.ended_at.is_empty() {
        draft.ended_at = draft.started_at.clone();
    }
    if draft.started_at.is_empty() {
        draft.started_at = draft.ended_at.clone();
    }
    if !is_in_range(&draft.ended_at, from, to) && !is_in_range(&draft.started_at, from, to) {
        return None;
    }

    let project_name = if !draft.project_name.is_empty() {
        draft.project_name
    } else {
        get_project_name(draft.cwd.as_deref().unwrap_or("未识别项目"))
    };

    Some(CodexReportSession {
        id: draft.id,
        provider: draft.provider,
        started_at: draft.started_at,
        ended_at: draft.ended_at,
        cwd: draft.cwd,
        project_name,
        user_messages: deduplicate(draft.user_messages),
        assistant_messages: deduplicate(draft.assistant_messages),
    })
}

fn extract_agy_user_request(content: &str) -> String {
    if let Some(start) = content.find("<USER_REQUEST>") {
        if let Some(end) = content.find("</USER_REQUEST>") {
            let req = &content[start + "<USER_REQUEST>".len()..end];
            return req.trim().to_string();
        }
    }
    content.trim().to_string()
}

pub fn normalize_project_path(raw: &str) -> String {
    let mut p = raw.trim().replace('/', "\\");
    if let Some(stripped) = p.strip_prefix("file:\\\\\\") {
        p = stripped.to_string();
    } else if let Some(stripped) = p.strip_prefix("file:\\\\") {
        p = stripped.to_string();
    } else if let Some(stripped) = p.strip_prefix("file:\\") {
        p = stripped.to_string();
    }
    p = p.replace("%3A", ":").replace("%3a", ":");

    while p.contains("\\\\") {
        p = p.replace("\\\\", "\\");
    }

    if p.len() >= 2 && p.chars().nth(1) == Some(':') {
        let drive = p.chars().next().unwrap().to_ascii_uppercase();
        p = format!("{}{}", drive, &p[1..]);
    }
    while p.len() > 3 && p.ends_with('\\') {
        p.pop();
    }
    p
}

fn load_antigravity_metadata(home: &Path) -> (
    HashMap<String, (String, String)>,
    Vec<(String, String)>,
    HashMap<String, (String, String)>,
    HashMap<String, (usize, Option<String>)>,
) {
    let mut project_id_map: HashMap<String, (String, String)> = HashMap::new();
    let mut project_mappings: Vec<(String, String)> = Vec::new();
    let mut conv_to_project: HashMap<String, (String, String)> = HashMap::new();
    let mut project_stats: HashMap<String, (usize, Option<String>)> = HashMap::new();

    // 1. Load project configurations from ~/.gemini/config/projects/*.json
    let config_projects = home.join(".gemini").join("config").join("projects");
    if config_projects.exists() {
        if let Ok(entries) = fs::read_dir(&config_projects) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(val) = serde_json::from_str::<Value>(&content) {
                            let id = val.get("id").and_then(Value::as_str)
                                .map(str::to_string)
                                .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string());
                            let name = val.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                            let updated_at = val.get("updatedAt").and_then(Value::as_str).map(str::to_string);
                            let folder_uri = val.pointer("/projectResources/resources/0/gitFolder/folderUri")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            let cwd = decode_uri_to_path(folder_uri).unwrap_or_else(|| name.clone());
                            let norm_cwd = normalize_project_path(&cwd);

                            if !name.is_empty() && name != "Outside of Project" {
                                if !project_mappings.iter().any(|(n, c)| n == &name && c == &norm_cwd) {
                                    project_mappings.push((name.clone(), norm_cwd.clone()));
                                }
                                if !id.is_empty() {
                                    project_id_map.insert(id, (name.clone(), norm_cwd.clone()));
                                }
                                if let Some(ts) = updated_at {
                                    let stat = project_stats.entry(norm_cwd.clone()).or_insert((0, None));
                                    if stat.1.as_ref().map_or(true, |existing| &ts > existing) {
                                        stat.1 = Some(ts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Load conversation summaries from ~/.gemini/antigravity/conversation_summaries.db
    let db_path = home.join(".gemini").join("antigravity").join("conversation_summaries.db");
    if db_path.exists() {
        if let Ok(conn) = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        ) {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT conversation_id, project_id, workspace_uris, last_modified_time FROM conversation_summaries"
            ) {
                let rows = stmt.query_map([], |row| {
                    let conv_id: String = row.get(0)?;
                    let proj_id: Option<String> = row.get(1)?;
                    let uris: Option<String> = row.get(2)?;
                    let last_mod: Option<String> = row.get(3)?;
                    Ok((conv_id, proj_id, uris, last_mod))
                });
                if let Ok(iter) = rows {
                    for item in iter.flatten() {
                        let (conv_id, proj_id, uris, last_mod) = item;

                        // Resolve project: 1. proj_id, 2. workspace_uris
                        let mut resolved = proj_id.as_deref().and_then(|pid| project_id_map.get(pid)).cloned();

                        if resolved.is_none() {
                            if let Some(u_str) = uris {
                                if let Ok(u_arr) = serde_json::from_str::<Vec<String>>(&u_str) {
                                    if let Some(first_uri) = u_arr.first() {
                                        if let Some(cwd) = decode_uri_to_path(first_uri) {
                                            let matched_name = project_mappings.iter()
                                                .find(|(_, pcwd)| pcwd.eq_ignore_ascii_case(&cwd))
                                                .map(|(pname, _)| pname.clone())
                                                .unwrap_or_else(|| get_project_name(&cwd));
                                            resolved = Some((matched_name, cwd));
                                        }
                                    }
                                }
                            }
                        }

                        if let Some((pname, pcwd)) = resolved {
                            let stat = project_stats.entry(pcwd.clone()).or_insert((0, None));
                            stat.0 += 1;
                            if let Some(ref ts) = last_mod {
                                if stat.1.as_ref().map_or(true, |existing| ts > existing) {
                                    stat.1 = Some(ts.clone());
                                }
                            }
                            conv_to_project.insert(conv_id, (pname, pcwd));
                        }
                    }
                }
            }
        }
    }

    (project_id_map, project_mappings, conv_to_project, project_stats)
}

fn decode_claude_project_path(folder_name: &str) -> String {
    let s = folder_name.replace("__", "\\").replace('_', "\\");
    if s.len() > 2 && s.chars().nth(1) == Some(':') {
        normalize_project_path(&s)
    } else {
        folder_name.to_string()
    }
}

fn decode_uri_to_path(uri: &str) -> Option<String> {
    let raw = uri.trim();
    if raw.is_empty() {
        return None;
    }
    let unescaped = raw.replace("%3A", ":").replace("%3a", ":");
    if let Some(stripped) = unescaped.strip_prefix("file:///") {
        let decoded = stripped.replace('/', "\\");
        return Some(normalize_project_path(&decoded));
    } else if let Some(stripped) = unescaped.strip_prefix("file://") {
        let decoded = stripped.replace('/', "\\");
        return Some(normalize_project_path(&decoded));
    } else if let Some(stripped) = unescaped.strip_prefix("file:") {
        let decoded = stripped.replace('/', "\\");
        return Some(normalize_project_path(&decoded));
    }
    Some(normalize_project_path(&unescaped))
}

fn extract_text_value(val: &Value) -> Option<String> {
    if let Some(s) = val.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = val.as_array() {
        let texts: Vec<String> = arr.iter().filter_map(|item| {
            if let Some(t) = item.get("text").and_then(Value::as_str) {
                Some(t.trim().to_string())
            } else if let Some(t) = item.get("content").and_then(Value::as_str) {
                Some(t.trim().to_string())
            } else {
                item.as_str().map(|s| s.trim().to_string())
            }
        }).filter(|s| !s.is_empty()).collect();
        if !texts.is_empty() {
            return Some(texts.join("\n"));
        }
    }
    None
}

fn is_in_range(timestamp: &str, from: &str, to: &str) -> bool {
    !timestamp.is_empty() && timestamp >= from && timestamp <= to
}

fn update_bounds(draft: &mut SessionDraft, timestamp: &str) {
    if draft.started_at.is_empty() || timestamp < draft.started_at.as_str() {
        draft.started_at = timestamp.to_string();
    }
    if draft.ended_at.is_empty() || timestamp > draft.ended_at.as_str() {
        draft.ended_at = timestamp.to_string();
    }
}

fn get_project_name(cwd: &str) -> String {
    let normalized = normalize_project_path(cwd);
    let path = Path::new(&normalized);
    path.file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(cwd)
        .to_string()
}

fn clean_user_message(text: &str) -> Option<String> {
    clean_report_text(text, 1_500, true)
}

fn clean_result_message(text: &str) -> Option<String> {
    clean_report_text(text, 2_000, false)
}

fn clean_report_text(text: &str, max_chars: usize, reject_runtime_context: bool) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || (reject_runtime_context && is_runtime_context(text)) {
        return None;
    }

    let compact = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if compact.is_empty() {
        return None;
    }

    if compact.chars().count() <= max_chars {
        return Some(compact);
    }
    Some(format!(
        "{}…（内容过长，已截断）",
        compact.chars().take(max_chars).collect::<String>()
    ))
}

fn is_runtime_context(text: &str) -> bool {
    [
        "<recommended_plugins>",
        "<app-context>",
        "<skills_instructions>",
        "<environment_context>",
        "<permissions instructions>",
        "<collaboration_mode>",
        "<apps_instructions>",
        "<plugins_instructions>",
        "# AGENTS.md instructions",
        "<USER_SETTINGS_CHANGE>",
        "<ADDITIONAL_METADATA>",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn deduplicate(messages: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for message in messages {
        if !unique.iter().any(|existing| existing == &message) {
            unique.push(message);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_parse_codex_session_and_directory_pruning() {
        let temp_dir = std::env::temp_dir().join(format!("halowake_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let session_dir = temp_dir.join("2026").join("08").join("21");
        fs::create_dir_all(&session_dir).expect("create temp dir");

        let file_path = session_dir.join("rollout-2026-08-21T13-26-11-01a022c8-aac1-77f2-9f3e-dcc481ea205d.jsonl");
        let mut file = File::create(&file_path).expect("create test file");
        writeln!(file, r#"{{"timestamp":"2026-08-21T05:26:12.296Z","type":"session_meta","payload":{{"session_id":"01a022c8-aac1-77f2-9f3e-dcc481ea205d","cwd":"D:\\test_project"}}}}"#).unwrap();
        writeln!(file, r#"{{"timestamp":"2026-08-25T02:59:50.846Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"帮我找一台离线电表"}}]}}}}"#).unwrap();
        writeln!(file, r#"{{"timestamp":"2026-08-25T03:00:37.806Z","type":"response_item","payload":{{"type":"message","role":"assistant","phase":"final_answer","content":[{{"type":"output_text","text":"已定位到离线电表数据"}}]}}}}"#).unwrap();
        drop(file);

        let mut collected = Vec::new();
        collect_codex_jsonl_files_pruned(&temp_dir, "2026-08-25", &mut collected);
        assert_eq!(collected.len(), 1, "Should collect historical session file");

        let session = parse_codex_session(&file_path, "2026-08-24T16:00:00.000Z", "2026-08-25T15:59:59.999Z")
            .expect("should parse session");
        assert_eq!(session.id, "01a022c8-aac1-77f2-9f3e-dcc481ea205d");
        assert_eq!(session.user_messages.len(), 1);
        assert!(session.user_messages[0].contains("帮我找一台离线电表"));
        assert_eq!(session.assistant_messages.len(), 1);
        assert!(session.assistant_messages[0].contains("已定位到离线电表数据"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_multi_provider_project_aggregation_and_path_normalization() {
        let mut project_map = HashMap::new();
        record_project_activity(&mut project_map, "D:\\ly_project\\ami-insight", None, Some("2026-08-28T06:00:00Z".to_string()), "codex", 13);
        record_project_activity(&mut project_map, "d:/ly_project/ami-insight", Some("ami-insight"), Some("2026-08-28T09:00:00Z".to_string()), "antigravity", 1);

        assert_eq!(project_map.len(), 1, "Should merge different casings/slashes into 1 project");
        let proj = project_map.values().next().unwrap();
        assert_eq!(proj.name, "ami-insight");
        assert_eq!(proj.session_count, 14);
        assert_eq!(proj.providers, vec!["codex".to_string(), "antigravity".to_string()]);
        assert_eq!(proj.last_active_at, Some("2026-08-28T09:00:00Z".to_string()));
    }

    #[test]
    fn test_parse_antigravity_transcript_does_not_infer_from_user_message_logs() {
        let temp_dir = std::env::temp_dir().join(format!("halowake_agy_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let file_path = temp_dir.join("transcript.jsonl");
        let mut file = File::create(&file_path).expect("create test file");

        // Line 1: User pasted error log mentioning another project D:\ly_project\ecogrid-mdm
        writeln!(
            file,
            r#"{{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","status":"DONE","created_at":"2026-09-10T08:13:55Z","content":"<USER_REQUEST>\n问题，我在devdock添加了一个react项目，但是加载出来后我点击执行命令报错了。应该执行npm run dev，但是devdock执行的是 准备执行：bun run dev\r\n工作目录：D:\\ly_project\\ecogrid-mdm\r\n进程已启动，PID 22008\r\n'bun' 不是内部或外部命令\n</USER_REQUEST>"}}"#
        ).unwrap();

        // Line 2: Agent executes tool in Lumina workspace
        writeln!(
            file,
            r#"{{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","status":"DONE","created_at":"2026-09-10T08:13:55Z","content":"已排查","tool_calls":[{{"name":"find_by_name","args":{{"SearchDirectory":"\"C:\\\\Users\\\\user\\\\Desktop\\\\templates\\\\Lumina\""}}}}]}}"#
        ).unwrap();
        drop(file);

        let conv_project_map = HashMap::new();
        let project_mappings = vec![
            ("ecogrid-mdm".to_string(), "D:\\ly_project\\ecogrid-mdm".to_string()),
            ("Lumina".to_string(), "C:\\Users\\user\\Desktop\\templates\\Lumina".to_string()),
        ];

        let session = parse_antigravity_transcript(
            &file_path,
            "test-conv-001",
            &conv_project_map,
            &project_mappings,
            "2026-09-10T00:00:00Z",
            "2026-09-10T23:59:59Z",
        ).expect("should parse session");

        // Must NOT be ecogrid-mdm, must be Lumina based on tool execution path
        assert_eq!(session.project_name, "Lumina");
        assert_eq!(session.cwd, Some("C:\\Users\\user\\Desktop\\templates\\Lumina".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_antigravity_transcript_prefers_database_mapping() {
        let temp_dir = std::env::temp_dir().join(format!("halowake_agy_test_db_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let file_path = temp_dir.join("transcript.jsonl");
        let mut file = File::create(&file_path).expect("create test file");

        writeln!(
            file,
            r#"{{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","status":"DONE","created_at":"2026-09-15T03:05:52Z","content":"<USER_REQUEST>刚刚新增了xiaomi mimo的额度适配器，帮我发个版本吧</USER_REQUEST>"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","status":"DONE","created_at":"2026-09-15T03:07:54Z","content":"新版本 v1.3.4 已完成发布"}}"#
        ).unwrap();
        drop(file);

        let mut conv_project_map = HashMap::new();
        conv_project_map.insert("9596f35f".to_string(), ("Lumina".to_string(), "C:\\Users\\user\\Desktop\\templates\\Lumina".to_string()));

        let project_mappings = vec![
            ("ecogrid-mdm".to_string(), "D:\\ly_project\\ecogrid-mdm".to_string()),
            ("Lumina".to_string(), "C:\\Users\\user\\Desktop\\templates\\Lumina".to_string()),
        ];

        let session = parse_antigravity_transcript(
            &file_path,
            "9596f35f",
            &conv_project_map,
            &project_mappings,
            "2026-09-15T00:00:00Z",
            "2026-09-15T23:59:59Z",
        ).expect("should parse session");

        assert_eq!(session.project_name, "Lumina");
        assert_eq!(session.cwd, Some("C:\\Users\\user\\Desktop\\templates\\Lumina".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_real_antigravity_conversation_9596f35f_resolves_to_lumina() {
        let home = match get_home_dir() {
            Ok(h) => h,
            Err(_) => return,
        };
        let real_transcript = home
            .join(".gemini")
            .join("antigravity")
            .join("brain")
            .join("9596f35f-f6b3-4090-a2ea-9ca13b00fc07")
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");

        if !real_transcript.exists() {
            return;
        }

        let (_id_map, project_mappings, conv_to_project, _stats) = load_antigravity_metadata(&home);

        // Verify conversation is recognized in DB as Lumina
        if let Some((pname, _pcwd)) = conv_to_project.get("9596f35f-f6b3-4090-a2ea-9ca13b00fc07") {
            assert_eq!(pname, "Lumina");
        }

        // Test parsing the real file on 2026-09-15 (when user asked for xiaomi mimo)
        let session = parse_antigravity_transcript(
            &real_transcript,
            "9596f35f-f6b3-4090-a2ea-9ca13b00fc07",
            &conv_to_project,
            &project_mappings,
            "2026-09-15T00:00:00Z",
            "2026-09-15T23:59:59Z",
        ).expect("should parse real session");

        assert_eq!(session.project_name, "Lumina");
        assert_ne!(session.project_name, "ecogrid-mdm");
        assert!(session.cwd.as_deref().unwrap_or("").to_lowercase().contains("lumina"));
        assert!(session.user_messages.iter().any(|m| m.contains("xiaomi mimo")));
    }

    #[test]
    fn test_parse_iso_timestamp() {
        assert_eq!(parse_iso_timestamp("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso_timestamp("2026-09-07T00:00:00.000Z"), Some(1788739200));
        assert_eq!(parse_iso_timestamp("2026-09-07T08:00:00+08:00"), Some(1788739200));
        assert_eq!(fast_extract_timestamp(r#"{"created_at":"2026-09-10T08:13:55Z"}"#), Some("2026-09-10T08:13:55Z"));
        assert_eq!(fast_extract_timestamp(r#"{"timestamp":"2026-09-10T08:13:55.123Z"}"#), Some("2026-09-10T08:13:55.123Z"));
    }


    #[test]
    fn test_sep_11_sessions_loads_continued_codex_sessions() {
        let home = match get_home_dir() {
            Ok(h) => h,
            Err(_) => return,
        };
        let codex_sep_11 = home.join(".codex").join("sessions").join("2026").join("09").join("11");
        if !codex_sep_11.exists() {
            // Skip when running in clean CI runner environment without local test data
            return;
        }

        // 2026-09-11 in Beijing time (UTC+8): 2026-09-10T16:00:00Z ~ 2026-09-11T15:59:59.999Z
        let query = CodexReportQuery {
            from: "2026-09-10T16:00:00.000Z".to_string(),
            to: "2026-09-11T15:59:59.999Z".to_string(),
            providers: None,
        };
        let res = load_codex_report_sessions_sync(query).expect("load 9-11 sessions");
        println!("Loaded {} sessions for 2026-09-11:", res.len());
        for s in &res {
            println!("  [{}] {} ({}) - {} msgs", s.provider, s.project_name, s.started_at, s.user_messages.len());
        }
        // Should include ecogrid-mdm and ami-insight which were continued on 9-11
        assert!(res.iter().any(|s| s.project_name == "ecogrid-mdm"), "ecogrid-mdm should be found on 9-11");
        assert!(res.iter().any(|s| s.project_name == "ami-insight"), "ami-insight should be found on 9-11");
    }
}

