use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[tauri::command]
pub async fn load_codex_projects() -> Result<Vec<CodexProjectInfo>, String> {
    tokio::task::spawn_blocking(load_codex_projects_sync)
        .await
        .map_err(|e| format!("执行加载项目任务失败: {}", e))?
}

fn load_codex_projects_sync() -> Result<Vec<CodexProjectInfo>, String> {
    let home = get_home_dir()?;
    let mut project_map: HashMap<String, (String, usize, Option<String>, String)> = HashMap::new();

    // 1. Codex Projects
    let codex_dir = home.join(".codex").join("sessions");
    if codex_dir.exists() {
        let mut session_files = Vec::new();
        let _ = collect_jsonl_files(&codex_dir, &mut session_files);
        for path in &session_files {
            if let Some((cwd, timestamp)) = extract_codex_session_meta(path) {
                let name = get_project_name(&cwd);
                let entry = project_map.entry(cwd.clone()).or_insert_with(|| (name, 0, None, "codex".to_string()));
                entry.1 += 1;
                if let Some(ts) = timestamp {
                    if entry.2.as_ref().map_or(true, |existing| &ts > existing) {
                        entry.2 = Some(ts);
                    }
                }
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
                    let name = get_project_name(&cwd);
                    let mut session_files = Vec::new();
                    let _ = collect_jsonl_files(&path, &mut session_files);
                    let count = session_files.len();
                    if count > 0 {
                        let entry = project_map.entry(cwd.clone()).or_insert_with(|| (name, 0, None, "claude".to_string()));
                        entry.1 += count;
                    }
                }
            }
        }
    }

    // 3. Antigravity Projects
    let agy_config_projects = home.join(".gemini").join("config").join("projects");
    if agy_config_projects.exists() {
        if let Ok(entries) = fs::read_dir(&agy_config_projects) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(val) = serde_json::from_str::<Value>(&content) {
                            let name = val.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                            let updated_at = val.get("updatedAt").and_then(Value::as_str).map(str::to_string);
                            let folder_uri = val.pointer("/projectResources/resources/0/gitFolder/folderUri")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            let cwd = decode_uri_to_path(folder_uri).unwrap_or_else(|| name.clone());
                            if !name.is_empty() && name != "Outside of Project" {
                                let entry = project_map.entry(cwd).or_insert_with(|| (name, 0, None, "antigravity".to_string()));
                                entry.1 += 1;
                                if let Some(ts) = updated_at {
                                    if entry.2.as_ref().map_or(true, |existing| &ts > existing) {
                                        entry.2 = Some(ts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut projects: Vec<CodexProjectInfo> = project_map
        .into_iter()
        .map(|(cwd, (name, count, last_active, provider))| CodexProjectInfo {
            name,
            cwd,
            session_count: count,
            last_active_at: last_active,
            provider: Some(provider),
        })
        .collect();

    projects.sort_by(|a, b| {
        b.last_active_at
            .cmp(&a.last_active_at)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(projects)
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

    let to_date = if query.to.len() >= 10 { &query.to[..10] } else { &query.to };

    let mut all_sessions = Vec::new();

    // 1. Codex (with date directory pruning for future dates)
    if selected_providers.iter().any(|p| p == "codex" || p == "all") {
        let codex_dir = home.join(".codex").join("sessions");
        if codex_dir.exists() {
            let mut session_files = Vec::new();
            collect_codex_jsonl_files_pruned(&codex_dir, to_date, &mut session_files);
            for path in &session_files {
                if let Some(session) = parse_codex_session(path, &query.from, &query.to) {
                    all_sessions.push(session);
                }
            }
        }
    }

    // 2. Claude Code
    if selected_providers.iter().any(|p| p == "claude" || p == "all") {
        let claude_projects_dir = home.join(".claude").join("projects");
        if claude_projects_dir.exists() {
            let mut claude_files = Vec::new();
            let _ = collect_jsonl_files(&claude_projects_dir, &mut claude_files);
            for path in &claude_files {
                if let Some(session) = parse_claude_session(path, &query.from, &query.to) {
                    all_sessions.push(session);
                }
            }
        }
    }

    // 3. Antigravity (with mtime check & fast streaming reader)
    if selected_providers.iter().any(|p| p == "antigravity" || p == "all") {
        let brain_dir = home.join(".gemini").join("antigravity").join("brain");
        if brain_dir.exists() {
            let project_mappings = load_antigravity_project_mappings(&home);
            if let Ok(entries) = fs::read_dir(&brain_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let conv_dir = entry.path();
                    if conv_dir.is_dir() {
                        let conv_id = conv_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                        let transcript_file = conv_dir.join(".system_generated").join("logs").join("transcript.jsonl");
                        if transcript_file.exists() {
                            if let Some(session) = parse_antigravity_transcript(&transcript_file, conv_id, &project_mappings, &query.from, &query.to) {
                                all_sessions.push(session);
                            }
                        }
                    }
                }
            }
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
        for storage_dir in &opencode_candidates {
            if storage_dir.exists() {
                if let Ok(entries) = fs::read_dir(storage_dir) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.extension().is_some_and(|ext| ext == "json") {
                            if let Some(session) = parse_opencode_session(&path, &query.from, &query.to) {
                                all_sessions.push(session);
                            }
                        }
                    }
                }
            }
        }
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

                let _ = collect_jsonl_files(&day_path, files);
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
        if !line.contains("\"session_meta\"")
            && !line.contains("\"user\"")
            && !line.contains("\"user_message\"")
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
                    draft.cwd = Some(cwd.to_string());
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
    project_mappings: &HashMap<String, (String, String)>,
    from: &str,
    to: &str,
) -> Option<CodexReportSession> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let (project_name, cwd) = project_mappings
        .get(conv_id)
        .cloned()
        .unwrap_or_else(|| ("Antigravity Project".to_string(), "".to_string()));

    let mut draft = SessionDraft {
        id: conv_id.to_string(),
        provider: "antigravity".to_string(),
        cwd: if cwd.is_empty() { None } else { Some(cwd.clone()) },
        project_name: project_name.clone(),
        ..Default::default()
    };

    for line_res in reader.lines() {
        let Ok(line) = line_res else { continue; };
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
            if draft.cwd.is_none() {
                if let Some(tool_calls) = entry.get("tool_calls").and_then(Value::as_array) {
                    for call in tool_calls {
                        if let Some(args) = call.get("args") {
                            if let Some(dir) = args.get("SearchDirectory").or_else(|| args.get("Cwd")).and_then(Value::as_str) {
                                let dir_clean = dir.trim_matches('"');
                                draft.cwd = Some(dir_clean.to_string());
                                draft.project_name = get_project_name(dir_clean);
                                break;
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

fn load_antigravity_project_mappings(home: &Path) -> HashMap<String, (String, String)> {
    let mut mappings = HashMap::new();
    let config_projects = home.join(".gemini").join("config").join("projects");
    if config_projects.exists() {
        if let Ok(entries) = fs::read_dir(config_projects) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(val) = serde_json::from_str::<Value>(&content) {
                            let name = val.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                            let folder_uri = val.pointer("/projectResources/resources/0/gitFolder/folderUri")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            let cwd = decode_uri_to_path(folder_uri).unwrap_or_else(|| name.clone());
                            if !name.is_empty() && name != "Outside of Project" {
                                mappings.insert(name.clone(), (name, cwd));
                            }
                        }
                    }
                }
            }
        }
    }
    mappings
}

fn decode_claude_project_path(folder_name: &str) -> String {
    let s = folder_name.replace("__", "\\").replace('_', "\\");
    if s.len() > 2 && s.chars().nth(1) == Some(':') {
        s
    } else {
        folder_name.to_string()
    }
}

fn decode_uri_to_path(uri: &str) -> Option<String> {
    if let Some(stripped) = uri.strip_prefix("file:///") {
        let decoded = stripped.replace("%3A", ":").replace("%3a", ":").replace('/', "\\");
        return Some(decoded);
    }
    None
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
    let path = Path::new(cwd);
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
        let temp_dir = std::env::temp_dir().join(format!("lumina_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
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
}

