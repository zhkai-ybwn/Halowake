use std::thread;

use crate::git::models::{
    GitFileActionPayload, GitFileDiffResponse, GitFileStat, GitSnapshot,
};

use super::branches::load_branches;
use super::process::{run_git, run_git_raw, silent_command};
use super::sync::load_repository_state;

pub fn load_git_snapshot(repo_path: &str) -> Result<GitSnapshot, String> {
    let (repo_root, branch, repository_state, status_raw, staged_files_raw, file_stats, branches) =
        thread::scope(|scope| {
            let repo_root =
                scope.spawn(|| run_git(repo_path, &["rev-parse", "--show-toplevel"]));
            let branch = scope.spawn(|| run_git(repo_path, &["branch", "--show-current"]));
            let repository_state = scope.spawn(|| load_repository_state(repo_path));
            let status_raw = scope.spawn(|| {
                run_git_raw(
                    repo_path,
                    &["status", "--porcelain=v1", "--untracked-files=all"],
                )
            });
            let staged_files_raw =
                scope.spawn(|| run_git(repo_path, &["diff", "--cached", "--name-only"]));
            let file_stats = scope.spawn(|| load_file_stats(repo_path));
            let branches = scope.spawn(|| load_branches(repo_path));

            Ok::<_, String>((
                repo_root
                    .join()
                    .map_err(|_| "读取仓库根目录任务异常".to_string())??,
                branch
                    .join()
                    .map_err(|_| "读取当前分支任务异常".to_string())??,
                repository_state
                    .join()
                    .map_err(|_| "读取仓库状态任务异常".to_string())?,
                status_raw
                    .join()
                    .map_err(|_| "读取文件状态任务异常".to_string())??,
                staged_files_raw
                    .join()
                    .map_err(|_| "读取暂存文件任务异常".to_string())??,
                file_stats
                    .join()
                    .map_err(|_| "读取文件统计任务异常".to_string())??,
                branches
                    .join()
                    .map_err(|_| "读取分支任务异常".to_string())??,
            ))
        })?;

    let status = if status_raw.trim().is_empty() {
        vec![]
    } else {
        status_raw.lines().map(|s| s.to_string()).collect()
    };

    let staged_files = if staged_files_raw.is_empty() {
        vec![]
    } else {
        staged_files_raw.lines().map(|s| s.to_string()).collect()
    };

    Ok(GitSnapshot {
        repo_path: repo_path.to_string(),
        repo_root,
        branch,
        repository_state,
        status,
        staged_files,
        // Selected-file prompts load only the necessary diffs on demand.
        staged_diff: String::new(),
        file_stats,
        branches,
    })
}

pub fn load_file_diff(
    repo_path: &str,
    file_path: &str,
    staged: bool,
    full_context: bool,
) -> Result<GitFileDiffResponse, String> {
    let unified = if full_context {
        "--unified=2147483647"
    } else {
        "--unified=3"
    };
    let args = if staged {
        vec!["diff", "--cached", unified, "--no-color", "--", file_path]
    } else {
        vec!["diff", unified, "--no-color", "--", file_path]
    };

    let diff = run_git_raw(repo_path, &args)?;

    Ok(GitFileDiffResponse {
        file_path: file_path.to_string(),
        staged,
        diff,
    })
}

pub fn load_file_head_diff(
    payload: &GitFileActionPayload,
) -> Result<GitFileDiffResponse, String> {
    let diff = load_selected_file_diff_with_context(
        &payload.repo_path,
        &payload.file_path,
        payload.full_context.unwrap_or(false),
    )?;

    Ok(GitFileDiffResponse {
        file_path: payload.file_path.clone(),
        staged: false,
        diff,
    })
}

pub fn load_selected_file_diff(repo_path: &str, file_path: &str) -> Result<String, String> {
    load_selected_file_diff_with_context(repo_path, file_path, false)
}

fn load_file_stats(repo_path: &str) -> Result<Vec<GitFileStat>, String> {
    if run_git(repo_path, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Ok(vec![]);
    }

    let unstaged_raw = run_git_raw(repo_path, &["diff", "--numstat", "--"])?;
    let staged_raw = run_git_raw(repo_path, &["diff", "--cached", "--numstat", "--"])?;

    let mut stats = unstaged_raw
        .lines()
        .chain(staged_raw.lines())
        .filter_map(parse_numstat_line)
        .collect::<Vec<_>>();

    stats.sort_by(|left, right| left.path.cmp(&right.path));
    stats.dedup_by(|left, right| {
        if left.path != right.path {
            return false;
        }

        right.added = merge_line_count(left.added, right.added);
        right.removed = merge_line_count(left.removed, right.removed);
        true
    });

    Ok(stats)
}

fn merge_line_count(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn parse_numstat_line(line: &str) -> Option<GitFileStat> {
    let mut parts = line.split('\t');
    let added_raw = parts.next()?;
    let removed_raw = parts.next()?;
    let path = parts.next()?.to_string();

    Some(GitFileStat {
        path,
        added: added_raw.parse::<usize>().ok(),
        removed: removed_raw.parse::<usize>().ok(),
    })
}

fn load_selected_file_diff_with_context(
    repo_path: &str,
    file_path: &str,
    full_context: bool,
) -> Result<String, String> {
    let unified = if full_context {
        "--unified=2147483647"
    } else {
        "--unified=3"
    };
    let unstaged = run_git_raw(
        repo_path,
        &["diff", unified, "--no-color", "--", file_path],
    )?;
    let staged = run_git_raw(
        repo_path,
        &["diff", "--cached", unified, "--no-color", "--", file_path],
    )?;
    let mut result = String::new();

    if !staged.trim().is_empty() {
        result.push_str(&staged);
    }

    if !unstaged.trim().is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&unstaged);
    }

    if result.trim().is_empty() {
        return load_untracked_file_diff(repo_path, file_path, unified);
    }

    Ok(result)
}

fn load_untracked_file_diff(
    repo_path: &str,
    file_path: &str,
    unified: &str,
) -> Result<String, String> {
    let output = silent_command("git")
        .args([
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-index",
            unified,
            "--no-color",
            "--",
            "/dev/null",
            file_path,
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("执行 git diff --no-index 失败 {}: {}", file_path, e))?;

    if output.status.success() || output.status.code() == Some(1) {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("git diff --no-index 执行失败: {}", file_path)
    } else {
        stderr
    })
}
