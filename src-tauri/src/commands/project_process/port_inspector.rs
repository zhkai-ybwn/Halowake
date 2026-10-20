use std::collections::HashMap;
#[cfg(not(windows))]
use std::process::Command;
use serde::{Deserialize, Serialize};

use super::process_runtime::{
    decode_process_output, kill_process_tree, pid_is_alive, silent_command,
    stop_process_by_pid, ProjectProcessState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub is_devdock_managed: bool,
    pub managed_process_id: Option<String>,
    pub project_name: Option<String>,
    pub command_name: Option<String>,
}

#[derive(Debug, Clone)]
struct RawPortRecord {
    pid: u32,
    protocol: String,
    local_address: String,
    state: String,
}

pub fn parse_port_from_addr(addr: &str) -> Option<u16> {
    let colon_pos = addr.rfind(':')?;
    addr[colon_pos + 1..].parse::<u16>().ok()
}

#[cfg(windows)]
fn query_platform_port_records(target_port: u16) -> Result<Vec<RawPortRecord>, String> {
    let output = silent_command("netstat")
        .args(["-ano"])
        .output()
        .map_err(|e| format!("执行 netstat 命令失败: {e}"))?;

    let text = decode_process_output(&output.stdout);
    let mut records = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let proto = parts[0].to_uppercase();
        if proto != "TCP" && proto != "UDP" {
            continue;
        }

        let local_addr = parts[1];
        if let Some(port) = parse_port_from_addr(local_addr) {
            if port == target_port {
                let (state, pid_str) = if proto == "TCP" {
                    if parts.len() >= 5 {
                        (parts[3].to_uppercase(), parts[4])
                    } else {
                        continue;
                    }
                } else {
                    ("LISTENING".to_string(), parts[parts.len() - 1])
                };

                if let Ok(pid) = pid_str.parse::<u32>() {
                    records.push(RawPortRecord {
                        pid,
                        protocol: proto,
                        local_address: local_addr.to_string(),
                        state,
                    });
                }
            }
        }
    }
    Ok(records)
}

#[cfg(not(windows))]
fn query_platform_port_records(target_port: u16) -> Result<Vec<RawPortRecord>, String> {
    let output = Command::new("lsof")
        .args(["-i", &format!(":{target_port}"), "-n", "-P"])
        .output();

    let mut records = Vec::new();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for (i, line) in text.lines().enumerate() {
            if i == 0 || line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                if let Ok(pid) = parts[1].parse::<u32>() {
                    let proto = parts[7].to_uppercase();
                    let node_name = parts[8..].join(" ");
                    let state = if node_name.contains("LISTEN") {
                        "LISTENING".to_string()
                    } else {
                        "ESTABLISHED".to_string()
                    };
                    records.push(RawPortRecord {
                        pid,
                        protocol: proto,
                        local_address: node_name,
                        state,
                    });
                }
            }
        }
    }
    Ok(records)
}

#[cfg(windows)]
fn get_process_details(pid: u32) -> (String, Option<String>) {
    if pid == 0 {
        return ("System Idle Process".to_string(), None);
    }
    if pid == 4 {
        return ("System".to_string(), None);
    }

    extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
        fn QueryFullProcessImageNameW(
            hProcess: *mut std::ffi::c_void,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> i32;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !handle.is_null() {
            let mut buffer = [0u16; 1024];
            let mut size = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size);
            CloseHandle(handle);
            if ok != 0 && size > 0 {
                let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
                let name = std::path::Path::new(&full_path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| full_path.clone());
                return (name, Some(full_path));
            }
        }
    }

    // Fallback: tasklist /FI "PID eq <pid>" /FO CSV /NH
    if let Ok(output) = silent_command("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
    {
        let text = decode_process_output(&output.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('"') {
                if let Some(end) = trimmed[1..].find('"') {
                    let name = trimmed[1..=end].to_string();
                    if !name.is_empty() && !name.starts_with("INFO:") {
                        return (name, None);
                    }
                }
            }
        }
    }

    (format!("PID {pid}"), None)
}

#[cfg(not(windows))]
fn get_process_details(pid: u32) -> (String, Option<String>) {
    if let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) {
        let path = exe.to_string_lossy().to_string();
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        return (name, Some(path));
    }
    if let Ok(output) = Command::new("ps").args(["-p", &pid.to_string(), "-o", "comm="]).output() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return (name, None);
        }
    }
    (format!("PID {pid}"), None)
}

pub fn find_processes(target_port: u16, state: &ProjectProcessState) -> Result<Vec<PortProcessInfo>, String> {
    if target_port == 0 {
        return Ok(Vec::new());
    }

    let records = query_platform_port_records(target_port)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }

    // Group records by PID to consolidate multiple sockets for the same process
    let mut grouped: HashMap<u32, Vec<RawPortRecord>> = HashMap::new();
    for record in records {
        grouped.entry(record.pid).or_default().push(record);
    }

    let mut result = Vec::new();
    for (pid, rows) in grouped {
        // Prioritize LISTENING state if present
        let primary = rows
            .iter()
            .find(|r| r.state == "LISTENING")
            .unwrap_or(&rows[0]);

        let state_name = primary.state.clone();
        let protocol = primary.protocol.clone();

        // Unique local addresses
        let mut addresses = Vec::new();
        for r in &rows {
            if !addresses.contains(&r.local_address) {
                addresses.push(r.local_address.clone());
            }
        }
        let local_address = addresses.join(", ");

        let (name, path) = get_process_details(pid);
        let managed_meta = state.find_managed_meta_by_pid(pid);
        let is_devdock_managed = managed_meta.is_some();
        let (managed_process_id, project_name, command_name) = match managed_meta {
            Some((id, proj, cmd)) => (Some(id), Some(proj), Some(cmd)),
            None => (None, None, None),
        };

        result.push(PortProcessInfo {
            pid,
            name,
            path,
            protocol,
            local_address,
            state: state_name,
            is_devdock_managed,
            managed_process_id,
            project_name,
            command_name,
        });
    }

    // Sort: DevDock managed first, then LISTENING first, then by PID
    result.sort_by(|a, b| {
        b.is_devdock_managed
            .cmp(&a.is_devdock_managed)
            .then_with(|| (b.state == "LISTENING").cmp(&(a.state == "LISTENING")))
            .then_with(|| a.pid.cmp(&b.pid))
    });

    Ok(result)
}

pub fn kill_process(pid: u32, state: &ProjectProcessState) -> Result<bool, String> {
    if pid == 0 || pid == 4 {
        return Err("无法终止系统核心保护进程。".to_string());
    }

    // 1. If managed by DevDock, trigger DevDock's managed process stop logic
    let _ = stop_process_by_pid(state, pid);

    // 2. Kill the process tree
    kill_process_tree(pid);

    // 3. Verify exit
    for _ in 0..12 {
        if !pid_is_alive(pid) {
            return Ok(true);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // 4. On Windows, retry forceful kill with taskkill /F /T
    #[cfg(target_os = "windows")]
    {
        let _ = silent_command("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .output();
        for _ in 0..6 {
            if !pid_is_alive(pid) {
                return Ok(true);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    if !pid_is_alive(pid) {
        Ok(true)
    } else {
        Err(format!("未能停止进程 PID {pid}，该进程可能受系统保护或需要管理员权限。"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn test_parse_port_from_addr() {
        assert_eq!(parse_port_from_addr("0.0.0.0:8080"), Some(8080));
        assert_eq!(parse_port_from_addr("127.0.0.1:3000"), Some(3000));
        assert_eq!(parse_port_from_addr("[::]:5173"), Some(5173));
        assert_eq!(parse_port_from_addr("[::1]:4200"), Some(4200));
        assert_eq!(parse_port_from_addr("*:9000"), Some(9000));
        assert_eq!(parse_port_from_addr("invalid"), None);
    }

    #[test]
    fn test_scan_listening_port() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral port");
        let local_port = listener.local_addr().unwrap().port();
        let state = ProjectProcessState::default();
        let processes = find_processes(local_port, &state).expect("Failed to find processes");
        assert!(!processes.is_empty(), "Should find the listener process for port {local_port}");
        let my_pid = std::process::id();
        assert!(
            processes.iter().any(|p| p.pid == my_pid),
            "Should include current test process PID {my_pid}"
        );
        drop(listener);
    }
}
