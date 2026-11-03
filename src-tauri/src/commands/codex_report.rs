use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

mod opencode;

use chrono::{DateTime, SecondsFormat, Utc};
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


fn configured_roots(home: &Path, variable: &str, default_name: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = env::var_os(variable).filter(|v| !v.is_empty()) {
        roots.push(PathBuf::from(root));
    }
    let default = home.join(default_name);
    if !roots.contains(&default) { roots.push(default); }
    roots
}

fn codex_history_dirs(home: &Path) -> Vec<PathBuf> {
    configured_roots(home, "CODEX_HOME", ".codex").into_iter()
        .flat_map(|root| [root.join("sessions"), root.join("archived_sessions")]).collect()
}

fn claude_project_dirs(home: &Path) -> Vec<PathBuf> {
    configured_roots(home, "CLAUDE_CONFIG_DIR", ".claude").into_iter()
        .map(|root| root.join("projects")).collect()
}

fn is_jsonl_history(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".jsonl") || n.ends_with(".jsonl.zst"))
}

fn history_reader(path: &Path) -> Option<Box<dyn BufRead>> {
    let file = fs::File::open(path).ok()?;
    if path.extension().is_some_and(|ext| ext == "zst") {
        let decoder = zstd::stream::read::Decoder::new(file).ok()?;
        Some(Box::new(BufReader::new(decoder)))
    } else {
        Some(Box::new(BufReader::new(file)))
    }
}

fn canonical_timestamp(timestamp: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(timestamp).ok()
        .map(|ts| ts.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn merge_sessions(sessions: Vec<CodexReportSession>) -> Vec<CodexReportSession> {
    let mut merged: HashMap<(String, String), CodexReportSession> = HashMap::new();
    for session in sessions {
        let key = (session.provider.clone(), session.id.clone());
        if let Some(existing) = merged.get_mut(&key) {
            if session.started_at < existing.started_at { existing.started_at = session.started_at; }
            if session.ended_at > existing.ended_at { existing.ended_at = session.ended_at; }
            if existing.cwd.is_none() && session.cwd.is_some() {
                existing.cwd = session.cwd;
                existing.project_name = session.project_name;
            }
            existing.user_messages.extend(session.user_messages);
            existing.assistant_messages.extend(session.assistant_messages);
        } else { merged.insert(key, session); }
    }
    let mut sessions: Vec<_> = merged.into_values().collect();
    for session in &mut sessions {
        session.user_messages = deduplicate(std::mem::take(&mut session.user_messages));
        session.assistant_messages = deduplicate(std::mem::take(&mut session.assistant_messages));
    }
    sessions.sort_by(|a,b| b.started_at.cmp(&a.started_at).then_with(|| a.provider.cmp(&b.provider)).then_with(|| a.id.cmp(&b.id)));
    sessions
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
    let codex_dirs = codex_history_dirs(&home);
    let codex_installed = codex_dirs.iter().any(|dir| dir.exists());
    let mut codex_count = 0;
    if codex_installed {
        let mut codex_files = Vec::new();
        for dir in &codex_dirs { if dir.exists() { let _ = collect_jsonl_files(dir, &mut codex_files); } }
        codex_count = codex_files.iter().map(|path| history_session_id(path).unwrap_or_else(|| path.to_string_lossy().into_owned())).collect::<HashSet<_>>().len();
    }
    tools.push(InstalledToolInfo {
        provider: "codex".to_string(),
        name: "Codex CLI".to_string(),
        is_installed: codex_installed,
        session_count: codex_count,
    });

    // 2. Claude Code
    let claude_dir = home.join(".claude");
    let claude_projects = claude_project_dirs(&home);
    let claude_installed = claude_dir.exists() || claude_projects.iter().any(|p| p.exists());
    let mut claude_count = 0;
    if claude_installed {
        let mut claude_files = Vec::new();
        for dir in &claude_projects { if dir.exists() { let _ = collect_jsonl_files(dir, &mut claude_files); } }
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
    let opencode_roots = opencode::data_roots(&home);
    let opencode_installed = opencode_roots.iter().any(|dir| dir.join("opencode.db").exists() || dir.join("storage").exists());
    let opencode_count = opencode::session_index(&opencode_roots)?.len();
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
    let mut seen_codex = HashSet::new();
    for dir in codex_history_dirs(&home) {
        if !dir.exists() { continue; }
        let mut files = Vec::new();
        collect_jsonl_files(&dir, &mut files)?;
        files.sort();
        for path in files {
            if let Some((cwd, timestamp)) = extract_codex_session_meta(&path) {
                // Metadata ID is shared by resumed/compacted rollout files.
                let id = history_session_id(&path).unwrap_or_else(|| path.to_string_lossy().into_owned());
                if seen_codex.insert(id) {
                    record_project_activity(&mut project_map, &cwd, None, timestamp, "codex", 1);
                }
            }
        }
    }

    // 2. Claude Code Projects: cwd in the record is authoritative; folder encoding is lossy.
    for dir in claude_project_dirs(&home) {
        if !dir.exists() { continue; }
        let mut files = Vec::new();
        collect_jsonl_files(&dir, &mut files)?;
        let sessions = parse_items_parallel(files, |path| parse_claude_session(path, "1970-01-01T00:00:00Z", "9999-12-31T23:59:59Z"));
        for session in merge_sessions(sessions) {
            if let Some(cwd) = session.cwd {
                record_project_activity(&mut project_map, &cwd, None, Some(session.ended_at), "claude", 1);
            }
        }
    }

    // 4. OpenCode Projects (SQLite and legacy JSON share the same discovery roots).
    for (_, cwd, timestamp) in opencode::session_index(&opencode::data_roots(&home))? {
        record_project_activity(&mut project_map, &cwd, None, timestamp, "opencode", 1);
    }

    // 3. Antigravity Projects
    let (_project_id_map, mut project_mappings, conv_to_project, project_stats) = load_antigravity_metadata(&home);
    for (name, cwd) in conv_to_project.values() {
        if !project_mappings.iter().any(|(_, existing)| existing.eq_ignore_ascii_case(cwd)) {
            project_mappings.push((name.clone(), cwd.clone()));
        }
    }
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
    load_report_sessions_from_home(&get_home_dir()?, query)
}

fn load_report_sessions_from_home(home: &Path, query: CodexReportQuery) -> Result<Vec<CodexReportSession>, String> {
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

    let from_time = DateTime::parse_from_rfc3339(&from_str).map_err(|_| "无效的日报开始时间")?;
    let to_time = DateTime::parse_from_rfc3339(&to_str).map_err(|_| "无效的日报结束时间")?;
    if from_time > to_time { return Err("无效的日报日期范围".into()); }
    let mut all_sessions = Vec::new();
    if selected_providers.iter().any(|p| p == "codex" || p == "all") {
        let mut files = Vec::new();
        for dir in codex_history_dirs(home) {
            if dir.exists() { collect_jsonl_files(&dir, &mut files)?; }
        }
        files.sort();
        files.dedup();
        all_sessions.extend(parse_items_parallel(files, |path| parse_codex_session(path, &from_str, &to_str)));
    }
    if selected_providers.iter().any(|p| p == "claude" || p == "all") {
        let mut files = Vec::new();
        for dir in claude_project_dirs(home) {
            if dir.exists() { collect_jsonl_files(&dir, &mut files)?; }
        }
        files.sort();
        files.dedup();
        all_sessions.extend(parse_items_parallel(files, |path| parse_claude_session(path, &from_str, &to_str)));
    }

    // 3. Antigravity transcripts; database metadata determines workspace attribution.
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
                        for path in [transcript_file.clone(), transcript_file.with_extension("jsonl.zst")] {
                            if path.exists() { agy_candidates.push((path, conv_id.clone())); }
                        }
                    }
                }
            }
            agy_candidates.sort();
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

    if selected_providers.iter().any(|p| p == "opencode" || p == "all") {
        all_sessions.extend(opencode::load_sessions(&opencode::data_roots(home), &from_str, &to_str)?);
    }
    Ok(merge_sessions(all_sessions))
}

// Storage directory dates and file mtimes are not message dates. Scan historical
// files even after they have been migrated, archived, compressed or resumed.
fn collect_jsonl_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("读取目录失败 {}: {}", directory.display(), error))?
    {
        let path = entry
            .map_err(|error| format!("读取条目失败: {}", error))?
            .path();
        if path.is_dir() {
            collect_jsonl_files(&path, files)?;
        } else if is_jsonl_history(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn extract_codex_session_meta(path: &Path) -> Option<(String, Option<String>)> {
    let reader = history_reader(path)?;

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

fn history_session_id(path: &Path) -> Option<String> {
    for line in history_reader(path)?.lines().take(15) {
        let entry: Value = serde_json::from_str(&line.ok()?).ok()?;
        if entry.get("type").and_then(Value::as_str) == Some("session_meta") {
            return entry.pointer("/payload/session_id").or_else(|| entry.pointer("/payload/id")).and_then(Value::as_str).map(str::to_string);
        }
    }
    None
}

fn parse_codex_session(path: &Path, from: &str, to: &str) -> Option<CodexReportSession> {
    let reader = history_reader(path)?;

    let uncompressed = path.with_extension("");
    let id_path = if path.extension().is_some_and(|e| e == "zst") { &uncompressed } else { path };
    let default_id = id_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            s.get(s.len().saturating_sub(36)..).unwrap_or(s).to_string()
        })
        .unwrap_or_default();

    let mut draft = SessionDraft {
        id: default_id,
        provider: "codex".to_string(),
        ..Default::default()
    };

    for line_res in reader.lines() {
        let Ok(line) = line_res else { break; };
        // Fast string pre-filter to avoid unnecessary serde deserialization
        let is_meta = line.contains("\"session_meta\"");
        if !is_meta
            && !line.contains("\"user\"")
            && !line.contains("\"user_message\"")
            && !line.contains("\"agent_message\"")
            && !line.contains("\"final_answer\"")
            && !line.contains("\"assistant\"")
        {
            continue;
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
        else if role == "assistant" || (entry_type == "event_msg" && payload_type == "agent_message") {
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
    let reader = history_reader(path)?;

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
        let Ok(line) = line_res else { break; };
        if !line.contains("\"user\"") && !line.contains("\"assistant\"") {
            continue;
        }

        let Ok(entry) = serde_json::from_str::<Value>(&line) else { continue; };
        if let Some(cwd) = entry.get("cwd").and_then(Value::as_str).filter(|cwd| !cwd.is_empty()) {
            draft.cwd = Some(normalize_project_path(cwd));
        }
        if let Some(id) = entry.get("sessionId").and_then(Value::as_str).filter(|id| !id.is_empty()) {
            // Subagents have their own file identity; do not merge their work into the parent.
            if !entry.get("isSidechain").and_then(Value::as_bool).unwrap_or(false) {
                draft.id = id.to_string();
            }
        }
        let timestamp = entry.get("timestamp")
            .or_else(|| entry.get("created_at"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        if !is_in_range(timestamp, from, to) {
            continue;
        }

        if !timestamp.is_empty() {
            update_bounds(&mut draft, timestamp);
        }

        let msg_type = entry.get("type").and_then(Value::as_str).unwrap_or_default();
        let role = entry.get("role").or_else(|| entry.pointer("/message/role")).and_then(Value::as_str).unwrap_or_default();

        if msg_type == "user" || role == "user" {
            let text = entry.pointer("/message/content")
                .or_else(|| entry.get("content"))
                .or_else(|| entry.get("message"))
                .and_then(extract_text_value);
            if let Some(t) = text.and_then(|s| clean_user_message(&s)) {
                draft.user_messages.push(t);
            }
        } else if msg_type == "assistant" || role == "assistant" {
            let text = entry.pointer("/message/content")
                .or_else(|| entry.get("content"))
                .or_else(|| entry.get("message"))
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
    let reader = history_reader(path)?;

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
        let Ok(line) = line_res else { break; };

        // Fast skip lines that are not user input or planner response
        if !line.contains("\"USER_INPUT\"") && !line.contains("\"PLANNER_RESPONSE\"") {
            continue;
        }

        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let timestamp = entry.get("created_at")
            .or_else(|| entry.get("timestamp"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        let in_range = is_in_range(timestamp, from, to);
        if in_range { update_bounds(&mut draft, timestamp); }

        let entry_type = entry.get("type").and_then(Value::as_str).unwrap_or_default();

        if entry_type == "USER_INPUT" && in_range {
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

            if let Some(content_str) = entry.get("content").and_then(Value::as_str).filter(|_| in_range) {
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
    let cwd = val.get("directory").or_else(|| val.get("cwd")).or_else(|| val.get("projectPath")).and_then(Value::as_str).map(str::to_string);

    let mut draft = SessionDraft {
        id: session_id,
        provider: "opencode".to_string(),
        cwd,
        ..Default::default()
    };

    if let Some(messages) = val.get("messages").and_then(Value::as_array) {
        for msg in messages {
            let role = msg.get("role").and_then(Value::as_str).unwrap_or_default();
            let Some(timestamp) = opencode::message_timestamp(msg) else { continue; };
            let timestamp = timestamp.as_str();
            let text = msg.get("content").and_then(extract_text_value);

            if !is_in_range(timestamp, from, to) {
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
    if val.is_object() {
        return val.get("text").or_else(|| val.get("content")).and_then(extract_text_value);
    }
    if let Some(arr) = val.as_array() {
        let texts: Vec<String> = arr.iter().filter_map(|item| {
            if let Some(kind) = item.get("type").and_then(Value::as_str) {
                if !matches!(kind, "text" | "input_text" | "output_text") { return None; }
            }
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
    match (DateTime::parse_from_rfc3339(timestamp), DateTime::parse_from_rfc3339(from), DateTime::parse_from_rfc3339(to)) {
        (Ok(timestamp), Ok(from), Ok(to)) => timestamp >= from && timestamp <= to,
        _ => false,
    }
}

fn update_bounds(draft: &mut SessionDraft, timestamp: &str) {
    let Some(canonical) = canonical_timestamp(timestamp) else { return; };
    let timestamp = canonical.as_str();
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
    fn antigravity_loads_compressed_and_plain_logs_with_old_mtimes() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join(".gemini/antigravity/brain/fixture-ag/.system_generated/logs");
        fs::create_dir_all(&dir).unwrap();
        let old = concat!(
            "{\"created_at\":\"2026-06-01T00:00:00Z\",\"type\":\"PLANNER_RESPONSE\",\"tool_calls\":[{\"args\":{\"Cwd\":\"D:/actual\"}}]}\n",
            "{\"created_at\":\"2026-07-01T00:00:00Z\",\"type\":\"USER_INPUT\",\"content\":\"question\"}\n"
        );
        fs::write(dir.join("transcript.jsonl.zst"), zstd::stream::encode_all(old.as_bytes(), 1).unwrap()).unwrap();
        let plain = dir.join("transcript.jsonl");
        fs::write(&plain, "{\"created_at\":\"2026-07-01T00:01:00Z\",\"type\":\"PLANNER_RESPONSE\",\"content\":\"answer\"}\n").unwrap();
        fs::File::options().write(true).open(&plain).unwrap().set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH)).unwrap();
        let sessions = load_report_sessions_from_home(temp.path(), CodexReportQuery {
            from: "2026-07-01T00:00:00.000Z".into(), to: "2026-07-01T23:59:59.999Z".into(), providers: Some(vec!["antigravity".into()]),
        }).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project_name, "actual");
        assert_eq!(sessions[0].user_messages, ["question"]);
        assert_eq!(sessions[0].assistant_messages, ["answer"]);
    }

    #[test]
    fn loader_reads_archives_later_directories_and_old_file_mtimes() {
        let temp = tempfile::tempdir().unwrap();
        let later = temp.path().join(".codex/sessions/2026/11/01");
        let archive = temp.path().join(".codex/archived_sessions");
        fs::create_dir_all(&later).unwrap();
        fs::create_dir_all(&archive).unwrap();
        let metadata = "{\"timestamp\":\"2026-07-01T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"fixture-archive\",\"cwd\":\"D:/archive\"}}\n";
        fs::write(later.join("resumed.jsonl"), format!("{metadata}{}\n", r#"{"timestamp":"2026-07-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"question"}}"#)).unwrap();
        let text = format!("{metadata}{}\n", r#"{"timestamp":"2026-07-01T00:01:00Z","type":"event_msg","payload":{"type":"agent_message","message":"answer"}}"#);
        let file = archive.join("archived.jsonl.zst");
        fs::write(&file, zstd::stream::encode_all(text.as_bytes(), 1).unwrap()).unwrap();
        fs::File::options().write(true).open(&file).unwrap().set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH)).unwrap();
        let sessions = load_report_sessions_from_home(temp.path(), CodexReportQuery {
            from: "2026-07-01T00:00:00Z".into(), to: "2026-09-30T23:59:59Z".into(), providers: Some(vec!["codex".into()]),
        }).unwrap();
        let session = sessions.iter().find(|s| s.id == "fixture-archive").unwrap();
        assert_eq!(session.user_messages, ["question"]);
        assert_eq!(session.assistant_messages, ["answer"]);
    }

    #[test]
    fn compressed_archived_codex_and_resumed_messages_are_merged() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("continued.jsonl.zst");
        let text = concat!(
            "{\"timestamp\":\"2026-07-01T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"same-session\",\"cwd\":\"D:/project\"}}\n",
            "{\"timestamp\":\"2026-07-01T00:00:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"repair\"}}\n"
        );
        fs::write(&file, zstd::stream::encode_all(text.as_bytes(), 1).unwrap()).unwrap();
        let first = parse_codex_session(&file, "2026-07-01T00:00:00.000Z", "2026-07-02T00:00:00Z").unwrap();
        assert_eq!(first.id, "same-session");
        assert_eq!(first.user_messages, ["repair"]);
        assert_eq!(extract_codex_session_meta(&file).unwrap().0, "D:/project");
        let mut second = first.clone();
        second.user_messages.clear();
        second.assistant_messages = vec!["fixed".into()];
        let merged = merge_sessions(vec![first.clone(), second, first]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].user_messages, ["repair"]);
        assert_eq!(merged[0].assistant_messages, ["fixed"]);
        let mut files = Vec::new();
        collect_jsonl_files(temp.path(), &mut files).unwrap();
        assert_eq!(files, [file]);
    }

    #[test]
    fn claude_nested_messages_use_real_workspace_and_session_id() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("agent.jsonl");
        fs::write(&file, concat!(
            "{\"timestamp\":\"2026-07-01T08:00:00+08:00\",\"type\":\"user\",\"sessionId\":\"claude-session\",\"cwd\":\"D:/my-project\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"question\"}]}}\n",
            "{\"timestamp\":\"2026-07-01T00:01:00Z\",\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"private\"},{\"type\":\"tool_use\",\"name\":\"Read\"},{\"type\":\"text\",\"text\":\"answer\"}]}}\n"
        )).unwrap();
        let session = parse_claude_session(&file, "2026-07-01T00:00:00.000Z", "2026-07-01T23:59:59.999Z").unwrap();
        assert_eq!(session.id, "claude-session");
        assert_eq!(session.project_name, "my-project");
        assert_eq!(session.user_messages, ["question"]);
        assert_eq!(session.assistant_messages, ["answer"]);
    }

    #[test]
    fn timestamp_range_compares_instants_and_fractional_boundaries() {
        let from = "2026-07-01T00:00:00.000Z";
        let to = "2026-07-01T23:59:59.999Z";
        assert!(is_in_range("2026-07-01T00:00:00Z", from, to));
        assert!(is_in_range("2026-07-02T07:59:59+08:00", from, to));
        assert!(!is_in_range("2026-07-01T07:59:59+08:00", from, to));
        assert!(!is_in_range("2026-07-01T23:59:59.999001Z", from, to));
    }

    #[test]
    fn antigravity_infers_workspace_from_earlier_tool_calls() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("transcript.jsonl");
        fs::write(&file, concat!(
            "{\"created_at\":\"2026-06-01T00:00:00Z\",\"type\":\"PLANNER_RESPONSE\",\"tool_calls\":[{\"args\":{\"Cwd\":\"D:/actual-project\"}}]}\n",
            "{\"created_at\":\"2026-07-01T00:00:00Z\",\"type\":\"USER_INPUT\",\"content\":\"question\"}\n",
            "{\"created_at\":\"2026-07-01T00:01:00Z\",\"type\":\"PLANNER_RESPONSE\",\"content\":\"answer\"}\n"
        )).unwrap();
        let session = parse_antigravity_transcript(&file, "ag-session", &HashMap::new(), &[], "2026-07-01T00:00:00Z", "2026-07-01T23:59:59Z").unwrap();
        assert_eq!(session.project_name, "actual-project");
        assert_eq!(session.assistant_messages, ["answer"]);
    }

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
        collect_jsonl_files(&temp_dir, &mut collected).unwrap();
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
    fn test_timestamp_normalization() {
        assert_eq!(canonical_timestamp("1970-01-01T00:00:00Z").as_deref(), Some("1970-01-01T00:00:00.000Z"));
        assert_eq!(canonical_timestamp("2026-09-07T08:00:00+08:00"), canonical_timestamp("2026-09-07T00:00:00.000Z"));
        assert!(canonical_timestamp("invalid").is_none());
    }

    #[test]
    fn test_sep_11_sessions_loads_continued_codex_sessions() {
        // 1. Hermetic fixture test for continued session collection and date pruning
        let temp_dir = std::env::temp_dir().join(format!("halowake_codex_continued_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let session_dir = temp_dir.join("2026").join("09").join("10");
        fs::create_dir_all(&session_dir).expect("create temp dir");

        let file_path = session_dir.join("rollout-2026-09-10T10-00-00-01a08e23-47ef-7f10-873d-9e9690da3839.jsonl");
        let mut file = File::create(&file_path).expect("create test file");
        writeln!(file, r#"{{"timestamp":"2026-09-10T10:00:00.000Z","type":"session_meta","payload":{{"session_id":"01a08e23-47ef-7f10-873d-9e9690da3839","cwd":"D:\\ly_project\\ecogrid-mdm"}}}}"#).unwrap();
        writeln!(file, r#"{{"timestamp":"2026-09-11T02:00:00.000Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"继续排查电表数据"}}]}}}}"#).unwrap();
        drop(file);

        let mut collected = Vec::new();
        collect_jsonl_files(&temp_dir, &mut collected).unwrap();
        assert_eq!(collected.len(), 1, "Should collect continued historical session file into current date");

        let session = parse_codex_session(&file_path, "2026-09-10T16:00:00.000Z", "2026-09-11T15:59:59.999Z")
            .expect("should parse continued session");
        assert_eq!(session.project_name, "ecogrid-mdm");
        assert_eq!(session.user_messages.len(), 1);
        assert!(session.user_messages[0].contains("继续排查电表数据"));

        let _ = fs::remove_dir_all(&temp_dir);

        // 2. Optional check on host machine sessions if full historical developer dataset is present
        if let Ok(home) = get_home_dir() {
            let codex_sep_11 = home.join(".codex").join("sessions").join("2026").join("09").join("11");
            if codex_sep_11.exists() {
                let query = CodexReportQuery {
                    from: "2026-09-10T16:00:00.000Z".to_string(),
                    to: "2026-09-11T15:59:59.999Z".to_string(),
                    providers: None,
                };
                if let Ok(res) = load_codex_report_sessions_sync(query) {
                    println!("Loaded {} host sessions for 2026-09-11", res.len());
                    // Only assert projects if historical datasets ecogrid-mdm and ami-insight are actually present in host files
                    if res.iter().any(|s| s.project_name == "ecogrid-mdm") {
                        assert!(res.iter().any(|s| s.project_name == "ami-insight"), "ami-insight should accompany ecogrid-mdm on 9-11");
                    }
                }
            }
        }
    }
}

