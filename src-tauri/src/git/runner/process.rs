use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::git::models::{GitCommandProgressEvent, GitCommandResult};

use super::proxy::apply_git_proxy_env;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub const DEFAULT_GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_NETWORK_GIT_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_HEAVY_GIT_TIMEOUT: Duration = Duration::from_secs(60);
pub const GIT_CLONE_TIMEOUT: Duration = Duration::from_secs(120);

#[cfg(windows)]
pub struct ProcessJobGuard {
    handle: std::os::windows::io::RawHandle,
}

#[cfg(windows)]
unsafe impl Send for ProcessJobGuard {}
#[cfg(windows)]
unsafe impl Sync for ProcessJobGuard {}

#[cfg(windows)]
impl ProcessJobGuard {
    pub fn new() -> Option<Self> {
        use std::os::windows::io::RawHandle;
        use std::ptr::null_mut;

        #[repr(C)]
        struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
            per_process_user_time_limit: i64,
            per_job_user_time_limit: i64,
            limit_flags: u32,
            minimum_working_set_size: usize,
            maximum_working_set_size: usize,
            active_process_limit: u32,
            affinity: usize,
            priority_class: u32,
            scheduling_class: u32,
        }

        #[repr(C)]
        struct IO_COUNTERS {
            read_operation_count: u64,
            write_operation_count: u64,
            other_operation_count: u64,
            read_transfer_count: u64,
            write_transfer_count: u64,
            other_transfer_count: u64,
        }

        #[repr(C)]
        struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
            io_info: IO_COUNTERS,
            process_memory_limit: usize,
            job_memory_limit: usize,
            peak_process_memory_limit: usize,
            peak_job_memory_limit: usize,
        }

        const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
        const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;

        extern "system" {
            fn CreateJobObjectW(lpJobAttributes: *mut std::ffi::c_void, lpName: *const u16) -> RawHandle;
            fn SetInformationJobObject(
                hJob: RawHandle,
                JobObjectInformationClass: i32,
                lpJobObjectInformation: *const std::ffi::c_void,
                cbJobObjectInformationLength: u32,
            ) -> i32;
            fn CloseHandle(hObject: RawHandle) -> i32;
        }

        unsafe {
            let handle = CreateJobObjectW(null_mut(), std::ptr::null());
            if handle.is_null() || handle == -1isize as RawHandle {
                return None;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ret = SetInformationJobObject(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ret == 0 {
                CloseHandle(handle);
                return None;
            }
            Some(Self { handle })
        }
    }

    pub fn assign_child(&self, child: &std::process::Child) -> bool {
        use std::os::windows::io::AsRawHandle;
        extern "system" {
            fn AssignProcessToJobObject(
                hJob: std::os::windows::io::RawHandle,
                hProcess: std::os::windows::io::RawHandle,
            ) -> i32;
        }
        unsafe { AssignProcessToJobObject(self.handle, child.as_raw_handle()) != 0 }
    }

    pub fn terminate(&self) {
        extern "system" {
            fn TerminateJobObject(hJob: std::os::windows::io::RawHandle, uExitCode: u32) -> i32;
        }
        unsafe {
            let _ = TerminateJobObject(self.handle, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessJobGuard {
    fn drop(&mut self) {
        extern "system" {
            fn CloseHandle(hObject: std::os::windows::io::RawHandle) -> i32;
        }
        unsafe {
            if !self.handle.is_null() && self.handle != -1isize as std::os::windows::io::RawHandle {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(unix)]
pub struct ProcessJobGuard;

#[cfg(unix)]
impl ProcessJobGuard {
    pub fn new() -> Option<Self> {
        Some(Self)
    }
    pub fn assign_child(&self, _child: &std::process::Child) -> bool {
        true
    }
    pub fn terminate(&self) {}
}

pub fn kill_process_tree(child: &mut std::process::Child, _job: &Option<ProcessJobGuard>) {
    #[cfg(windows)]
    if let Some(ref j) = _job {
        j.terminate();
    }
    #[cfg(unix)]
    unsafe {
        let pid = child.id() as i32;
        if pid > 0 {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    #[cfg(windows)]
    {
        let pid = child.id();
        if pid > 0 {
            let mut kill_cmd = Command::new("taskkill");
            kill_cmd.args(["/F", "/T", "/PID", &pid.to_string()]);
            kill_cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = kill_cmd.status();
        }
    }
}

pub fn run_command_with_timeout(
    mut cmd: Command,
    timeout: Duration,
    label: &str,
) -> Result<Output, String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("执行命令失败 ({label}): {e}"))?;

    let job = match ProcessJobGuard::new() {
        Some(guard) => {
            if !guard.assign_child(&child) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("将子进程加入隔离任务失败 ({label}): 权限不足或系统限制"));
            }
            Some(guard)
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("创建进程隔离任务失败 ({label}): 系统调用失败"));
        }
    };

    let (stdout_tx, stdout_rx) = mpsc::channel();
    let (stderr_tx, stderr_rx) = mpsc::channel();

    let stdout_handle = child.stdout.take().map(|mut reader| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf);
            let _ = stdout_tx.send(buf);
        })
    });
    let stderr_handle = child.stderr.take().map(|mut reader| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf);
            let _ = stderr_tx.send(buf);
        })
    });

    let start = Instant::now();
    let deadline = start + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    kill_process_tree(&mut child, &job);
                    let _ = child.wait();
                    return Err(format!(
                        "执行命令超时 ({}s): {}",
                        timeout.as_secs().max(1),
                        label
                    ));
                }
                thread::sleep(Duration::from_millis(15));
            }
            Err(e) => {
                kill_process_tree(&mut child, &job);
                let _ = child.wait();
                return Err(format!("等待命令结束失败 ({label}): {e}"));
            }
        }
    };

    // Process exited, now collect stdout and stderr strictly within remaining absolute deadline
    let now = Instant::now();
    if now >= deadline {
        kill_process_tree(&mut child, &job);
        let _ = child.wait();
        return Err(format!(
            "命令执行或管道收尾超时 ({}s): {}",
            timeout.as_secs().max(1),
            label
        ));
    }
    let remaining_stdout = deadline.saturating_duration_since(now);
    let stdout = match stdout_rx.recv_timeout(remaining_stdout) {
        Ok(data) => {
            if let Some(h) = stdout_handle {
                let _ = h.join();
            }
            data
        }
        Err(_) => {
            kill_process_tree(&mut child, &job);
            let _ = child.wait();
            // CRITICAL (S02): MUST return Err on stdout drain timeout! Never pretend Ok!
            return Err(format!(
                "等待命令输出管道收尾超时 ({}s): {}",
                timeout.as_secs().max(1),
                label
            ));
        }
    };

    let now = Instant::now();
    if now >= deadline {
        kill_process_tree(&mut child, &job);
        let _ = child.wait();
        return Err(format!(
            "命令执行或管道收尾超时 ({}s): {}",
            timeout.as_secs().max(1),
            label
        ));
    }
    let remaining_stderr = deadline.saturating_duration_since(now);
    let stderr = match stderr_rx.recv_timeout(remaining_stderr) {
        Ok(data) => {
            if let Some(h) = stderr_handle {
                let _ = h.join();
            }
            data
        }
        Err(_) => {
            kill_process_tree(&mut child, &job);
            let _ = child.wait();
            // CRITICAL (S02): MUST return Err on stderr drain timeout! Never pretend Ok!
            return Err(format!(
                "等待命令错误管道收尾超时 ({}s): {}",
                timeout.as_secs().max(1),
                label
            ));
        }
    };

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

pub fn run_git_with_timeout(
    repo_path: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, String> {
    let mut cmd = git_command(repo_path);
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let label = format!("git {}", args.join(" "));
    let output = run_command_with_timeout(cmd, timeout, &label)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {:?} 执行失败", args)
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run_git(repo_path: &str, args: &[&str]) -> Result<String, String> {
    run_git_with_timeout(repo_path, args, DEFAULT_GIT_COMMAND_TIMEOUT)
}

pub fn run_git_raw(repo_path: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = git_command(repo_path);
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let label = format!("git {}", args.join(" "));
    let output = run_command_with_timeout(cmd, DEFAULT_GIT_COMMAND_TIMEOUT, &label)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {:?} 执行失败", args)
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(super) fn run_git_raw_owned(repo_path: &str, args: &[String]) -> Result<String, String> {
    let mut cmd = git_command(repo_path);
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let label = format!("git {}", args.join(" "));
    let output = run_command_with_timeout(cmd, DEFAULT_GIT_COMMAND_TIMEOUT, &label)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {:?} 执行失败", args)
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(super) fn run_git_capture(
    repo_path: &str,
    args: &[&str],
) -> Result<GitCommandResult, String> {
    let command = format!("git {}", args.join(" "));
    let mut cmd = git_command(repo_path);
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = run_command_with_timeout(cmd, DEFAULT_GIT_COMMAND_TIMEOUT, &command)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let suggestion = git_error_suggestion(&stderr);
        return Err(format_command_error(
            &command,
            &stdout,
            &stderr,
            suggestion.as_deref(),
        ));
    }

    Ok(GitCommandResult {
        command,
        message: if stdout.trim().is_empty() {
            "Command completed".to_string()
        } else {
            stdout.trim().to_string()
        },
        stdout,
        stderr,
        suggestion: None,
    })
}

pub(super) fn run_git_capture_streaming<F>(
    repo_path: &str,
    args: &[&str],
    on_progress: F,
) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    run_git_capture_streaming_with_timeout(repo_path, args, DEFAULT_NETWORK_GIT_TIMEOUT, on_progress)
}

pub(super) fn run_git_capture_streaming_with_timeout<F>(
    repo_path: &str,
    args: &[&str],
    timeout: Duration,
    mut on_progress: F,
) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let command_label = format!("git {}", args.join(" "));
    let mut cmd = git_command(repo_path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;

    let job = {
        let j = ProcessJobGuard::new();
        if let Some(ref guard) = j {
            guard.assign_child(&child);
        }
        j
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel::<(String, Vec<u8>)>();

    if let Some(stdout) = stdout {
        spawn_git_output_reader(stdout, "stdout", tx.clone());
    }
    if let Some(stderr) = stderr {
        spawn_git_output_reader(stderr, "stderr", tx.clone());
    }
    drop(tx);

    let start = Instant::now();
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();

    loop {
        if start.elapsed() >= timeout {
            kill_process_tree(&mut child, &job);
            let _ = child.wait();
            return Err(format!("网络 Git 操作超时 ({}s): {}", timeout.as_secs().max(1), command_label));
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok((stream, chunk)) => {
                let text = String::from_utf8_lossy(&chunk).to_string();
                if stream == "stderr" {
                    stderr_text.push_str(&text);
                } else {
                    stdout_text.push_str(&text);
                }
                let event = build_progress_event(repo_path, &command_label, &stream, &text);
                on_progress(event);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(Some(_)) = child.try_wait() {
                    while let Ok((stream, chunk)) = rx.try_recv() {
                        let text = String::from_utf8_lossy(&chunk).to_string();
                        if stream == "stderr" {
                            stderr_text.push_str(&text);
                        } else {
                            stdout_text.push_str(&text);
                        }
                    }
                    break;
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    kill_process_tree(&mut child, &job);
                    let _ = child.wait();
                    return Err(format!("网络 Git 操作超时 ({}s): {}", timeout.as_secs().max(1), command_label));
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                kill_process_tree(&mut child, &job);
                let _ = child.wait();
                return Err(format!("等待 git 命令结束失败 {:?}: {}", args, e));
            }
        }
    };

    if !status.success() {
        let suggestion = git_error_suggestion(&stderr_text);
        return Err(format_command_error(
            &command_label,
            &stdout_text,
            &stderr_text,
            suggestion.as_deref(),
        ));
    }

    Ok(GitCommandResult {
        command: command_label,
        message: if stdout_text.trim().is_empty() {
            "Command completed".to_string()
        } else {
            stdout_text.trim().to_string()
        },
        stdout: stdout_text,
        stderr: stderr_text,
        suggestion: None,
    })
}

pub(super) fn run_git_capture_status(
    repo_path: &str,
    args: &[&str],
) -> Result<(bool, GitCommandResult), String> {
    let output = git_command(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let command = format!("git {}", args.join(" "));
    let success = output.status.success();
    let message = if stdout.trim().is_empty() {
        if success {
            "Command completed".to_string()
        } else {
            "Command failed".to_string()
        }
    } else {
        stdout.trim().to_string()
    };

    Ok((
        success,
        GitCommandResult {
            command,
            message,
            stdout,
            stderr,
            suggestion: None,
        },
    ))
}

pub(super) fn git_command(repo_path: &str) -> Command {
    let mut command = silent_command("git");
    command.args(["-c", "core.quotePath=false"]);
    command.current_dir(repo_path);
    apply_git_proxy_env(&mut command, repo_path);
    command.stdin(Stdio::null());
    command.env("GIT_TERMINAL_PROMPT", "0");
    command
}

pub(super) fn silent_command(program: &str) -> Command {
    let mut command = Command::new(program);
    hide_command_window(&mut command);
    command
}

#[cfg(windows)]
fn hide_command_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_command_window(_command: &mut Command) {}

pub(super) fn format_command_error(
    command: &str,
    stdout: &str,
    stderr: &str,
    suggestion: Option<&str>,
) -> String {
    let mut message = format!("> {}\n", command);
    if !stdout.trim().is_empty() {
        message.push_str(stdout.trim());
        message.push('\n');
    }
    if !stderr.trim().is_empty() {
        message.push_str(stderr.trim());
    }
    if let Some(suggestion) = suggestion {
        message.push_str("\n\n建议: ");
        message.push_str(suggestion);
    }
    message
}

pub(super) fn git_error_suggestion(stderr: &str) -> Option<String> {
    let lower = stderr.to_lowercase();
    if lower.contains("ambiguous argument 'head'") || lower.contains("unknown revision") {
        Some(
            "当前仓库还没有首个提交。请先完成 Commit，再使用 Push -u origin main 推送到 GitHub。"
                .to_string(),
        )
    } else if lower.contains("no such ref was fetched") {
        Some(
            "当前分支配置的 upstream 在远端不存在。请重新设置上游分支，或确认远端默认分支名称。"
                .to_string(),
        )
    } else if lower.contains("no tracking information") || lower.contains("no upstream") {
        Some("当前分支没有 upstream。请先 push -u 设置上游分支，或选择远端分支。".to_string())
    } else if lower.contains("would be overwritten") {
        Some("本地改动会被覆盖。请先提交、暂存或撤销相关改动后再拉取。".to_string())
    } else if lower.contains("non-fast-forward") || lower.contains("fetch first") {
        Some("远端包含本地没有的提交。请先拉取远端变更，再重新推送。".to_string())
    } else if lower.contains("failed to connect")
        || lower.contains("could not connect to server")
        || lower.contains("connection timed out")
        || lower.contains("unable to access")
    {
        Some("Git 已进入网络连接阶段，但无法连接远端服务器。请检查 GitHub/GitLab 网络、代理/VPN、公司防火墙，或在命令行执行 git ls-remote <remote-url> 验证连通性。若命令行可连接但客户端失败，通常是客户端启动环境没有继承代理，可重启客户端或配置 Git 的 http.proxy/https.proxy。这个错误通常不是 user.name/email 导致的。".to_string())
    } else {
        None
    }
}

pub(super) fn spawn_git_output_reader<R>(
    mut reader: R,
    stream: &'static str,
    sender: mpsc::Sender<(String, Vec<u8>)>,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    if sender
                        .send((stream.to_string(), buffer[..count].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

pub(super) fn build_progress_event(
    repo_path: &str,
    command: &str,
    stream: &str,
    text: &str,
) -> GitCommandProgressEvent {
    let cleaned = strip_ansi_sequences(text);
    GitCommandProgressEvent {
        repo_path: repo_path.to_string(),
        command: command.to_string(),
        stream: stream.to_string(),
        text: cleaned.clone(),
        phase: parse_progress_phase(&cleaned),
        percent: parse_progress_percent(&cleaned),
        transfer: parse_transfer_text(&cleaned),
    }
}

pub(super) fn emit_manual_git_progress<F>(
    repo_path: &str,
    command: &str,
    phase: &str,
    percent: Option<u8>,
    transfer: Option<String>,
    on_progress: &mut F,
) where
    F: FnMut(GitCommandProgressEvent),
{
    on_progress(GitCommandProgressEvent {
        repo_path: repo_path.to_string(),
        command: command.to_string(),
        stream: "stdout".to_string(),
        text: format!(
            "{}{}\n",
            phase,
            transfer
                .as_ref()
                .map(|value| format!(": {}", value))
                .unwrap_or_default()
        ),
        phase: Some(phase.to_string()),
        percent,
        transfer,
    });
}

fn strip_ansi_sequences(text: &str) -> String {
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

pub(super) fn parse_progress_phase(text: &str) -> Option<String> {
    let line = latest_progress_line(text)?;
    let (phase, _) = line.split_once(':')?;
    let phase = phase.trim();
    if phase.is_empty() {
        None
    } else {
        Some(phase.to_string())
    }
}

pub(super) fn parse_progress_percent(text: &str) -> Option<u8> {
    let line = latest_progress_line(text)?;
    let percent_index = line.find('%')?;
    let prefix = &line[..percent_index];
    let digits = prefix
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit() || ch.is_ascii_whitespace())
        .collect::<String>();
    let percent = digits
        .chars()
        .rev()
        .collect::<String>()
        .trim()
        .parse::<u8>()
        .ok()?;
    Some(percent.min(100))
}

pub(super) fn parse_transfer_text(text: &str) -> Option<String> {
    let line = latest_progress_line(text)?;
    let lower = line.to_ascii_lowercase();
    if !(lower.contains("bytes")
        || lower.contains("kib")
        || lower.contains("mib")
        || lower.contains("gib"))
    {
        return None;
    }
    line.split(',')
        .skip(1)
        .map(str::trim)
        .find(|part| {
            let lower = part.to_ascii_lowercase();
            lower.contains("bytes")
                || lower.contains("kib")
                || lower.contains("mib")
                || lower.contains("gib")
        })
        .map(ToString::to_string)
}

fn latest_progress_line(text: &str) -> Option<&str> {
    text.split(['\r', '\n'])
        .rev()
        .map(str::trim)
        .find(|line| line.contains(':') && line.contains('%'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_run_command_with_timeout_kills_hanging_process() {
        let cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "ping -n 10 127.0.0.1 > nul"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("10");
            c
        };

        let start = Instant::now();
        let result = run_command_with_timeout(cmd, Duration::from_millis(150), "test-hang");
        let elapsed = start.elapsed();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("超时"));
        assert!(elapsed < Duration::from_secs(3), "Should timeout quickly, took {:?}", elapsed);
    }

    #[test]
    fn test_run_command_with_timeout_parent_exits_child_holds_pipe_returns_err_and_bounded() {
        // Reproduction of S02: parent exits immediately, but spawned child process inherits stdout and sleeps for 2000ms.
        // Configured timeout is 200ms. Must return Err, must NOT return Ok(Output) with empty vec, and elapsed must be bounded.
        let cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args([
                "/c",
                "powershell -NoProfile -Command \"Start-Process -NoNewWindow ping -ArgumentList '127.0.0.1','-n','4'; exit 0\"",
            ]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 3 & exit 0"]);
            c
        };

        let start = Instant::now();
        let result = run_command_with_timeout(cmd, Duration::from_millis(200), "test-parent-exit-child-pipe");
        let elapsed = start.elapsed();

        // Must return Err (either wait timeout or pipe drain timeout), NOT Ok!
        assert!(result.is_err(), "Must return Err on pipe timeout, but got {:?}", result);
        let err = result.unwrap_err();
        assert!(err.contains("超时"), "Error message should mention timeout: {}", err);
        // Elapsed must be strictly bounded well below the 4000ms ping duration of the child
        assert!(elapsed < Duration::from_millis(2500), "Elapsed time must be bounded: {:?}", elapsed);
    }

    #[test]
    fn test_run_command_with_timeout_allows_legitimate_slow_operations() {
        // S02 requirement: Do NOT use instant echo to fake slow operation test.
        // Actually perform a slow operation (taking 150-300ms) with a generous 3000ms timeout.
        let cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "ping 127.0.0.1 -n 2 > nul & echo slow-operation-completed"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 0.2 && echo slow-operation-completed"]);
            c
        };

        let start = Instant::now();
        let result = run_command_with_timeout(cmd, Duration::from_secs(8), "test-legitimate-slow");
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Legitimate slow operation should succeed, but got: {:?}", result);
        let output = result.unwrap();
        assert!(output.status.success());
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        assert!(stdout_str.contains("slow-operation-completed"));
        // Confirm it actually took non-trivial time, verifying it was legitimately slow
        assert!(elapsed >= Duration::from_millis(80), "Legitimate slow operation took {:?}", elapsed);
        assert!(elapsed < Duration::from_secs(8), "Should complete within deadline, took {:?}", elapsed);
    }
}
