use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::git::models::{GitCommandProgressEvent, GitCommandResult};

use super::proxy::apply_git_proxy_env;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn run_git(repo_path: &str, args: &[&str]) -> Result<String, String> {
    let output = git_command(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;

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

pub fn run_git_raw(repo_path: &str, args: &[&str]) -> Result<String, String> {
    let output = git_command(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;

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
    let output = git_command(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;

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
    let output = git_command(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let command = format!("git {}", args.join(" "));

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
    mut on_progress: F,
) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let command_label = format!("git {}", args.join(" "));
    let mut child = git_command(repo_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("执行 git 命令失败 {:?}: {}", args, e))?;

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

    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    for (stream, chunk) in rx {
        let text = String::from_utf8_lossy(&chunk).to_string();
        if stream == "stderr" {
            stderr_text.push_str(&text);
        } else {
            stdout_text.push_str(&text);
        }
        let event = build_progress_event(repo_path, &command_label, &stream, &text);
        on_progress(event);
    }

    let status = child
        .wait()
        .map_err(|e| format!("等待 git 命令结束失败 {:?}: {}", args, e))?;

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
