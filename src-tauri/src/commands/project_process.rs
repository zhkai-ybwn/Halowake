use serde::{Deserialize, Serialize};
#[cfg(any(test, not(target_os = "windows")))]
use std::process::Command;

#[cfg(test)]
use std::{
    collections::VecDeque,
    net::TcpListener,
    process::Stdio,
    sync::{Arc, Mutex},
};

use crate::storage::AppDatabase;
use tauri::{Manager, State};

mod log_parser;
mod process_runtime;

#[cfg(test)]
use log_parser::{
    append_detected_ports, detect_ports, detect_urls,
};
pub use process_runtime::ProjectProcessState;
use process_runtime::{
    list_processes, load_process_logs, restart_process, silent_command, start_process,
    start_resolved_command, stop_all_processes as stop_all_processes_runtime,
    stop_process_by_id,
};

#[cfg(test)]
use process_runtime::{
    configure_managed_command, decode_process_output, now_millis,
    package_manager_process_command, pid_is_alive, push_log, snapshot_process, stop_process,
    ManagedProcess, ProjectProcessMeta, LOG_LIMIT,
};

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
    list_processes(&state)
}

#[tauri::command]
pub async fn stop_project_process(
    process_id: String,
    state: State<'_, ProjectProcessState>,
) -> Result<ProjectProcessSnapshot, String> {
    stop_process_by_id(&state, &process_id)
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
    stop_all_processes_runtime(state)
}

#[tauri::command]
pub async fn restart_project_process(
    app: tauri::AppHandle,
    process_id: String,
    state: tauri::State<'_, ProjectProcessState>,
) -> Result<ProjectProcessSnapshot, String> {
    let database = app.try_state::<AppDatabase>().map(|db| db.inner().clone());
    restart_process(&state, &process_id, database)
}

#[tauri::command]
pub async fn load_project_process_logs(
    process_id: String,
    state: State<'_, ProjectProcessState>,
) -> Result<ProjectProcessLogs, String> {
    load_process_logs(&state, &process_id)
}

#[tauri::command]
pub async fn open_project_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("只支持打开 http 或 https 链接。".to_string());
    }

    open_url(&url)
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
    fn ignores_file_locations_and_bundle_line_numbers_in_port_detection() {
        let mut ports = Vec::new();
        append_detected_ports("chunk-vendors.js:35788:12", &mut ports);
        append_detected_ports("ERROR in ./src/app/component.ts:14884", &mut ports);
        append_detected_ports("at Object.run (vendor.js:52164)", &mut ports);
        append_detected_ports("main.ts:42544", &mut ports);
        append_detected_ports("styles.js:53312", &mut ports);
        assert!(ports.is_empty());

        append_detected_ports("Local: http://localhost:4204/", &mut ports);
        append_detected_ports("Server listening on port 3000", &mut ports);
        append_detected_ports("Ready at http://127.0.0.1:8080", &mut ports);
        ports.sort_unstable();
        assert_eq!(ports, vec![3000, 4204, 8080]);
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
            .args(["/D", "/S", "/C", "echo halowake-log"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run hidden command");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("halowake-log"));
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
