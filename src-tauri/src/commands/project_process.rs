use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, BufReader},
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
use encoding_rs::GBK;

use tauri::{Manager, State};
use crate::storage::{
    history_repository::{save_devdock_run_history_record, DevDockRunHistoryRecord},
    AppDatabase,
};

use super::project::read_project_manifest;
use super::project_executor::{build_process_command, resolve_executable, ResolvedCommand};

const LOG_LIMIT: usize = 500;
const PROCESS_LIMIT: usize = 40;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Default)]
pub struct ProjectProcessState {
    next_id: AtomicU64,
    processes: Mutex<HashMap<String, ManagedProcess>>,
}

impl Drop for ProjectProcessState {
    fn drop(&mut self) {
        if let Ok(processes) = self.processes.lock() {
            for process in processes.values() {
                if process_status(process).state == "running" {
                    kill_process_tree(process.meta.pid);
                    if let Ok(mut child) = process.child.lock() {
                        let _ = child.kill();
                    }
                }
            }
        }
    }
}

struct ManagedProcess {
    child: Arc<Mutex<Child>>,
    logs: Arc<Mutex<VecDeque<ProjectProcessLogLine>>>,
    detected_ports: Arc<Mutex<Vec<u16>>>,
    detected_urls: Arc<Mutex<Vec<String>>>,
    status: Arc<Mutex<ProjectProcessStatus>>,
    meta: ProjectProcessMeta,
}

#[derive(Clone)]
struct ProjectProcessMeta {
    id: String,
    project_path: String,
    project_name: String,
    script_name: String,
    command: String,
    package_manager: String,
    pid: u32,
    started_at: u128,
    command_id: Option<String>,
    command_name: Option<String>,
    executor: Option<String>,
    working_directory: Option<String>,
    config_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProcessSnapshot {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub script_name: String,
    pub command: String,
    pub package_manager: String,
    pub pid: u32,
    pub status: ProjectProcessStatus,
    pub started_at: u128,
    pub exited_at: Option<u128>,
    pub exit_code: Option<i32>,
    pub ports: Vec<u16>,
    pub urls: Vec<String>,
    pub log_count: usize,
    pub last_log_line: Option<String>,
    pub command_id: Option<String>,
    pub command_name: Option<String>,
    pub executor: Option<String>,
    pub command_preview: Option<String>,
    pub working_directory: Option<String>,
    pub config_revision: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProcessLogLine {
    pub stream: String,
    pub text: String,
    pub timestamp: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProcessLogs {
    pub process: ProjectProcessSnapshot,
    pub lines: Vec<ProjectProcessLogLine>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProcessStatus {
    pub state: String,
    pub exit_code: Option<i32>,
    pub exited_at: Option<u128>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartProjectProcessPayload {
    pub project_path: String,
    pub project_name: Option<String>,
    pub script_name: String,
    pub package_manager: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartProjectCommandPayload {
    pub project_path: String,
    pub command_id: String,
}

#[tauri::command]
pub async fn start_project_process(
    app: tauri::AppHandle,
    payload: StartProjectProcessPayload,
    state: tauri::State<'_, ProjectProcessState>,
) -> Result<ProjectProcessSnapshot, String> {
    let database = app.try_state::<AppDatabase>().map(|db| db.inner().clone());
    start_process(payload, &state, database)
}

#[tauri::command]
pub async fn start_project_command(
    app: tauri::AppHandle,
    payload: StartProjectCommandPayload,
    state: tauri::State<'_, ProjectProcessState>,
) -> Result<ProjectProcessSnapshot, String> {
    let database = app.try_state::<AppDatabase>().map(|db| db.inner().clone());
    start_resolved_command(payload, &state, database)
}

#[tauri::command]
pub async fn list_project_processes(
    state: State<'_, ProjectProcessState>,
) -> Result<Vec<ProjectProcessSnapshot>, String> {
    let mut processes = state
        .processes
        .lock()
        .map_err(|_| "读取进程列表失败。".to_string())?;
    prune_finished_processes(&mut processes);
    let mut latest = HashMap::<String, ProjectProcessSnapshot>::new();
    for snapshot in processes.values().map(snapshot_process) {
        let key = process_command_key(&snapshot.project_path, snapshot.command_id.as_deref().unwrap_or(&snapshot.script_name));
        let replace = latest.get(&key).map_or(true, |current| {
            (snapshot.started_at, snapshot.id.as_str()) > (current.started_at, current.id.as_str())
        });
        if replace {
            latest.insert(key, snapshot);
        }
    }
    let mut snapshots = latest.into_values().collect::<Vec<_>>();
    snapshots.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    Ok(snapshots)
}

#[tauri::command]
pub async fn stop_project_process(
    process_id: String,
    state: State<'_, ProjectProcessState>,
) -> Result<ProjectProcessSnapshot, String> {
    let process = find_process(&state, &process_id)?;
    stop_process(&process)?;
    Ok(snapshot_process(&process))
}

#[tauri::command]
pub async fn stop_all_project_processes(
    state: State<'_, ProjectProcessState>,
) -> Result<Vec<ProjectProcessSnapshot>, String> {
    stop_all_processes(&state)
}

pub fn stop_all_processes(
    state: &ProjectProcessState,
) -> Result<Vec<ProjectProcessSnapshot>, String> {
    let processes = state
        .processes
        .lock()
        .map_err(|_| "读取进程状态失败。".to_string())?
        .values()
        .map(clone_process)
        .collect::<Vec<_>>();
    let snapshots = processes.iter().map(snapshot_process).collect::<Vec<_>>();
    let mut failures = Vec::new();

    for process in &processes {
        if let Err(error) = stop_process(process) {
            failures.push(format!("{} · {}: {}", process.meta.project_name, process.meta.script_name, error));
        }
    }

    if !failures.is_empty() {
        return Err(format!("以下进程未能完全停止:\n{}", failures.join("\n")));
    }

    state
        .processes
        .lock()
        .map_err(|_| "清理进程状态失败。".to_string())?
        .clear();
    Ok(snapshots)
}

#[tauri::command]
pub async fn restart_project_process(
    app: tauri::AppHandle,
    process_id: String,
    state: tauri::State<'_, ProjectProcessState>,
) -> Result<ProjectProcessSnapshot, String> {
    let database = app.try_state::<AppDatabase>().map(|db| db.inner().clone());
    let process = find_process(&state, &process_id)?;
    let meta = process.meta.clone();
    stop_process(&process)?;
    if let Some(command_id) = meta.command_id {
        start_resolved_command(StartProjectCommandPayload { project_path: meta.project_path, command_id }, &state, database)
    } else {
        let payload = StartProjectProcessPayload {
            project_path: meta.project_path,
            project_name: Some(meta.project_name),
            script_name: meta.script_name,
            package_manager: meta.package_manager,
        };
        start_process(payload, &state, database)
    }
}

#[tauri::command]
pub async fn load_project_process_logs(
    process_id: String,
    state: State<'_, ProjectProcessState>,
) -> Result<ProjectProcessLogs, String> {
    let process = find_process(&state, &process_id)?;
    let lines = process
        .logs
        .lock()
        .map_err(|_| "读取进程日志失败。".to_string())?
        .iter()
        .cloned()
        .collect::<Vec<_>>();

    Ok(ProjectProcessLogs {
        process: snapshot_process(&process),
        lines,
    })
}

#[tauri::command]
pub async fn open_project_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("只支持打开 http 或 https 链接。".to_string());
    }

    open_url(&url)
}

fn start_process(
    payload: StartProjectProcessPayload,
    state: &ProjectProcessState,
    database: Option<AppDatabase>,
) -> Result<ProjectProcessSnapshot, String> {
    if !is_safe_script_name(&payload.script_name) {
        return Err("脚本名称包含不支持的字符。".to_string());
    }

    if let Some(existing) = find_active_script(state, &payload.project_path, &payload.script_name)? {
        return Ok(snapshot_process(&existing));
    }

    let manifest = read_project_manifest(&payload.project_path)?;
    let script = manifest
        .scripts
        .iter()
        .find(|script| script.name == payload.script_name)
        .ok_or_else(|| "package.json 中未找到该脚本。".to_string())?;
    let package_manager = package_manager_command(&payload.package_manager);
    let command_label = format!("{} run {}", package_manager, script.name);
    let project_name = payload
        .project_name
        .or(manifest.name)
        .unwrap_or_else(|| display_name_from_path(&payload.project_path));

    let mut command = package_manager_process_command(&package_manager, &script.name);
    configure_managed_command(&mut command, &payload.project_path);

    let mut child = command
        .spawn()
        .map_err(|e| format!("启动命令失败: {}", e))?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));
    let logs = Arc::new(Mutex::new(VecDeque::with_capacity(LOG_LIMIT)));
    let detected_ports = Arc::new(Mutex::new(Vec::new()));
    let detected_urls = Arc::new(Mutex::new(Vec::new()));
    let status = Arc::new(Mutex::new(ProjectProcessStatus {
        state: "running".to_string(),
        exit_code: None,
        exited_at: None,
    }));
    let id = format!("proc-{}-{}", now_millis(), state.next_id.fetch_add(1, Ordering::Relaxed));
    let meta = ProjectProcessMeta {
        id,
        project_path: payload.project_path,
        project_name,
        script_name: script.name.clone(),
        command: command_label,
        package_manager,
        pid,
        started_at: now_millis(),
        command_id: None,
        command_name: None,
        executor: None,
        working_directory: None,
        config_revision: None,
    };

    push_system_log(&logs, &detected_ports, &detected_urls, format!("准备执行：{}", meta.command));
    push_system_log(&logs, &detected_ports, &detected_urls, format!("工作目录：{}", meta.project_path));
    push_system_log(&logs, &detected_ports, &detected_urls, format!("进程已启动，PID {pid}"));

    if let Some(stdout) = stdout {
        spawn_log_reader(
            stdout,
            "stdout",
            Arc::clone(&logs),
            Arc::clone(&detected_ports),
            Arc::clone(&detected_urls),
        );
    }
    if let Some(stderr) = stderr {
        spawn_log_reader(
            stderr,
            "stderr",
            Arc::clone(&logs),
            Arc::clone(&detected_ports),
            Arc::clone(&detected_urls),
        );
    }
    spawn_waiter(
        Arc::clone(&child),
        Arc::clone(&status),
        Arc::clone(&logs),
        Arc::clone(&detected_ports),
        Arc::clone(&detected_urls),
        false,
        meta.clone(),
        database,
    );

    let managed = ManagedProcess {
        child,
        logs,
        detected_ports,
        detected_urls,
        status,
        meta,
    };
    let snapshot = snapshot_process(&managed);
    let mut processes = state
        .processes
        .lock()
        .map_err(|_| "保存进程状态失败。".to_string())?;
    processes.insert(snapshot.id.clone(), managed);
    prune_finished_processes(&mut processes);
    Ok(snapshot)
}

fn start_resolved_command(
    payload: StartProjectCommandPayload,
    state: &ProjectProcessState,
    database: Option<AppDatabase>,
) -> Result<ProjectProcessSnapshot, String> {
    if let Some(existing) = find_active_script(state, &payload.project_path, &payload.command_id)? {
        return Ok(snapshot_process(&existing));
    }
    let resolved = resolve_executable(&payload.project_path, &payload.command_id)?;
    let manifest = read_project_manifest(&payload.project_path)?;
    let project_name = manifest.name.unwrap_or_else(|| display_name_from_path(&payload.project_path));
    let mut command = build_process_command(&resolved);
    configure_managed_command(&mut command, resolved.working_directory.to_string_lossy().as_ref());
    spawn_resolved_process(state, payload.project_path, project_name, resolved, command, database)
}

fn spawn_resolved_process(
    state: &ProjectProcessState,
    project_path: String,
    project_name: String,
    resolved: ResolvedCommand,
    mut command: Command,
    database: Option<AppDatabase>,
) -> Result<ProjectProcessSnapshot, String> {
    let mut child = command.spawn().map_err(|error| format!("SPAWN_FAILED: {error}"))?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));
    let logs = Arc::new(Mutex::new(VecDeque::with_capacity(LOG_LIMIT)));
    let detected_ports = Arc::new(Mutex::new(Vec::new()));
    let detected_urls = Arc::new(Mutex::new(Vec::new()));
    let status = Arc::new(Mutex::new(ProjectProcessStatus { state: "running".to_string(), exit_code: None, exited_at: None }));
    let id = format!("run-{}-{}", now_millis(), state.next_id.fetch_add(1, Ordering::Relaxed));
    let meta = ProjectProcessMeta {
        id,
        project_path,
        project_name,
        script_name: resolved.command_id.clone(),
        command: resolved.command_preview.clone(),
        package_manager: resolved.executor.clone(),
        pid,
        started_at: now_millis(),
        command_id: Some(resolved.command_id),
        command_name: Some(resolved.command_name),
        executor: Some(resolved.executor),
        working_directory: Some(resolved.working_directory.to_string_lossy().to_string()),
        config_revision: Some(resolved.config_revision),
    };
    push_system_log(&logs, &detected_ports, &detected_urls, format!("准备执行：{}", meta.command));
    if let Some(directory) = &meta.working_directory {
        push_system_log(&logs, &detected_ports, &detected_urls, format!("工作目录：{directory}"));
    }
    push_system_log(&logs, &detected_ports, &detected_urls, format!("进程已启动，PID {pid}"));
    if let Some(stdout) = stdout { spawn_log_reader(stdout, "stdout", Arc::clone(&logs), Arc::clone(&detected_ports), Arc::clone(&detected_urls)); }
    if let Some(stderr) = stderr { spawn_log_reader(stderr, "stderr", Arc::clone(&logs), Arc::clone(&detected_ports), Arc::clone(&detected_urls)); }
    spawn_waiter(
        Arc::clone(&child),
        Arc::clone(&status),
        Arc::clone(&logs),
        Arc::clone(&detected_ports),
        Arc::clone(&detected_urls),
        true,
        meta.clone(),
        database,
    );
    let managed = ManagedProcess { child, logs, detected_ports, detected_urls, status, meta };
    let snapshot = snapshot_process(&managed);
    let mut processes = state.processes.lock().map_err(|_| "保存进程状态失败。".to_string())?;
    processes.insert(snapshot.id.clone(), managed);
    prune_finished_processes(&mut processes);
    Ok(snapshot)
}

fn find_process(state: &ProjectProcessState, process_id: &str) -> Result<ManagedProcess, String> {
    let processes = state
        .processes
        .lock()
        .map_err(|_| "读取进程状态失败。".to_string())?;
    processes
        .get(process_id)
        .map(clone_process)
        .ok_or_else(|| "进程不存在或已被清理。".to_string())
}

fn process_command_key(project_path: &str, command_id: &str) -> String {
    format!("{}::{command_id}", project_path.replace('\\', "/").to_lowercase())
}

fn find_active_script(
    state: &ProjectProcessState,
    project_path: &str,
    script_name: &str,
) -> Result<Option<ManagedProcess>, String> {
    let processes = state
        .processes
        .lock()
        .map_err(|_| "读取进程状态失败。".to_string())?;
    Ok(processes
        .values()
        .find(|process| {
            process.meta.project_path.eq_ignore_ascii_case(project_path)
                && process.meta.command_id.as_deref().unwrap_or(&process.meta.script_name) == script_name
                && process_is_active(process)
        })
        .map(clone_process))
}

fn stop_process(process: &ManagedProcess) -> Result<(), String> {
    if process_status(process).state != "running" {
        return Ok(());
    }

    let pid = process.meta.pid;
    kill_process_tree(pid);
    if let Ok(mut child) = process.child.lock() {
        let _ = child.kill();
    }
    for _ in 0..20 {
        if !pid_is_alive(pid) {
            if let Ok(mut status) = process.status.lock() {
                status.state = "stopped".to_string();
                status.exited_at = Some(now_millis());
            }
            push_system_log(
                &process.logs,
                &process.detected_ports,
                &process.detected_urls,
                format!("进程已停止，PID {pid}"),
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(format!("STOP_FAILED: PID {pid} 仍未结束"))
}

fn snapshot_process(process: &ManagedProcess) -> ProjectProcessSnapshot {
    let logs = process.logs.lock().ok();
    let ports = process.detected_ports.lock().map(|ports| ports.clone()).unwrap_or_default();
    let urls = process.detected_urls.lock().map(|urls| urls.clone()).unwrap_or_default();
    let status = process_status(process);
    let occupied_ports = if status.state == "stopped" {
        ports.iter().copied().filter(|port| is_port_listening(*port)).collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    ProjectProcessSnapshot {
        id: process.meta.id.clone(),
        project_path: process.meta.project_path.clone(),
        project_name: process.meta.project_name.clone(),
        script_name: process.meta.script_name.clone(),
        command: process.meta.command.clone(),
        package_manager: process.meta.package_manager.clone(),
        pid: process.meta.pid,
        status: status.clone(),
        started_at: process.meta.started_at,
        exited_at: status.exited_at,
        exit_code: status.exit_code,
        ports,
        urls,
        log_count: logs.as_ref().map(|lines| lines.len()).unwrap_or(0),
        last_log_line: logs
            .as_ref()
            .and_then(|lines| lines.back())
            .map(|line| line.text.clone()),
        command_id: process.meta.command_id.clone(),
        command_name: process.meta.command_name.clone(),
        executor: process.meta.executor.clone(),
        command_preview: Some(process.meta.command.clone()),
        working_directory: process.meta.working_directory.clone(),
        config_revision: process.meta.config_revision.clone(),
        warning: (!occupied_ports.is_empty()).then(|| format!("进程已停止，但端口 {} 仍被占用。", occupied_ports.iter().map(u16::to_string).collect::<Vec<_>>().join(", "))),
    }
}

#[allow(dead_code)]
fn detect_ports(lines: &VecDeque<ProjectProcessLogLine>) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in lines.iter().rev() {
        append_detected_ports(&line.text, &mut ports);
        if ports.len() >= 4 {
            break;
        }
    }
    ports.sort_unstable();
    ports
}

#[allow(dead_code)]
fn detect_urls(lines: &VecDeque<ProjectProcessLogLine>) -> Vec<String> {
    let mut urls = Vec::new();
    for line in lines.iter().rev() {
        append_detected_urls(&line.text, &mut urls);
    }
    urls
}

fn append_detected_ports(text: &str, ports: &mut Vec<u16>) {
    let text = strip_ansi(text);
    for token in text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != ':' && ch != '.' && ch != '/') {
        if let Some(port) = parse_port_token(token) {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
}

fn append_detected_urls(text: &str, urls: &mut Vec<String>) {
    let text = strip_ansi(text);
    for token in text.split_whitespace() {
        let candidate = token
            .trim_matches(|ch: char| matches!(ch, '(' | ')' | '[' | ']' | '<' | '>' | ',' | ';' | '"' | '\'' | '|' | '`' | '#' | '!'))
            .trim_end_matches('.')
            .trim_end_matches(':')
            .trim_end_matches(',');
        if (candidate.starts_with("http://") || candidate.starts_with("https://"))
            && is_localhost_url(candidate)
            && !urls.iter().any(|url| url == candidate)
        {
            urls.push(candidate.to_string());
        }
    }
}

fn is_localhost_url(url: &str) -> bool {
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_protocol
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    host == "localhost"
        || host == "127.0.0.1"
        || host == "0.0.0.0"
        || host.ends_with(".local")
        || is_private_ip(host)
}

fn is_private_ip(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    let nums: Option<Vec<u8>> = parts.iter().map(|p| p.parse::<u8>().ok()).collect();
    if let Some([a, b, _, _]) = nums.as_deref() {
        // 10.0.0.0/8
        if *a == 10 {
            return true;
        }
        // 172.16.0.0/12
        if *a == 172 && (16..=31).contains(b) {
            return true;
        }
        // 192.168.0.0/16
        if *a == 192 && *b == 168 {
            return true;
        }
        // 169.254.0.0/16 (link-local)
        if *a == 169 && *b == 254 {
            return true;
        }
    }
    false
}

fn process_is_active(process: &ManagedProcess) -> bool {
    process_status(process).state == "running"
}

fn strip_ansi(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        output.push(ch);
    }
    output
}

fn parse_port_token(token: &str) -> Option<u16> {
    let port_text = if let Some(index) = token.rfind(':') {
        &token[index + 1..]
    } else {
        return None;
    };
    let port = port_text.trim_matches('/').parse::<u16>().ok()?;
    if (1024..=65535).contains(&port) {
        Some(port)
    } else {
        None
    }
}

fn clone_process(process: &ManagedProcess) -> ManagedProcess {
    ManagedProcess {
        child: Arc::clone(&process.child),
        logs: Arc::clone(&process.logs),
        detected_ports: Arc::clone(&process.detected_ports),
        detected_urls: Arc::clone(&process.detected_urls),
        status: Arc::clone(&process.status),
        meta: process.meta.clone(),
    }
}

fn process_status(process: &ManagedProcess) -> ProjectProcessStatus {
    process.status.lock().map(|status| status.clone()).unwrap_or(ProjectProcessStatus {
        state: "unknown".to_string(),
        exit_code: None,
        exited_at: None,
    })
}

fn prune_finished_processes(processes: &mut HashMap<String, ManagedProcess>) {
    while processes.len() > PROCESS_LIMIT {
        let removable_id = processes
            .values()
            .filter(|process| process_status(process).state != "running")
            .min_by_key(|process| process.meta.started_at)
            .map(|process| process.meta.id.clone());
        if let Some(id) = removable_id {
            processes.remove(&id);
        } else {
            break;
        }
    }
}

fn spawn_log_reader<R>(
    reader: R,
    stream: &'static str,
    logs: Arc<Mutex<VecDeque<ProjectProcessLogLine>>>,
    detected_ports: Arc<Mutex<Vec<u16>>>,
    detected_urls: Arc<Mutex<Vec<String>>>,
)
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            let count = match reader.read_until(b'\n', &mut bytes) {
                Ok(count) => count,
                Err(_) => break,
            };
            if count == 0 { break; }
            while matches!(bytes.last(), Some(b'\n' | b'\r')) { bytes.pop(); }
            let line = decode_process_output(&bytes);
            push_log(
                &logs,
                &detected_ports,
                &detected_urls,
                ProjectProcessLogLine {
                    stream: stream.to_string(),
                    text: line,
                    timestamp: now_millis(),
                },
            );
        }
    });
}

fn decode_process_output(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    #[cfg(windows)]
    {
        let (decoded, _) = GBK.decode_without_bom_handling(bytes);
        return decoded.into_owned();
    }
    #[cfg(not(windows))]
    String::from_utf8_lossy(bytes).into_owned()
}

fn spawn_waiter(
    child: Arc<Mutex<Child>>,
    status: Arc<Mutex<ProjectProcessStatus>>,
    logs: Arc<Mutex<VecDeque<ProjectProcessLogLine>>>,
    detected_ports: Arc<Mutex<Vec<u16>>>,
    detected_urls: Arc<Mutex<Vec<String>>>,
    classify_exit: bool,
    meta: ProjectProcessMeta,
    database: Option<AppDatabase>,
) {
    thread::spawn(move || loop {
        let result = child.lock().ok().and_then(|mut child| child.try_wait().ok()).flatten();
        if let Some(exit_status) = result {
            let final_status = if let Ok(mut status) = status.lock() {
                let exit_code = exit_status.code();
                if status.state == "running" {
                    status.state = if classify_exit {
                        if exit_status.success() { "succeeded" } else { "failed" }
                    } else {
                        "exited"
                    }.to_string();
                }
                status.exit_code = exit_code;
                status.exited_at = Some(now_millis());
                status.clone()
            } else {
                ProjectProcessStatus {
                    state: "exited".to_string(),
                    exit_code: exit_status.code(),
                    exited_at: Some(now_millis()),
                }
            };
            let exit_description = exit_status.code().map_or_else(
                || "进程已退出，未返回退出码".to_string(),
                |code| format!("进程已退出，退出码 {code}"),
            );
            push_system_log(&logs, &detected_ports, &detected_urls, exit_description);

            if let Some(db) = database {
                let last_line = logs.lock().ok().and_then(|lines| lines.back().map(|l| l.text.clone()));
                let duration_ms = (now_millis().saturating_sub(meta.started_at as u128)) as i64;
                let record = DevDockRunHistoryRecord {
                    id: meta.id.clone(),
                    project_path: meta.project_path.clone(),
                    project_name: meta.project_name.clone(),
                    command_id: meta.command_id.clone().unwrap_or_else(|| meta.script_name.clone()),
                    command_name: meta.command_name.clone().unwrap_or_else(|| meta.script_name.clone()),
                    executor: meta.executor.clone().unwrap_or_else(|| meta.package_manager.clone()),
                    command_preview: Some(meta.command.clone()),
                    exit_code: final_status.exit_code,
                    status: final_status.state,
                    started_at: meta.started_at as i64,
                    duration_ms,
                    last_log_line: last_line,
                    expires_at: None,
                };
                let _ = save_devdock_run_history_record(&db, &record);
            }
            break;
        }
        if status.lock().map(|status| status.state != "running").unwrap_or(true) {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    });
}

fn push_system_log(
    logs: &Arc<Mutex<VecDeque<ProjectProcessLogLine>>>,
    detected_ports: &Arc<Mutex<Vec<u16>>>,
    detected_urls: &Arc<Mutex<Vec<String>>>,
    text: String,
) {
    push_log(
        logs,
        detected_ports,
        detected_urls,
        ProjectProcessLogLine { stream: "system".to_string(), text, timestamp: now_millis() },
    );
}

fn push_log(
    logs: &Arc<Mutex<VecDeque<ProjectProcessLogLine>>>,
    detected_ports: &Arc<Mutex<Vec<u16>>>,
    detected_urls: &Arc<Mutex<Vec<String>>>,
    line: ProjectProcessLogLine,
) {
    if let Ok(mut ports) = detected_ports.lock() {
        append_detected_ports(&line.text, &mut ports);
        ports.sort_unstable();
    }
    if let Ok(mut urls) = detected_urls.lock() {
        append_detected_urls(&line.text, &mut urls);
    }
    if let Ok(mut logs) = logs.lock() {
        if logs.len() >= LOG_LIMIT {
            logs.pop_front();
        }
        logs.push_back(line);
    }
}

fn silent_command(program: &str) -> Command {
    let mut command = Command::new(program);
    hide_command_window(&mut command);
    command
}

fn configure_managed_command(command: &mut Command, project_path: &str) {
    hide_command_window(command);
    command
        .current_dir(project_path)
        // Do not let npx download missing packages from a managed script. Its installer
        // can spawn an untracked Windows Terminal window; dependencies should be installed
        // explicitly in the project before DevDock starts the script.
        .env("npm_config_yes", "false")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}

#[cfg(windows)]
fn hide_command_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_command_window(_command: &mut Command) {}

fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = silent_command("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("pkill")
            .args(["-P", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        silent_command("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    Ok(())
}

fn is_port_listening(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_err() || TcpListener::bind(("::1", port)).is_err()
}

fn package_manager_command(package_manager: &str) -> String {
    if matches!(package_manager, "corepack pnpm" | "corepack yarn") {
        return package_manager.to_string();
    }
    let name = package_manager.split('@').next().unwrap_or(package_manager).trim();
    let has_version = package_manager.contains('@');
    match (name, has_version) {
        ("pnpm", true) => "corepack pnpm",
        ("yarn", true) => "corepack yarn",
        ("pnpm", false) => "pnpm",
        ("yarn", false) => "yarn",
        ("bun", _) => "bun",
        _ => "npm",
    }
    .to_string()
}

fn package_manager_process_command(package_manager: &str, script_name: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut command = silent_command("cmd");
        command.args([
            "/D",
            "/S",
            "/C",
            &format!("{} run {}", package_manager, script_name),
        ]);
        return command;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut parts = package_manager.split_whitespace();
        let program = parts.next().unwrap_or("npm");
        let mut command = silent_command(program);
        command.args(parts);
        command.args(["run", script_name]);
        command
    }
}

fn is_safe_script_name(script_name: &str) -> bool {
    !script_name.is_empty()
        && script_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-' | '.'))
}

fn display_name_from_path(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn pid_is_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let output = silent_command("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.lines().any(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .is_some_and(|value| value == pid.to_string())
                })
            }
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    {
        // On Unix, kill(pid, 0) checks if process exists without sending signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn decodes_gbk_output_from_cmd_without_replacement_characters() {
        assert_eq!(decode_process_output(&[0xB4, 0xED, 0xCE, 0xF3]), "错误");
    }

    #[test]
    fn detects_complete_urls_from_ansi_logs() {
        let lines = VecDeque::from([
            ProjectProcessLogLine {
                stream: "stdout".to_string(),
                text: "Local: http://localhost:3000/".to_string(),
                timestamp: 1,
            },
            ProjectProcessLogLine {
                stream: "stdout".to_string(),
                text: "Network: \u{1b}[36mhttp://192.168.1.20:3000/app\u{1b}[0m".to_string(),
                timestamp: 2,
            },
        ]);

        assert_eq!(
            detect_urls(&lines),
            vec![
                "http://192.168.1.20:3000/app".to_string(),
                "http://localhost:3000/".to_string(),
            ]
        );
    }

    #[test]
    fn detects_angular_url_before_later_log_output() {
        let mut lines = VecDeque::from([ProjectProcessLogLine {
            stream: "stdout".to_string(),
            text: "Angular Live Development Server is listening on 0.0.0.0:4300, open your browser on http://localhost:4300/".to_string(),
            timestamp: 1,
        }]);
        for index in 2..=102 {
            lines.push_back(ProjectProcessLogLine {
                stream: "stdout".to_string(),
                text: format!("later application log {index}"),
                timestamp: index,
            });
        }

        assert_eq!(detect_ports(&lines), vec![4300]);
        assert_eq!(detect_urls(&lines), vec!["http://localhost:4300/".to_string()]);
    }

    #[test]
    fn keeps_detected_endpoints_after_startup_log_is_evicted() {
        let logs = Arc::new(Mutex::new(VecDeque::with_capacity(LOG_LIMIT)));
        let detected_ports = Arc::new(Mutex::new(Vec::new()));
        let detected_urls = Arc::new(Mutex::new(Vec::new()));

        push_log(
            &logs,
            &detected_ports,
            &detected_urls,
            ProjectProcessLogLine {
                stream: "stdout".to_string(),
                text: "Local: http://localhost:4300/".to_string(),
                timestamp: 1,
            },
        );
        for index in 0..LOG_LIMIT {
            push_log(
                &logs,
                &detected_ports,
                &detected_urls,
                ProjectProcessLogLine {
                    stream: "stdout".to_string(),
                    text: format!("later application log {index}"),
                    timestamp: index as u128 + 2,
                },
            );
        }

        assert!(detect_urls(&logs.lock().unwrap()).is_empty());
        assert_eq!(*detected_ports.lock().unwrap(), vec![4300]);
        assert_eq!(
            *detected_urls.lock().unwrap(),
            vec!["http://localhost:4300/".to_string()]
        );
    }

    #[test]
    fn package_manager_command_uses_hidden_shell_on_windows() {
        let command = package_manager_process_command("corepack pnpm", "dev");

        #[cfg(windows)]
        {
            assert_eq!(command.get_program().to_string_lossy(), "cmd");
            let args = command
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            assert_eq!(args, vec!["/D", "/S", "/C", "corepack pnpm run dev"]);
        }

        #[cfg(not(windows))]
        {
            assert_eq!(command.get_program().to_string_lossy(), "corepack");
            let args = command
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            assert_eq!(args, vec!["pnpm", "run", "dev"]);
        }
    }

    #[test]
    fn managed_command_disables_npx_temporary_installs() {
        let mut command = Command::new("npm");
        configure_managed_command(&mut command, ".");

        let install_setting = command
            .get_envs()
            .find_map(|(key, value)| (key == "npm_config_yes").then_some(value))
            .flatten();
        assert_eq!(install_setting, Some(std::ffi::OsStr::new("false")));
    }

    #[cfg(windows)]
    #[test]
    fn hidden_command_preserves_piped_output() {
        let output = silent_command("cmd")
            .args(["/D", "/S", "/C", "echo lumina-log"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run hidden command");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("lumina-log"));
    }

    #[test]
    fn stop_process_terminates_managed_child() {
        #[cfg(windows)]
        let child = silent_command("cmd")
            .args(["/C", "ping 127.0.0.1 -n 30 > nul"])
            .spawn()
            .expect("spawn child");
        #[cfg(not(windows))]
        let child = Command::new("sh")
            .args(["-c", "sleep 30"])
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        let process = ManagedProcess {
            child: Arc::new(Mutex::new(child)),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            detected_ports: Arc::new(Mutex::new(Vec::new())),
            detected_urls: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(Mutex::new(ProjectProcessStatus {
                state: "running".to_string(),
                exit_code: None,
                exited_at: None,
            })),
            meta: ProjectProcessMeta {
                id: "test-process".to_string(),
                project_path: ".".to_string(),
                project_name: "test".to_string(),
                script_name: "test".to_string(),
                command: "test".to_string(),
                package_manager: "npm".to_string(),
                pid,
                started_at: now_millis(),
                command_id: None,
                command_name: None,
                executor: None,
                working_directory: None,
                config_revision: None,
            },
        };

        stop_process(&process).expect("stop child process");
        assert!(!pid_is_alive(pid));
    }

    #[test]
    fn stop_process_does_not_fail_for_an_unrelated_detected_port() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test port");
        let port = listener.local_addr().expect("read test address").port();
        #[cfg(windows)]
        let child = silent_command("cmd")
            .args(["/C", "ping 127.0.0.1 -n 30 > nul"])
            .spawn()
            .expect("spawn child");
        #[cfg(not(windows))]
        let child = Command::new("sh")
            .args(["-c", "sleep 30"])
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        let process = ManagedProcess {
            child: Arc::new(Mutex::new(child)),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            detected_ports: Arc::new(Mutex::new(vec![port])),
            detected_urls: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(Mutex::new(ProjectProcessStatus {
                state: "running".to_string(),
                exit_code: None,
                exited_at: None,
            })),
            meta: ProjectProcessMeta {
                id: "test-port-warning".to_string(),
                project_path: ".".to_string(),
                project_name: "test".to_string(),
                script_name: "test".to_string(),
                command: "test".to_string(),
                package_manager: "none".to_string(),
                pid,
                started_at: now_millis(),
                command_id: Some("config:test".to_string()),
                command_name: Some("test".to_string()),
                executor: Some("python".to_string()),
                working_directory: Some(".".to_string()),
                config_revision: Some("revision".to_string()),
            },
        };

        stop_process(&process).expect("stop managed process even while unrelated port remains open");
        assert!(!pid_is_alive(pid));
        assert!(snapshot_process(&process).warning.is_some());
        drop(listener);
    }
}

#[tauri::command]
pub async fn check_pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}
