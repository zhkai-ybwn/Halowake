use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
use std::collections::HashMap;

use crate::git::models::{
    GitClonePayload, GitCommandProgressEvent, GitCommandResult, GitCommitPayload,
    GitFileActionPayload, GitFilesActionPayload,
};

#[cfg(test)]
use crate::git::models::{
    GitBranchKind, GitBranchPayload, GitMergePayload, GitRemoteBranchCheckoutPayload,
    GitRepositoryState, GitSyncRecommendedAction,
};

mod branches;
mod diff;
mod history;
mod process;
mod proxy;
mod sync;

use process::{
    emit_manual_git_progress, git_command, run_git_capture, run_git_capture_streaming,
    run_git_pathspec, run_git_with_paths,
};
pub use branches::{
    checkout_remote_branch, configure_origin_remote, create_branch, delete_branch, load_branches,
    repair_upstream, set_branch_upstream, switch_branch,
};
pub(crate) use diff::{load_selected_file_diffs, load_untracked_file_diff};
pub use diff::{
    load_file_diff, load_file_head_diff, load_file_stats, load_git_snapshot, load_git_status, load_selected_file_diff,
};
pub use history::{load_git_commit_detail, load_git_commit_file_diff, load_git_log};
pub use process::{
    kill_process_tree, run_command_with_timeout, run_git, run_git_raw, run_git_with_timeout,
    DEFAULT_GIT_COMMAND_TIMEOUT, DEFAULT_HEAVY_GIT_TIMEOUT, DEFAULT_NETWORK_GIT_TIMEOUT,
    GIT_CLONE_TIMEOUT, ProcessJobGuard,
};
pub use sync::{
    abort_merge, abort_rebase, continue_merge, continue_rebase, fetch_changes,
    fetch_changes_with_progress, merge_branch, pull_changes, pull_changes_with_progress,
    push_changes, push_changes_with_progress, rebase_changes, rebase_changes_with_progress,
    sync_status,
};

#[cfg(test)]
use process::{parse_progress_percent, parse_progress_phase, parse_transfer_text};

#[cfg(test)]
use proxy::{
    is_proxy_bypass_host, no_proxy_rule_matches_host, proxy_env_from_windows_proxy_server,
    registry_value, remote_host_from_url,
};

#[cfg(test)]
use history::parse_commit_changed_file;

#[cfg(test)]
use sync::recommended_sync_action;

fn clear_index_for_selected_commit(repo_path: &str) -> Result<GitCommandResult, String> {
    if run_git(repo_path, &["rev-parse", "--verify", "HEAD"]).is_ok() {
        return run_git_capture(repo_path, &["reset"]);
    }

    run_git_capture(repo_path, &["read-tree", "--empty"])
}

struct IndexBackup {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

impl IndexBackup {
    fn capture(repo_path: &str) -> Result<Self, String> {
        let git_path = run_git(repo_path, &["rev-parse", "--git-path", "index"])?;
        let path = PathBuf::from(git_path.trim());
        let path = if path.is_absolute() { path } else { Path::new(repo_path).join(path) };
        let content = if path.exists() {
            Some(fs::read(&path).map_err(|error| format!("备份 Git 暂存区失败: {}", error))?)
        } else {
            None
        };
        Ok(Self { path, content })
    }

    fn restore(&self) -> Result<(), String> {
        match &self.content {
            Some(content) => fs::write(&self.path, content).map_err(|error| format!("恢复 Git 暂存区失败: {}", error)),
            None if self.path.exists() => fs::remove_file(&self.path).map_err(|error| format!("恢复 Git 暂存区失败: {}", error)),
            None => Ok(()),
        }
    }
}

struct SelectedCommitFile {
    path: String,
    ignored_tracked: bool,
}

fn validate_selected_files(repo_path: &str, selected_files: &[String]) -> Result<Vec<SelectedCommitFile>, String> {
    let paths: Vec<&str> = selected_files.iter().map(String::as_str).collect();
    if paths.iter().any(|path| path.trim().is_empty()) {
        return Err("选中文件路径为空，请刷新仓库状态后重新勾选文件。".to_string());
    }
    let tracked_raw = run_git_raw(repo_path, &["ls-files", "--cached", "-z"])?;
    let tracked: HashSet<_> = tracked_raw.split_terminator('\0').collect();
    let ignored_output = run_git_with_paths(
        git_command(repo_path),
        &["check-ignore", "--no-index", "-z", "--stdin"],
        paths.iter().copied(),
        DEFAULT_GIT_COMMAND_TIMEOUT,
    )?;
    if !matches!(ignored_output.status.code(), Some(0 | 1)) {
        return Err(format!("检查 Git 忽略规则失败: {}", String::from_utf8_lossy(&ignored_output.stderr).trim()));
    }
    let ignored_raw = String::from_utf8_lossy(&ignored_output.stdout);
    let ignored: HashSet<_> = ignored_raw.split_terminator('\0').collect();
    let mut validated_files = Vec::with_capacity(paths.len());
    for normalized in paths {
        let exists_in_worktree = Path::new(repo_path).join(normalized).exists();
        let tracked_by_git = tracked.contains(normalized.trim_end_matches('/'));
        let ignored_by_git = ignored.contains(normalized);
        if ignored_by_git && !tracked_by_git {
            return Err(format!(
                "选中文件已被 .gitignore 忽略，不能作为普通文件加入提交: {}\n\n建议: 请刷新仓库状态后重新勾选文件。",
                normalized
            ));
        }
        if !exists_in_worktree && !tracked_by_git {
            return Err(format!(
                "选中文件在当前仓库中不存在，也不是已跟踪文件: {}\n\n建议: 请刷新仓库状态后重新勾选文件。如果刚切换过项目，请确认当前仓库路径是否正确。",
                normalized
            ));
        }

        validated_files.push(SelectedCommitFile {
            path: normalized.to_string(),
            ignored_tracked: ignored_by_git && tracked_by_git,
        });
    }

    Ok(validated_files)
}

fn stage_selected_files_for_commit(repo_path: &str, selected_files: &[SelectedCommitFile]) -> Result<Vec<GitCommandResult>, String> {
    let (ignored_tracked, regular): (Vec<_>, Vec<_>) = selected_files
        .iter()
        .partition(|file| file.ignored_tracked);

    let mut results = Vec::with_capacity(2);
    if !regular.is_empty() {
        results.push(run_git_pathspec(repo_path, &["add"], regular.iter().map(|file| file.path.as_str()))?);
    }
    if !ignored_tracked.is_empty() {
        results.push(run_git_pathspec(repo_path, &["rm", "--cached"], ignored_tracked.iter().map(|file| file.path.as_str()))?);
    }
    Ok(results)
}

pub fn commit_changes(payload: &GitCommitPayload) -> Result<GitCommandResult, String> {
    commit_changes_with_progress(payload, |_| {})
}

pub fn commit_changes_with_progress<F>(payload: &GitCommitPayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let title = payload.title.trim();
    let body = payload.body.trim();

    if title.is_empty() {
        return Err("Commit title cannot be empty".to_string());
    }

    if payload.selected_files.is_empty() {
        return Err("No files selected for commit".to_string());
    }

    let file_count = payload.selected_files.len();
    emit_manual_git_progress(
        &payload.repo_path,
        "git ls-files / git check-ignore",
        "Validating files",
        None,
        None,
        &mut on_progress,
    );
    let selected_files = validate_selected_files(&payload.repo_path, &payload.selected_files)?;
    emit_manual_git_progress(
        &payload.repo_path,
        "git reset",
        "Preparing index",
        None,
        None,
        &mut on_progress,
    );
    let index_backup = IndexBackup::capture(&payload.repo_path)?;

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut commands = Vec::new();

    let commit_result = (|| {
        let clear_index = clear_index_for_selected_commit(&payload.repo_path)?;
        commands.push(clear_index.command);
        stdout.push_str(&clear_index.stdout);
        stderr.push_str(&clear_index.stderr);

        emit_manual_git_progress(
            &payload.repo_path,
            "git add",
            "Staging files",
            None,
            None,
            &mut on_progress,
        );
        for add in stage_selected_files_for_commit(&payload.repo_path, &selected_files)? {
            commands.push(add.command);
            stdout.push_str(&add.stdout);
            stderr.push_str(&add.stderr);
        }
        emit_manual_git_progress(
            &payload.repo_path,
            "git diff --cached --name-only",
            "Checking staged changes",
            None,
            Some(format!("{file_count} / {file_count}")),
            &mut on_progress,
        );

        let staged_diff_raw = run_git_raw(&payload.repo_path, &["diff", "--cached", "--name-only"])?;
        if staged_diff_raw.trim().is_empty() {
            return Ok(None);
        }

        emit_manual_git_progress(
            &payload.repo_path,
            "git commit",
            "Committing",
            None,
            None,
            &mut on_progress,
        );
        let commit_output = if body.is_empty() {
            run_git_capture_streaming(&payload.repo_path, &["commit", "-m", title], &mut on_progress)?
        } else {
            run_git_capture_streaming(&payload.repo_path, &["commit", "-m", title, "-m", body], &mut on_progress)?
        };
        Ok(Some(commit_output))
    })();

    let commit_opt = match commit_result {
        Ok(commit) => commit,
        Err(error) => {
            index_backup.restore()?;
            return Err(error);
        }
    };

    let Some(commit) = commit_opt else {
        index_backup.restore().map_err(|error| format!("恢复暂存区失败: {}", error))?;
        let _ = stage_selected_files_for_commit(&payload.repo_path, &selected_files);
        emit_manual_git_progress(
            &payload.repo_path,
            "git status",
            "Index synchronized",
            Some(100),
            None,
            &mut on_progress,
        );
        return Ok(GitCommandResult {
            command: commands.join("\n"),
            message: "选中的文件与 HEAD 无代码差异，已自动校准索引并同步文件状态。".to_string(),
            stdout,
            stderr,
            suggestion: Some("所选文件未检测到实际代码修改（通常由换行符规范化或元数据差异引起），暂存区索引已自动校准更新。".to_string()),
        });
    };

    emit_manual_git_progress(
        &payload.repo_path,
        "git reset",
        "Restoring index",
        None,
        None,
        &mut on_progress,
    );
    index_backup.restore().map_err(|error| format!("提交已经完成，但{}", error))?;
    run_git_pathspec(
        &payload.repo_path,
        &["reset", "--no-refresh", "HEAD"],
        selected_files.iter().map(|file| file.path.as_str()),
    )
        .map_err(|error| format!("提交已经完成，但恢复未选文件的暂存状态失败: {}", error))?;
    commands.push(commit.command);
    stdout.push_str(&commit.stdout);
    stderr.push_str(&commit.stderr);
    emit_manual_git_progress(
        &payload.repo_path,
        "git commit",
        "Commit completed",
        Some(100),
        None,
        &mut on_progress,
    );

    Ok(GitCommandResult {
        command: commands.join("\n"),
        message: if commit.message.trim().is_empty() {
            "Commit completed".to_string()
        } else {
            commit.message
        },
        stdout,
        stderr,
        suggestion: None,
    })
}

pub fn open_file_external(payload: &GitFileActionPayload) -> Result<GitCommandResult, String> {
    let file_path = payload.file_path.trim();
    if file_path.is_empty() {
        return Err("文件路径为空，无法打开外部编辑器。".to_string());
    }

    let target_path = Path::new(&payload.repo_path).join(file_path);
    if !target_path.exists() {
        return Err(format!("文件不存在，无法打开外部编辑器: {}", file_path));
    }

    let (program, mut command) = if cfg!(target_os = "windows") {
        ("explorer", Command::new("explorer"))
    } else if cfg!(target_os = "macos") {
        ("open", Command::new("open"))
    } else {
        ("xdg-open", Command::new("xdg-open"))
    };

    command
        .arg(&target_path)
        .spawn()
        .map_err(|e| format!("打开外部编辑器失败 {}: {}", file_path, e))?;

    Ok(GitCommandResult {
        command: format!("{} {}", program, target_path.display()),
        message: format!("已打开外部编辑器: {}", file_path),
        stdout: String::new(),
        stderr: String::new(),
        suggestion: None,
    })
}

pub fn mark_files_resolved(payload: &GitFilesActionPayload) -> Result<GitCommandResult, String> {
    run_file_index_action(
        payload,
        &["add", "-A"],
        "冲突文件已标记为已解决",
        "请选择需要标记为已解决的冲突文件。",
    )
}

pub fn stage_files(payload: &GitFilesActionPayload) -> Result<GitCommandResult, String> {
    run_file_index_action(payload, &["add", "-A"], "已暂存文件", "没有可暂存的文件。")
}

pub fn unstage_files(payload: &GitFilesActionPayload) -> Result<GitCommandResult, String> {
    run_file_index_action(payload, &["restore", "--staged"], "已取消暂存文件", "没有可取消暂存的文件。")
}

fn run_file_index_action(
    payload: &GitFilesActionPayload,
    args: &[&str],
    message: &str,
    empty_message: &str,
) -> Result<GitCommandResult, String> {
    let files: Vec<_> = payload.file_paths.iter().map(String::as_str)
        .filter(|file| !file.trim().is_empty())
        .collect();
    if files.is_empty() {
        return Err(empty_message.to_string());
    }
    let result = run_git_pathspec(&payload.repo_path, args, files)?;
    Ok(GitCommandResult { message: message.to_string(), ..result })
}

pub fn init_repository(repo_path: &str) -> Result<GitCommandResult, String> {
    let result = run_git_capture(repo_path, &["init"])?;
    Ok(GitCommandResult { message: "已初始化 Git 仓库".to_string(), ..result })
}

pub fn clone_repository(payload: &GitClonePayload) -> Result<GitCommandResult, String> {
    let remote_url = payload.remote_url.trim();
    let destination = payload.destination_path.trim();
    if remote_url.is_empty() || destination.is_empty() {
        return Err("仓库地址和目标目录不能为空。".to_string());
    }
    if remote_url.starts_with('-') || destination.starts_with('-') {
        return Err("仓库地址和目标目录不能以连字符开头。".to_string());
    }
    let parent = Path::new(destination)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| "目标目录无效。".to_string())?;
    let parent_path = parent.to_string_lossy();
    let result = run_git_capture(&parent_path, &["clone", "--", remote_url, destination])?;
    Ok(GitCommandResult { message: format!("已克隆仓库到: {}", destination), ..result })
}

pub fn revert_file(payload: &GitFileActionPayload) -> Result<GitCommandResult, String> {
    let file_path = payload.file_path.trim();
    if file_path.is_empty() {
        return Err("文件路径为空，无法回退。".to_string());
    }

    let status = run_git_raw(&payload.repo_path, &["status", "--porcelain", "--", file_path])?;
    if status.trim().is_empty() {
        return Err(format!("文件没有可回退的变更: {}", file_path));
    }

    let is_untracked = status.lines().any(|line| line.starts_with("??"));
    if is_untracked {
        let target_path = safe_repo_file_path(&payload.repo_path, file_path)?;
        if target_path.is_file() {
            fs::remove_file(&target_path).map_err(|error| format!("删除未跟踪文件失败 {}: {}", file_path, error))?;
        } else if target_path.is_dir() {
            fs::remove_dir_all(&target_path).map_err(|error| format!("删除未跟踪目录失败 {}: {}", file_path, error))?;
        }

        return Ok(GitCommandResult {
            command: format!("delete {}", file_path),
            message: format!("已移除未跟踪文件: {}", file_path),
            stdout: String::new(),
            stderr: String::new(),
            suggestion: None,
        });
    }

    let result = run_git_capture(&payload.repo_path, &["restore", "--staged", "--worktree", "--", file_path])?;
    Ok(GitCommandResult {
        message: format!("已回退文件: {}", file_path),
        ..result
    })
}

fn safe_repo_file_path(repo_path: &str, file_path: &str) -> Result<PathBuf, String> {
    let repo_root = fs::canonicalize(repo_path).map_err(|error| format!("读取仓库路径失败: {}", error))?;
    let target = repo_root.join(file_path);
    let normalized = normalize_candidate_path(&target);
    if !normalized.starts_with(&repo_root) {
        return Err(format!("拒绝回退仓库外文件: {}", file_path));
    }
    Ok(target)
}

fn normalize_candidate_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("halowake-{}-{}-{}", name, std::process::id(), now));
        fs::create_dir_all(&path).expect("create temp repo");
        path
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, content).expect("write test file");
    }

    fn git(repo_path: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .stdin(std::process::Stdio::null())
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("run git");

        if !output.status.success() {
            panic!(
                "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
                args,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn normalizes_plain_windows_proxy_server() {
        let proxy = proxy_env_from_windows_proxy_server("127.0.0.1:10808").expect("proxy");

        assert_eq!(proxy.http.as_deref(), Some("http://127.0.0.1:10808"));
        assert_eq!(proxy.https.as_deref(), Some("http://127.0.0.1:10808"));
        assert_eq!(proxy.all.as_deref(), Some("http://127.0.0.1:10808"));
    }

    #[test]
    fn parses_split_windows_proxy_server() {
        let proxy = proxy_env_from_windows_proxy_server(
            "http=127.0.0.1:7890;https=127.0.0.1:7891;socks=127.0.0.1:7892",
        )
        .expect("proxy");

        assert_eq!(proxy.http.as_deref(), Some("http://127.0.0.1:7890"));
        assert_eq!(proxy.https.as_deref(), Some("http://127.0.0.1:7891"));
        assert_eq!(proxy.all.as_deref(), Some("socks5://127.0.0.1:7892"));
    }

    #[test]
    fn parses_registry_values() {
        let registry_output = r#"
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Internet Settings
    ProxyEnable    REG_DWORD    0x1
    ProxyServer    REG_SZ    127.0.0.1:10808
"#;

        assert_eq!(registry_value(registry_output, "ProxyEnable").as_deref(), Some("0x1"));
        assert_eq!(
            registry_value(registry_output, "ProxyServer").as_deref(),
            Some("127.0.0.1:10808")
        );
    }

    #[test]
    fn parses_git_progress_percent_and_phase() {
        let text = "Receiving objects:  42% (42/100), 1.25 MiB | 512.00 KiB/s\r";

        assert_eq!(parse_progress_phase(text).as_deref(), Some("Receiving objects"));
        assert_eq!(parse_progress_percent(text), Some(42));
    }

    #[test]
    fn parses_git_progress_transfer_text() {
        let text = "Writing objects: 100% (12/12), 2.01 MiB | 1.25 MiB/s, done.\n";

        assert_eq!(
            parse_transfer_text(text).as_deref(),
            Some("2.01 MiB | 1.25 MiB/s")
        );
    }

    #[test]
    fn extracts_remote_hosts() {
        assert_eq!(
            remote_host_from_url("http://192.168.0.127:9980/AMI/frontenddeveloper/ami-simulator.git").as_deref(),
            Some("192.168.0.127")
        );
        assert_eq!(
            remote_host_from_url("https://github.com/zhkai-ybwn/Halowake.git").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            remote_host_from_url("git@github.com:zhkai-ybwn/Halowake.git").as_deref(),
            Some("github.com")
        );
    }

    #[test]
    fn bypasses_private_git_hosts() {
        assert!(is_proxy_bypass_host("192.168.0.127"));
        assert!(is_proxy_bypass_host("172.17.182.113"));
        assert!(is_proxy_bypass_host("10.0.8.12"));
        assert!(is_proxy_bypass_host("localhost"));
        assert!(!is_proxy_bypass_host("github.com"));
    }

    #[test]
    fn matches_no_proxy_rules() {
        assert!(no_proxy_rule_matches_host("*.corp.local", "git.corp.local"));
        assert!(no_proxy_rule_matches_host(".example.com", "git.example.com"));
        assert!(no_proxy_rule_matches_host("192.168.*", "192.168.0.127"));
        assert!(no_proxy_rule_matches_host("<local>", "gitlab"));
        assert!(!no_proxy_rule_matches_host("<local>", "github.com"));
    }

    #[test]
    fn recommends_sync_actions_from_ahead_behind() {
        let mut state = GitRepositoryState {
            has_commits: true,
            remote_name: Some("origin".to_string()),
            remote_url: Some("http://example.test/repo.git".to_string()),
            upstream: Some("origin/main".to_string()),
            upstream_gone: false,
            ahead: 1,
            behind: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(recommended_sync_action(&state), GitSyncRecommendedAction::Push);

        state.ahead = 0;
        state.behind = 1;
        assert_eq!(recommended_sync_action(&state), GitSyncRecommendedAction::Pull);

        state.ahead = 1;
        state.behind = 1;
        assert_eq!(
            recommended_sync_action(&state),
            GitSyncRecommendedAction::ResolveDivergence
        );

        state.ahead = 0;
        state.behind = 0;
        assert_eq!(recommended_sync_action(&state), GitSyncRecommendedAction::None);
    }

    #[test]
    fn parses_commit_changed_file_rename() {
        let mut stats = HashMap::new();
        stats.insert("src/new.ts".to_string(), (Some(2), Some(1)));
        let parsed = parse_commit_changed_file("R100\tsrc/old.ts\tsrc/new.ts", &stats).expect("changed file");

        assert_eq!(parsed.status, "R100");
        assert_eq!(parsed.original_path.as_deref(), Some("src/old.ts"));
        assert_eq!(parsed.path, "src/new.ts");
        assert_eq!(parsed.added, Some(2));
        assert_eq!(parsed.removed, Some(1));
    }

    #[test]
    fn status_keeps_chinese_paths_readable() {
        let repo = temp_repo("chinese-path");
        git(&repo, &["init"]);
        let file_name = "大屏接口文档-当前实现-2026-07-08.md";
        write_file(&repo.join(file_name), "# 文档\n");

        let status = run_git_raw(&repo.to_string_lossy(), &["status", "--porcelain=v1", "--untracked-files=all"])
            .expect("read git status");

        assert!(status.contains(file_name));
        assert!(!status.contains("\\345"));
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn commit_removes_tracked_files_that_are_now_ignored() {
        let repo = temp_repo("ignored-tracked-file");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);

        let lumina_file = repo.join(".lumina").join("commit-prompt-debug.json");
        write_file(&lumina_file, "{\"version\":1}\n");
        git(&repo, &["add", "--", ".lumina/commit-prompt-debug.json"]);
        git(&repo, &["commit", "-m", "chore: add lumina debug file"]);

        write_file(&repo.join(".gitignore"), ".lumina\n");
        write_file(&lumina_file, "{\"version\":2}\n");

        let result = commit_changes(&GitCommitPayload {
            repo_path: repo.to_string_lossy().to_string(),
            title: "chore: ignore local lumina data".to_string(),
            body: String::new(),
            selected_files: vec![
                ".gitignore".to_string(),
                ".lumina/commit-prompt-debug.json".to_string(),
            ],
        })
        .expect("commit ignored tracked file removal");

        assert!(result
            .command
            .contains("git --literal-pathspecs rm --cached --pathspec-from-file=- --pathspec-file-nul"));
        assert!(lumina_file.exists());

        let tracked = Command::new("git")
            .args(["ls-files", "--error-unmatch", "--", ".lumina/commit-prompt-debug.json"])
            .current_dir(&repo)
            .stdin(std::process::Stdio::null())
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("run git ls-files");
        assert!(!tracked.status.success());

        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn revert_file_restores_tracked_file() {
        let repo = temp_repo("revert-tracked-file");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("tracked.txt"), "initial\n");
        git(&repo, &["add", "--", "tracked.txt"]);
        git(&repo, &["commit", "-m", "initial"]);

        write_file(&repo.join("tracked.txt"), "changed\n");
        git(&repo, &["add", "--", "tracked.txt"]);
        write_file(&repo.join("tracked.txt"), "changed again\n");

        revert_file(&GitFileActionPayload {
            repo_path: repo.to_string_lossy().to_string(),
            file_path: "tracked.txt".to_string(),
            full_context: None,
        })
        .expect("revert tracked file");

        assert!(run_git(&repo.to_string_lossy(), &["status", "--porcelain"]).unwrap().is_empty());
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn revert_file_removes_untracked_file() {
        let repo = temp_repo("revert-untracked-file");
        git(&repo, &["init"]);
        write_file(&repo.join("new.txt"), "new\n");

        revert_file(&GitFileActionPayload {
            repo_path: repo.to_string_lossy().to_string(),
            file_path: "new.txt".to_string(),
            full_context: None,
        })
        .expect("remove untracked file");

        assert!(!repo.join("new.txt").exists());
        assert!(run_git(&repo.to_string_lossy(), &["status", "--porcelain"]).unwrap().is_empty());
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn selected_commit_preserves_unselected_staged_files() {
        let repo = temp_repo("preserve-index");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("selected.txt"), "initial\n");
        write_file(&repo.join("staged.txt"), "initial\n");
        git(&repo, &["add", "--", "."]);
        git(&repo, &["commit", "-m", "initial"]);

        write_file(&repo.join("selected.txt"), "selected change\n");
        write_file(&repo.join("staged.txt"), "staged change\n");
        git(&repo, &["add", "--", "staged.txt"]);

        commit_changes(&GitCommitPayload {
            repo_path: repo.to_string_lossy().to_string(),
            title: "commit selected file".to_string(),
            body: String::new(),
            selected_files: vec!["selected.txt".to_string()],
        })
        .expect("commit selected file");

        assert_eq!(run_git(&repo.to_string_lossy(), &["diff", "--cached", "--name-only"]).unwrap(), "staged.txt");
        assert!(run_git(&repo.to_string_lossy(), &["status", "--porcelain"]).unwrap().contains("M  staged.txt"));
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn failed_selected_commit_restores_index() {
        let repo = temp_repo("restore-index");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("selected.txt"), "initial\n");
        write_file(&repo.join("staged.txt"), "initial\n");
        git(&repo, &["add", "--", "."]);
        git(&repo, &["commit", "-m", "initial"]);
        write_file(&repo.join("selected.txt"), "selected change\n");
        write_file(&repo.join("staged.txt"), "staged change\n");
        git(&repo, &["add", "--", "staged.txt"]);
        let hook = repo.join(".git").join("hooks").join("pre-commit");
        write_file(&hook, "#!/bin/sh\nexit 1\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).expect("make hook executable");
        }

        let result = commit_changes(&GitCommitPayload {
            repo_path: repo.to_string_lossy().to_string(),
            title: "rejected commit".to_string(),
            body: String::new(),
            selected_files: vec!["selected.txt".to_string()],
        });

        assert!(result.is_err());
        assert_eq!(run_git(&repo.to_string_lossy(), &["diff", "--cached", "--name-only"]).unwrap(), "staged.txt");
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn stages_and_unstages_selected_file() {
        let repo = temp_repo("stage-unstage");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("file.txt"), "initial\n");
        git(&repo, &["add", "--", "file.txt"]);
        git(&repo, &["commit", "-m", "initial"]);
        write_file(&repo.join("file.txt"), "changed\n");

        let payload = GitFilesActionPayload {
            repo_path: repo.to_string_lossy().to_string(),
            file_paths: vec!["file.txt".to_string()],
        };
        stage_files(&payload).expect("stage file");
        assert_eq!(run_git(&payload.repo_path, &["diff", "--cached", "--name-only"]).unwrap(), "file.txt");

        unstage_files(&payload).expect("unstage file");
        assert!(run_git(&payload.repo_path, &["diff", "--cached", "--name-only"]).unwrap().is_empty());
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn manages_branches_and_protects_current_branch() {
        let repo = temp_repo("branch-management");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("file.txt"), "initial\n");
        git(&repo, &["add", "--", "file.txt"]);
        git(&repo, &["commit", "-m", "initial"]);
        let repo_path = repo.to_string_lossy().to_string();

        create_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "feature/test".to_string() }).expect("create branch");
        assert_eq!(run_git(&repo_path, &["branch", "--show-current"]).unwrap(), "feature/test");
        assert!(load_git_snapshot(&repo_path, true).unwrap().branches.iter().any(|branch| branch.name == "feature/test"));
        assert!(delete_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "feature/test".to_string() }).is_err());

        let main_branch = run_git(&repo_path, &["branch", "--format=%(refname:short)"])
            .unwrap()
            .lines()
            .find(|branch| *branch != "feature/test")
            .expect("base branch")
            .to_string();
        switch_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: main_branch }).expect("switch branch");
        delete_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "feature/test".to_string() }).expect("delete branch");
        assert!(load_branches(&repo_path).unwrap().iter().all(|branch| branch.name != "feature/test"));
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn conflict_snapshot_defers_statistics_and_diff_never_runs_external_helpers() {
        let repo = temp_repo("conflict-status-only");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("translations.json"), "{\"value\":\"base\"}\n");
        write_file(&repo.join(".gitattributes"), "*.json diff=diagnostic\n");
        git(&repo, &["add", "--", "."]);
        git(&repo, &["commit", "-m", "base"]);
        let repo_path = repo.to_string_lossy().to_string();
        let base_branch = run_git(&repo_path, &["branch", "--show-current"]).unwrap();
        git(&repo, &["switch", "-c", "incoming"]);
        write_file(&repo.join("translations.json"), "{\"value\":\"incoming\"}\n");
        write_file(&repo.join("added.txt"), "non-conflicting change\n");
        git(&repo, &["add", "--", "."]);
        git(&repo, &["commit", "-m", "incoming"]);
        git(&repo, &["switch", &base_branch]);
        write_file(&repo.join("translations.json"), "{\"value\":\"local\"}\n");
        git(&repo, &["add", "--", "translations.json"]);
        git(&repo, &["commit", "-m", "local"]);
        write_file(&repo.join(".git/record-diff.sh"), "#!/bin/sh\necho invoked > .git/diff-helper-invoked\nexit 1\n");
        git(&repo, &["config", "diff.diagnostic.textconv", "sh .git/record-diff.sh"]);
        git(&repo, &["config", "diff.external", "sh .git/record-diff.sh"]);

        let result = merge_branch(&GitMergePayload {
            repo_path: repo_path.clone(), source_branch: "incoming".to_string(), no_fast_forward: false,
        }).expect("conflict is a recoverable merge result");
        assert!(result.stdout.contains("CONFLICT"));
        let snapshot = load_git_snapshot(&repo_path, true).expect("load conflicted repository");
        assert!(snapshot.repository_state.merge_in_progress);
        assert!(snapshot.status.iter().any(|line| line.starts_with("UU translations.json")));
        assert!(snapshot.staged_files.iter().any(|path| path == "added.txt"));
        assert!(snapshot.file_stats.is_empty(), "conflict refresh must not compute line statistics");
        let diff = load_file_diff(&repo_path, "translations.json", false, false).expect("built-in conflict diff");
        assert!(!diff.diff.is_empty());
        assert!(!repo.join(".git/diff-helper-invoked").exists());

        write_file(&repo.join("translations.json"), "{\"value\":\"resolved\"}\n");
        git(&repo, &["add", "--", "translations.json"]);
        continue_merge(&crate::git::models::GitRepoPayload { repo_path: repo_path.clone() }).expect("finish merge");
        write_file(&repo.join("translations.json"), "{\"value\":\"changed again\"}\n");
        let resolved = load_git_snapshot(&repo_path, true).expect("statistics resume after resolution");
        assert!(resolved.file_stats.iter().any(|stat| stat.path == "translations.json"));
        assert!(!repo.join(".git/diff-helper-invoked").exists());
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn merges_selected_branch_and_rejects_dirty_worktree() {
        let repo = temp_repo("merge-branch");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("file.txt"), "initial\n");
        git(&repo, &["add", "--", "file.txt"]);
        git(&repo, &["commit", "-m", "initial"]);
        let repo_path = repo.to_string_lossy().to_string();
        let base_branch = run_git(&repo_path, &["branch", "--show-current"]).expect("base branch");

        git(&repo, &["switch", "-c", "feature/merge"]);
        write_file(&repo.join("feature.txt"), "feature change\n");
        git(&repo, &["add", "--", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature"]);
        git(&repo, &["switch", &base_branch]);

        merge_branch(&GitMergePayload {
            repo_path: repo_path.clone(),
            source_branch: "feature/merge".to_string(),
            no_fast_forward: false,
        })
        .expect("merge feature branch");
        assert!(run_git(&repo_path, &["merge-base", "--is-ancestor", "feature/merge", "HEAD"]).is_ok());

        git(&repo, &["switch", "-c", "feature/no-ff"]);
        write_file(&repo.join("no-ff.txt"), "no fast forward\n");
        git(&repo, &["add", "--", "no-ff.txt"]);
        git(&repo, &["commit", "-m", "no fast forward"]);
        git(&repo, &["switch", &base_branch]);
        merge_branch(&GitMergePayload {
            repo_path: repo_path.clone(),
            source_branch: "feature/no-ff".to_string(),
            no_fast_forward: true,
        })
        .expect("merge feature branch without fast-forward");
        assert_eq!(
            run_git(&repo_path, &["rev-list", "--parents", "-n", "1", "HEAD"])
                .expect("merge parents")
                .split_whitespace()
                .count(),
            3,
        );

        write_file(&repo.join("file.txt"), "uncommitted\n");
        let error = merge_branch(&GitMergePayload {
            repo_path,
            source_branch: "feature/merge".to_string(),
            no_fast_forward: true,
        })
        .expect_err("reject dirty worktree");
        assert!(error.contains("合并前请先提交"));
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn lists_remote_branches_and_checks_them_out_with_tracking() {
        let repo = temp_repo("remote-branch-checkout");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "lumina@example.test"]);
        write_file(&repo.join("file.txt"), "initial\n");
        git(&repo, &["add", "--", "file.txt"]);
        git(&repo, &["commit", "-m", "initial"]);
        let repo_path = repo.to_string_lossy().to_string();
        git(&repo, &["remote", "add", "origin", "https://example.test/repo.git"]);
        git(&repo, &["update-ref", "refs/remotes/origin/release", "HEAD"]);

        let branches = load_branches(&repo_path).expect("load branches");
        assert!(branches.iter().any(|branch| branch.name == "origin/release" && matches!(branch.kind, GitBranchKind::Remote)));

        checkout_remote_branch(&GitRemoteBranchCheckoutPayload {
            repo_path: repo_path.clone(),
            remote_branch: "origin/release".to_string(),
            local_branch: "release".to_string(),
        })
        .expect("checkout tracked remote branch");

        assert_eq!(run_git(&repo_path, &["branch", "--show-current"]).unwrap(), "release");
        assert_eq!(run_git(&repo_path, &["config", "branch.release.remote"]).unwrap(), "origin");
        assert_eq!(run_git(&repo_path, &["config", "branch.release.merge"]).unwrap(), "refs/heads/release");
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn checkout_and_commit_handle_path_lists_larger_than_windows_argv() {
        let repo = tempfile::tempdir().expect("create repository fixture");
        let root = repo.path();
        let repo_path = root.to_string_lossy().into_owned();
        git(root, &["init", "-b", "main"]);
        git(root, &["config", "user.name", "Halowake Test"]);
        git(root, &["config", "user.email", "halowake@example.test"]);
        git(root, &["config", "core.autocrlf", "false"]);
        write_file(&root.join(".gitattributes"), "*.ts text eol=crlf\n");
        write_file(&root.join("other.txt"), "original\n");
        let paths: Vec<_> = (0..410).map(|i| {
            format!("fixtures/long-nested-directory-for-windows-command-line/long-source-name-for-pathspec-{i:04}.ts")
        }).collect();
        assert!(paths.iter().map(|path| path.len() + 1).sum::<usize>() > 32_767);
        for path in &paths {
            write_file(&root.join(path), "main\r\n");
        }
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "base"]);
        git(root, &["switch", "-c", "target"]);
        for path in &paths {
            write_file(&root.join(path), "target content\r\n");
        }
        git(root, &["commit", "-am", "target"]);
        git(root, &["switch", "main"]);
        for path in &paths {
            write_file(&root.join(path), "main\n");
        }
        assert_eq!(run_git_raw(&repo_path, &["status", "--porcelain"]).unwrap().lines().count(), paths.len());
        switch_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "target".to_string() }).unwrap();
        assert_eq!(fs::read(root.join(&paths[0])).unwrap(), b"target content\r\n");

        for path in &paths {
            write_file(&root.join(path), "new content\r\n");
        }
        write_file(&root.join("other.txt"), "staged separately\n");
        git(root, &["add", "other.txt"]);
        let mut phases = Vec::new();
        commit_changes_with_progress(&GitCommitPayload {
            repo_path: repo_path.clone(),
            title: "bulk selected changes".to_string(),
            body: String::new(),
            selected_files: paths.clone(),
        }, |event| {
            if let Some(phase) = event.phase { phases.push(phase); }
        }).unwrap();
        assert_eq!(phases.first().map(String::as_str), Some("Validating files"));
        assert!(phases.iter().any(|phase| phase == "Restoring index"));
        assert_eq!(run_git(&repo_path, &["show", "--format=", "--name-only", "HEAD"]).unwrap().lines().count(), paths.len());
        assert_eq!(run_git(&repo_path, &["diff", "--cached", "--name-only"]).unwrap(), "other.txt");
        assert!(!root.join(".git/index.lock").exists());
    }

    #[test]
    fn selected_patch_batch_keeps_staged_and_unstaged_patches_with_their_paths() {
        let repo = tempfile::tempdir().expect("create repository fixture");
        let root = repo.path();
        let repo_path = root.to_string_lossy();
        git(root, &["init", "-b", "main"]);
        git(root, &["config", "user.name", "Halowake Test"]);
        git(root, &["config", "user.email", "halowake@example.test"]);
        for path in ["folder space/a b.txt", "中文[1].txt", "unselected.txt"] {
            write_file(&root.join(path), "initial\n");
        }
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "base"]);
        write_file(&root.join("folder space/a b.txt"), "staged evidence\n");
        git(root, &["add", "--", "folder space/a b.txt"]);
        write_file(&root.join("folder space/a b.txt"), "worktree evidence\n");
        write_file(&root.join("中文[1].txt"), "second file evidence\n");
        write_file(&root.join("unselected.txt"), "unselected evidence\n");

        let paths = ["中文[1].txt", "folder space/a b.txt"];
        let batched = load_selected_file_diffs(&repo_path, &paths).unwrap();
        for path in paths {
            assert_eq!(batched[path], load_selected_file_diff(&repo_path, path).unwrap());
        }
        assert!(!batched.values().any(|diff| diff.contains("unselected evidence")));
    }

    #[test]
    fn branch_actions_refresh_eol_phantoms_without_staging_real_edits() {
        for action in ["local", "remote", "create"] {
            let repo = temp_repo(&format!("branch-eol-{}", action));
            git(&repo, &["init", "-b", "main"]);
            git(&repo, &["config", "user.name", "Halowake Test"]);
            git(&repo, &["config", "user.email", "halowake@example.test"]);
            git(&repo, &["config", "core.autocrlf", "false"]);
            write_file(&repo.join(".gitattributes"), "*.ts text eol=crlf\n");
            let phantom_names = ["file.ts", "space name.ts", "中文.ts", "literal[1].ts"];
            for name in phantom_names {
                write_file(&repo.join(name), "base\r\n");
            }
            write_file(&repo.join("staged.txt"), "base\n");
            write_file(&repo.join("unstaged.txt"), "base\n");
            git(&repo, &["add", "."]);
            git(&repo, &["commit", "-m", "base"]);
            git(&repo, &["switch", "-c", "target"]);
            for name in phantom_names {
                write_file(&repo.join(name), "target\r\n");
            }
            git(&repo, &["add", "."]);
            git(&repo, &["commit", "-m", "target"]);
            git(&repo, &["remote", "add", "origin", "https://example.test/repo.git"]);
            git(&repo, &["update-ref", "refs/remotes/origin/release", "HEAD"]);
            git(&repo, &["switch", "main"]);
            for name in phantom_names {
                write_file(&repo.join(name), "base\n");
            }
            write_file(&repo.join("staged.txt"), "staged edit\n");
            git(&repo, &["add", "staged.txt"]);
            write_file(&repo.join("unstaged.txt"), "unstaged edit\n");
            write_file(&repo.join("untracked.txt"), "untracked edit\n");
            let repo_path = repo.to_string_lossy().to_string();
            let staged_before = run_git_raw(&repo_path, &["diff", "--cached", "--binary"]).unwrap();
            let index_before = fs::read(repo.join(".git/index")).unwrap();
            assert!(load_git_status(&repo_path).unwrap().lines().all(|line| !line.ends_with(".ts")));
            assert_eq!(fs::read(repo.join(".git/index")).unwrap(), index_before, "status must stay read-only");

            let expected_branch = match action {
                "local" => {
                    switch_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "target".to_string() }).unwrap();
                    "target"
                }
                "remote" => {
                    checkout_remote_branch(&GitRemoteBranchCheckoutPayload {
                        repo_path: repo_path.clone(), remote_branch: "origin/release".to_string(), local_branch: "release".to_string(),
                    }).unwrap();
                    assert_eq!(run_git(&repo_path, &["rev-parse", "--abbrev-ref", "@{u}"]).unwrap(), "origin/release");
                    "release"
                }
                _ => {
                    create_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "new".to_string() }).unwrap();
                    "new"
                }
            };
            assert_eq!(run_git(&repo_path, &["branch", "--show-current"]).unwrap(), expected_branch);
            assert_eq!(run_git_raw(&repo_path, &["diff", "--cached", "--binary"]).unwrap(), staged_before);
            assert_eq!(fs::read(repo.join("unstaged.txt")).unwrap(), b"unstaged edit\n");
            assert_eq!(fs::read(repo.join("untracked.txt")).unwrap(), b"untracked edit\n");
            let raw_status = run_git_raw(&repo_path, &["status", "--porcelain"]).unwrap();
            assert!(raw_status.lines().all(|line| !line.ends_with(".ts")));
            for name in phantom_names {
                let expected: &[u8] = if action == "create" { b"base\n" } else { b"target\r\n" };
                assert_eq!(fs::read(repo.join(name)).unwrap(), expected);
            }
            assert!(!repo.join(".git/index.lock").exists());
            fs::remove_dir_all(&repo).expect("remove temp repo");
        }
    }

    #[test]
    fn branch_checkout_preserves_real_conflicting_edits() {
        for staged in [false, true] {
            let repo = temp_repo("branch-real-edit");
            git(&repo, &["init", "-b", "main"]);
            git(&repo, &["config", "user.name", "Halowake Test"]);
            git(&repo, &["config", "user.email", "halowake@example.test"]);
            git(&repo, &["config", "core.autocrlf", "false"]);
            write_file(&repo.join(".gitattributes"), "*.ts text eol=crlf\n");
            write_file(&repo.join("phantom.ts"), "base\r\n");
            write_file(&repo.join("real.txt"), "base\n");
            git(&repo, &["add", "."]);
            git(&repo, &["commit", "-m", "base"]);
            git(&repo, &["switch", "-c", "target"]);
            write_file(&repo.join("real.txt"), "target\n");
            git(&repo, &["add", "."]);
            git(&repo, &["commit", "-m", "target"]);
            git(&repo, &["switch", "main"]);
            write_file(&repo.join("phantom.ts"), "base\n");
            write_file(&repo.join("real.txt"), "local edit\n");
            if staged { git(&repo, &["add", "real.txt"]); }
            let repo_path = repo.to_string_lossy().to_string();
            let staged_before = run_git_raw(&repo_path, &["diff", "--cached", "--binary"]).unwrap();
            let error = switch_branch(&GitBranchPayload { repo_path: repo_path.clone(), branch: "target".to_string() }).unwrap_err();
            assert!(error.contains("would be overwritten"));
            assert!(error.contains("git stash"));
            assert!(!error.contains("再拉取"));
            assert_eq!(run_git(&repo_path, &["branch", "--show-current"]).unwrap(), "main");
            assert_eq!(run_git_raw(&repo_path, &["diff", "--cached", "--binary"]).unwrap(), staged_before);
            assert_eq!(fs::read(repo.join("real.txt")).unwrap(), b"local edit\n");
            assert_eq!(fs::read(repo.join("phantom.ts")).unwrap(), b"base\n");
            assert!(!repo.join(".git/index.lock").exists());
            fs::remove_dir_all(&repo).expect("remove temp repo");
        }
    }

    #[test]
    fn snapshot_omits_eol_phantoms_and_tracked_empty_diff_stays_empty() {
        let repo = temp_repo("phantom-status");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "halowake@example.test"]);
        git(&repo, &["config", "core.autocrlf", "false"]);
        write_file(&repo.join(".gitattributes"), "*.ts text eol=crlf\n");
        write_file(&repo.join("file.ts"), "export const a = 1;\r\n");
        write_file(&repo.join("space name.ts"), "export const b = 1;\r\n");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "initial"]);
        let repo_path = repo.to_string_lossy();
        let index_path = repo.join(".git/index");
        write_file(&repo.join("file.ts"), "export const a = 1;\n");
        write_file(&repo.join("space name.ts"), "export const b = 1;\n");
        assert!(run_git_raw(&repo_path, &["status", "--porcelain"]).unwrap().contains("file.ts"));
        let index_before = fs::read(&index_path).unwrap();

        assert!(load_git_snapshot(&repo_path, true).unwrap().status.is_empty());
        assert!(load_selected_file_diff(&repo_path, "file.ts").unwrap().is_empty());
        assert_eq!(fs::read(&index_path).unwrap(), index_before);
        assert_eq!(fs::read(repo.join("file.ts")).unwrap(), b"export const a = 1;\n");

        write_file(&repo.join("new.ts"), "export const added = 1;\n");
        let untracked_diff = load_selected_file_diff(&repo_path, "new.ts").unwrap();
        assert!(untracked_diff.contains("new file mode"));
        assert!(untracked_diff.contains("+export const added = 1;"));
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn status_preserves_real_binary_staged_deleted_and_untracked_changes() {
        let repo = temp_repo("real-status");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "halowake@example.test"]);
        for name in ["file.ts", "space name.ts", "中文.ts", "staged.ts", "deleted.ts", "mode.ts"] {
            write_file(&repo.join(name), "initial\n");
        }
        fs::write(repo.join("binary.dat"), b"\0initial").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "initial"]);
        for name in ["file.ts", "space name.ts", "中文.ts", "staged.ts"] {
            write_file(&repo.join(name), "changed\n");
        }
        fs::remove_file(repo.join("deleted.ts")).unwrap();
        fs::write(repo.join("binary.dat"), b"\0changed").unwrap();
        write_file(&repo.join("new.ts"), "new\n");
        // Simulate a worktree executable-bit difference, including on Windows.
        git(&repo, &["update-index", "--chmod=+x", "mode.ts"]);
        git(&repo, &["commit", "-m", "executable"]);
        git(&repo, &["config", "core.filemode", "true"]);
        git(&repo, &["add", "staged.ts"]);
        let repo_path = repo.to_string_lossy();
        let raw = run_git_raw(&repo_path, &["status", "--porcelain=v1", "--untracked-files=all"]).unwrap();
        let filtered = load_git_status(&repo_path).unwrap();
        assert_eq!(filtered, raw);
        assert!(filtered.contains("binary.dat"));
        assert!(filtered.contains("space name.ts"));
        assert!(filtered.contains("?? new.ts"));
        assert!(filtered.contains("M  staged.ts"));
        assert!(filtered.contains(" D deleted.ts"));
        assert!(filtered.contains(" M mode.ts"));
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }

    #[test]
    fn commit_phantom_diff_calibrates_index_and_succeeds() {
        let repo = temp_repo("phantom-diff-commit");
        git(&repo, &["init"]);
        git(&repo, &["config", "user.name", "Halowake Test"]);
        git(&repo, &["config", "user.email", "halowake@example.test"]);
        git(&repo, &["config", "core.autocrlf", "false"]);
        write_file(&repo.join(".gitattributes"), "*.ts text eol=crlf\n");
        let ts_file = repo.join("file.ts");
        write_file(&ts_file, "export const a = 1;\r\n");
        git(&repo, &["add", "--", "."]);
        git(&repo, &["commit", "-m", "initial"]);

        // Simulate editor saving with LF line endings
        fs::write(&ts_file, b"export const a = 1;\n").unwrap();
        assert!(run_git(&repo.to_string_lossy(), &["status", "--porcelain"]).unwrap().contains("file.ts"));

        let result = commit_changes(&GitCommitPayload {
            repo_path: repo.to_string_lossy().to_string(),
            title: "try phantom commit".to_string(),
            body: String::new(),
            selected_files: vec!["file.ts".to_string()],
        });

        assert!(result.is_ok());
        let command_result = result.unwrap();
        assert!(command_result.message.contains("无代码差异"));
        assert!(run_git(&repo.to_string_lossy(), &["status", "--porcelain"]).unwrap().trim().is_empty());
        fs::remove_dir_all(&repo).expect("remove temp repo");
    }
}
