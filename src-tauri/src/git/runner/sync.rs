use std::path::Path;

use crate::git::models::{
    GitCommandProgressEvent, GitCommandResult, GitMergePayload, GitPullPayload, GitPushPayload,
    GitRebasePayload, GitRepoPayload, GitRepositoryState, GitSyncRecommendedAction, GitSyncStatus,
};

use super::process::{
    emit_manual_git_progress, format_command_error, git_command, git_error_suggestion, run_git,
    run_git_capture, run_git_capture_status, run_git_capture_streaming, run_git_raw,
};

fn has_unmerged_files(repo_path: &str) -> bool {
    let Ok(status_raw) = run_git_raw(repo_path, &["status", "--porcelain=v1", "--untracked-files=all"]) else {
        return false;
    };

    status_raw.lines().any(|line| {
        let status = line.get(0..2).unwrap_or_default();
        matches!(status, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU")
    })
}

fn working_tree_is_clean(repo_path: &str) -> Result<bool, String> {
    let worktree_clean = git_command(repo_path)
        .args(["diff", "--quiet"])
        .status()
        .map_err(|e| format!("检查工作区状态失败: {}", e))?
        .success();
    let index_clean = git_command(repo_path)
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map_err(|e| format!("检查暂存区状态失败: {}", e))?
        .success();
    let untracked = run_git_raw(repo_path, &["ls-files", "--others", "--exclude-standard"])?;

    Ok(worktree_clean && index_clean && untracked.trim().is_empty())
}

fn git_path_exists(repo_path: &str, git_path: &str) -> bool {
    let Ok(relative_path) = run_git(repo_path, &["rev-parse", "--git-path", git_path]) else {
        return false;
    };

    Path::new(repo_path).join(relative_path.trim()).exists()
}

pub(super) fn load_repository_state(repo_path: &str) -> GitRepositoryState {
    let has_commits = run_git(repo_path, &["rev-parse", "--verify", "HEAD"]).is_ok();
    let remote_url = run_git(repo_path, &["remote", "get-url", "origin"]).ok();
    let upstream = if has_commits {
        run_git(repo_path, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]).ok()
    } else {
        None
    };
    let upstream_gone = upstream
        .as_ref()
        .is_some_and(|value| run_git(repo_path, &["rev-parse", "--verify", value]).is_err());

    let (ahead, behind) = if has_commits && upstream.is_some() && !upstream_gone {
        run_git(repo_path, &["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .ok()
            .and_then(|value| {
                let mut parts = value.split_whitespace();
                let ahead = parts.next()?.parse::<usize>().ok()?;
                let behind = parts.next()?.parse::<usize>().ok()?;
                Some((ahead, behind))
            })
            .unwrap_or((0, 0))
    } else {
        (0, 0)
    };

    GitRepositoryState {
        has_commits,
        remote_name: remote_url.as_ref().map(|_| "origin".to_string()),
        remote_url,
        upstream,
        upstream_gone,
        ahead,
        behind,
        merge_in_progress: git_path_exists(repo_path, "MERGE_HEAD"),
        rebase_in_progress: git_path_exists(repo_path, "rebase-merge")
            || git_path_exists(repo_path, "rebase-apply"),
    }
}

pub(super) fn recommended_sync_action(state: &GitRepositoryState) -> GitSyncRecommendedAction {
    if !state.has_commits {
        return GitSyncRecommendedAction::None;
    }
    if state.remote_url.is_none() {
        return GitSyncRecommendedAction::ConfigureRemote;
    }
    if state.upstream.is_none() || state.upstream_gone {
        return GitSyncRecommendedAction::PublishBranch;
    }
    if state.ahead > 0 && state.behind > 0 {
        return GitSyncRecommendedAction::ResolveDivergence;
    }
    if state.behind > 0 {
        return GitSyncRecommendedAction::Pull;
    }
    if state.ahead > 0 {
        return GitSyncRecommendedAction::Push;
    }

    GitSyncRecommendedAction::None
}

fn sync_status_message(state: &GitRepositoryState, action: &GitSyncRecommendedAction) -> String {
    let upstream = state.upstream.as_deref().unwrap_or("未设置");
    let base = format!(
        "远端检查完成\nupstream: {}\nahead: {}\nbehind: {}",
        upstream, state.ahead, state.behind
    );

    let advice = match action {
        GitSyncRecommendedAction::Push => "",
        GitSyncRecommendedAction::Pull => "远端有本地没有的提交，请先 Pull，再 Push。",
        GitSyncRecommendedAction::ResolveDivergence => {
            "本地和远端都有新提交，不能直接 Push。请先 Pull 或打开 Log 确认分叉后再处理。"
        }
        GitSyncRecommendedAction::ConfigureRemote => "当前仓库没有 origin remote，请先配置远端。",
        GitSyncRecommendedAction::PublishBranch => "当前分支没有可用 upstream，可以发布当前分支并设置 upstream。",
        GitSyncRecommendedAction::None => "本地与远端已同步。",
    };

    if advice.is_empty() {
        base
    } else {
        format!("{}\n{}", base, advice)
    }
}

pub fn sync_status(payload: &GitRepoPayload) -> Result<GitSyncStatus, String> {
    let state = load_repository_state(&payload.repo_path);
    let recommended_action = recommended_sync_action(&state);
    let message = sync_status_message(&state, &recommended_action);
    let suggestion = match recommended_action {
        GitSyncRecommendedAction::Pull => Some("远端有本地没有的提交。请先 Pull，同步后再 Push。".to_string()),
        GitSyncRecommendedAction::ResolveDivergence => Some("当前分支已分叉。请先 Pull 并处理可能的冲突，或打开 Log 确认差异。".to_string()),
        GitSyncRecommendedAction::ConfigureRemote => Some("当前仓库没有 origin remote，推送前需要先连接远端仓库。".to_string()),
        GitSyncRecommendedAction::PublishBranch => Some("当前分支没有 upstream。Push 会使用 -u 发布当前分支。".to_string()),
        _ => None,
    };

    Ok(GitSyncStatus {
        command: "git status (local)".to_string(),
        message,
        stdout: String::new(),
        stderr: String::new(),
        suggestion,
        state,
        recommended_action,
    })
}

pub fn push_changes(payload: &GitPushPayload) -> Result<GitCommandResult, String> {
    push_changes_with_progress(payload, |_| {})
}

pub fn push_changes_with_progress<F>(payload: &GitPushPayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let branch = run_git(&payload.repo_path, &["branch", "--show-current"])?;
    if branch.trim().is_empty() {
        return Err("Cannot push because current branch is detached".to_string());
    }
    if run_git(&payload.repo_path, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Err("当前仓库还没有首个提交，无法推送。\n\n建议: 请先提交勾选文件，再执行 Push。".to_string());
    }
    if run_git(&payload.repo_path, &["remote", "get-url", "origin"]).is_err() {
        return Err("当前仓库没有配置 origin remote，无法推送。\n\n建议: 请先添加 GitHub 仓库地址，例如 git remote add origin <url>。".to_string());
    }

    let fetch = run_git_capture_streaming(&payload.repo_path, &["fetch", "--progress", "--prune"], &mut on_progress)?;
    emit_manual_git_progress(
        &payload.repo_path,
        "git push --progress",
        "Checking remote status",
        None,
        None,
        &mut on_progress,
    );
    let state = load_repository_state(&payload.repo_path);
    let action = recommended_sync_action(&state);
    if matches!(action, GitSyncRecommendedAction::Pull | GitSyncRecommendedAction::ResolveDivergence) {
        return Err(sync_status_message(&state, &action));
    }

    let upstream = run_git(&payload.repo_path, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
    let mut result = match upstream {
        Ok(value) if run_git(&payload.repo_path, &["rev-parse", "--verify", &value]).is_ok() => {
            run_git_capture_streaming(&payload.repo_path, &["push", "--progress"], &mut on_progress)?
        }
        _ => run_git_capture_streaming(
            &payload.repo_path,
            &["push", "--progress", "-u", "origin", branch.trim()],
            &mut on_progress,
        )?,
    };
    result.command = format!("{}\n{}", fetch.command, result.command);
    result.stdout = format!("{}{}", fetch.stdout, result.stdout);
    result.stderr = format!("{}{}", fetch.stderr, result.stderr);
    let message = if result.stdout.trim().is_empty() {
        "Push completed".to_string()
    } else {
        result.stdout.trim().to_string()
    };

    Ok(GitCommandResult {
        message,
        ..result
    })
}

pub fn pull_changes(payload: &GitPullPayload) -> Result<GitCommandResult, String> {
    pull_changes_with_progress(payload, |_| {})
}

pub fn merge_branch(payload: &GitMergePayload) -> Result<GitCommandResult, String> {
    let source_branch = payload.source_branch.trim();
    if source_branch.is_empty() {
        return Err("请选择要合并的分支。".to_string());
    }

    let state = load_repository_state(&payload.repo_path);
    if state.merge_in_progress || state.rebase_in_progress || has_unmerged_files(&payload.repo_path) {
        return Err("当前仓库已有未完成的 merge 或 rebase，请先解决冲突、继续或中止当前操作。".to_string());
    }
    if !working_tree_is_clean(&payload.repo_path)? {
        return Err("合并前请先提交、暂存或还原当前工作区变更，避免覆盖未提交内容。".to_string());
    }

    let current_branch = run_git(&payload.repo_path, &["branch", "--show-current"])?;
    if current_branch.trim() == source_branch {
        return Err("不能将当前分支合并到自身。".to_string());
    }
    run_git(&payload.repo_path, &["rev-parse", "--verify", &format!("{}^{{commit}}", source_branch)])
        .map_err(|_| format!("未找到可合并的分支或提交：{}", source_branch))?;

    let mut args = vec!["merge", "--no-edit"];
    if payload.no_fast_forward {
        args.push("--no-ff");
    }
    args.push(source_branch);
    let (success, result) = run_git_capture_status(&payload.repo_path, &args)?;

    if !success {
        if has_unmerged_files(&payload.repo_path) {
            return Ok(GitCommandResult {
                message: "合并已开始，但产生冲突。请解决冲突后标记 resolved，再完成 merge。".to_string(),
                suggestion: Some("请在冲突列表中打开文件解决冲突，然后点击 Continue merge；如需放弃本次合并，使用 Abort merge。".to_string()),
                ..result
            });
        }

        let suggestion = git_error_suggestion(&result.stderr);
        return Err(format_command_error(
            &result.command,
            &result.stdout,
            &result.stderr,
            suggestion.as_deref(),
        ));
    }

    Ok(GitCommandResult {
        message: if result.stdout.trim().is_empty() {
            "Merge completed".to_string()
        } else {
            result.stdout.trim().to_string()
        },
        ..result
    })
}

pub fn pull_changes_with_progress<F>(payload: &GitPullPayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut commands = Vec::new();
    let upstream = run_git(&payload.repo_path, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .map_err(|e| format!("当前分支没有 upstream，无法拉取。\n{}\n\n建议: 请先推送并设置 upstream，或手动设置上游分支。", e))?;

    let fetch = run_git_capture_streaming(&payload.repo_path, &["fetch", "--progress", "--prune"], &mut on_progress)?;
    commands.push(fetch.command);
    stdout.push_str(&fetch.stdout);
    stderr.push_str(&fetch.stderr);

    if run_git(&payload.repo_path, &["rev-parse", "--verify", &upstream]).is_err() {
        return Err(format!(
            "当前分支配置的 upstream 是 {}，但 fetch 后没有找到这个远端引用。\n\n建议: 请检查远端分支是否存在，或重新设置当前分支的 upstream。",
            upstream
        ));
    }

    let (merge_success, merge) =
        run_git_capture_status(&payload.repo_path, &["merge", "--no-edit", &upstream])?;
    commands.push(merge.command);
    stdout.push_str(&merge.stdout);
    stderr.push_str(&merge.stderr);

    if !merge_success {
        if has_unmerged_files(&payload.repo_path) {
            return Ok(GitCommandResult {
                command: commands.join("\n"),
                message: "Pull 已拉取远端变更，但合并产生冲突。请解决冲突后标记 resolved，再提交合并结果。".to_string(),
                stdout,
                stderr,
                suggestion: Some("远端代码已经进入本地工作区。请在冲突列表中打开文件解决冲突，然后标记已解决并完成提交。".to_string()),
            });
        }

        let suggestion = git_error_suggestion(&stderr);
        return Err(format_command_error(
            &commands.join("\n"),
            &stdout,
            &stderr,
            suggestion.as_deref(),
        ));
    }

    Ok(GitCommandResult {
        command: commands.join("\n"),
        message: if stdout.trim().is_empty() {
            "Already up to date".to_string()
        } else {
            stdout.trim().to_string()
        },
        stdout,
        stderr,
        suggestion: None,
    })
}

pub fn rebase_changes(payload: &GitRebasePayload) -> Result<GitCommandResult, String> {
    rebase_changes_with_progress(payload, |_| {})
}

pub fn rebase_changes_with_progress<F>(payload: &GitRebasePayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut commands = Vec::new();
    let upstream = run_git(
        &payload.repo_path,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .map_err(|e| format!("当前分支没有 upstream，无法 rebase。\n{}\n\n建议: 请先推送并设置 upstream，或手动设置上游分支。", e))?;

    let fetch = run_git_capture_streaming(&payload.repo_path, &["fetch", "--progress", "--prune"], &mut on_progress)?;
    commands.push(fetch.command);
    stdout.push_str(&fetch.stdout);
    stderr.push_str(&fetch.stderr);

    if run_git(&payload.repo_path, &["rev-parse", "--verify", &upstream]).is_err() {
        return Err(format!(
            "当前分支配置的 upstream 是 {}，但 fetch 后没有找到这个远端引用。\n\n建议: 请检查远端分支是否存在，或重新设置当前分支的 upstream。",
            upstream
        ));
    }

    let (rebase_success, rebase) = run_git_capture_status(&payload.repo_path, &["rebase", &upstream])?;
    commands.push(rebase.command);
    stdout.push_str(&rebase.stdout);
    stderr.push_str(&rebase.stderr);

    if !rebase_success {
        if has_unmerged_files(&payload.repo_path) || load_repository_state(&payload.repo_path).rebase_in_progress {
            return Ok(GitCommandResult {
                command: commands.join("\n"),
                message: "Rebase 已开始，但产生冲突。请解决冲突后标记 resolved，再继续 rebase。".to_string(),
                stdout,
                stderr,
                suggestion: Some("远端代码已经拉取，本地提交正在重放。请解决冲突后点击 Continue Rebase，或 Abort Rebase 回到 rebase 前状态。".to_string()),
            });
        }

        let suggestion = git_error_suggestion(&stderr);
        return Err(format_command_error(
            &commands.join("\n"),
            &stdout,
            &stderr,
            suggestion.as_deref(),
        ));
    }

    Ok(GitCommandResult {
        command: commands.join("\n"),
        message: if stdout.trim().is_empty() {
            "Rebase completed".to_string()
        } else {
            stdout.trim().to_string()
        },
        stdout,
        stderr,
        suggestion: None,
    })
}

pub fn continue_rebase(payload: &GitRepoPayload) -> Result<GitCommandResult, String> {
    let (success, result) = run_git_capture_status(&payload.repo_path, &["rebase", "--continue"])?;
    if success {
        return Ok(GitCommandResult {
            message: if result.stdout.trim().is_empty() {
                "Rebase continued".to_string()
            } else {
                result.stdout.trim().to_string()
            },
            ..result
        });
    }

    if has_unmerged_files(&payload.repo_path) || load_repository_state(&payload.repo_path).rebase_in_progress {
        return Ok(GitCommandResult {
            message: "Rebase 仍有冲突或待处理步骤。请继续解决冲突后再执行 Continue Rebase。".to_string(),
            suggestion: Some("如果不想继续 rebase，可以执行 Abort Rebase。".to_string()),
            ..result
        });
    }

    let suggestion = git_error_suggestion(&result.stderr);
    Err(format_command_error(
        &result.command,
        &result.stdout,
        &result.stderr,
        suggestion.as_deref(),
    ))
}

pub fn abort_rebase(payload: &GitRepoPayload) -> Result<GitCommandResult, String> {
    let result = run_git_capture(&payload.repo_path, &["rebase", "--abort"])?;

    Ok(GitCommandResult {
        message: "Rebase 已中止".to_string(),
        ..result
    })
}

pub fn continue_merge(payload: &GitRepoPayload) -> Result<GitCommandResult, String> {
    let result = run_git_capture(&payload.repo_path, &["commit", "--no-edit"])?;

    Ok(GitCommandResult {
        message: "Merge commit 已完成".to_string(),
        ..result
    })
}

pub fn fetch_changes(payload: &GitRepoPayload) -> Result<GitCommandResult, String> {
    fetch_changes_with_progress(payload, |_| {})
}

pub fn fetch_changes_with_progress<F>(payload: &GitRepoPayload, on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let result = run_git_capture_streaming(&payload.repo_path, &["fetch", "--progress", "--prune"], on_progress)?;

    Ok(GitCommandResult {
        message: if result.stdout.trim().is_empty() {
            "Fetch completed".to_string()
        } else {
            result.stdout.trim().to_string()
        },
        ..result
    })
}

pub fn abort_merge(payload: &GitRepoPayload) -> Result<GitCommandResult, String> {
    let result = run_git_capture(&payload.repo_path, &["merge", "--abort"])?;

    Ok(GitCommandResult {
        message: "Merge 已中止".to_string(),
        ..result
    })
}
