use crate::git::models::{
    GitBranch, GitBranchKind, GitBranchPayload, GitBranchUpstreamPayload, GitCommandResult,
    GitConfigureRemotePayload, GitRemoteBranchCheckoutPayload, GitRepairUpstreamPayload,
};

use super::process::{run_git, run_git_capture, run_git_raw};

pub fn configure_origin_remote(payload: &GitConfigureRemotePayload) -> Result<GitCommandResult, String> {
    let remote_url = payload.remote_url.trim();
    if remote_url.is_empty() {
        return Err("Remote URL cannot be empty".to_string());
    }

    let result = if run_git(&payload.repo_path, &["remote", "get-url", "origin"]).is_ok() {
        run_git_capture(&payload.repo_path, &["remote", "set-url", "origin", remote_url])?
    } else {
        run_git_capture(&payload.repo_path, &["remote", "add", "origin", remote_url])?
    };

    Ok(GitCommandResult {
        message: format!("origin configured: {}", remote_url),
        ..result
    })
}

pub fn repair_upstream(payload: &GitRepairUpstreamPayload) -> Result<GitCommandResult, String> {
    let branch = run_git(&payload.repo_path, &["branch", "--show-current"])?;
    let branch = branch.trim();
    if branch.is_empty() {
        return Err("Cannot repair upstream because current branch is detached".to_string());
    }

    if run_git(&payload.repo_path, &["remote", "get-url", "origin"]).is_err() {
        return Err("当前仓库没有配置 origin remote，无法修复 upstream。\n\n建议: 请先添加 GitHub 仓库地址。".to_string());
    }

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut commands = Vec::new();

    let fetch = run_git_capture(&payload.repo_path, &["fetch", "--prune", "origin"])?;
    commands.push(fetch.command);
    stdout.push_str(&fetch.stdout);
    stderr.push_str(&fetch.stderr);

    let upstream = format!("origin/{}", branch);
    if run_git(&payload.repo_path, &["rev-parse", "--verify", &upstream]).is_err() {
        return Err(format!(
            "远端还没有 {} 分支，无法直接修复 upstream。\n\n建议: 请使用 Push 发布当前分支，系统会自动执行 git push -u origin {}。",
            branch, branch
        ));
    }

    let repair = run_git_capture(&payload.repo_path, &["branch", "--set-upstream-to", &upstream, branch])?;
    commands.push(repair.command);
    stdout.push_str(&repair.stdout);
    stderr.push_str(&repair.stderr);

    Ok(GitCommandResult {
        command: commands.join("\n"),
        message: format!("Upstream repaired: {}", upstream),
        stdout,
        stderr,
        suggestion: None,
    })
}

pub fn load_branches(repo_path: &str) -> Result<Vec<GitBranch>, String> {
    let output = run_git_raw(repo_path, &["branch", "--all", "--no-color"])?;
    let branches = output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let current = line.starts_with('*');
            let name = line.trim_start_matches('*').trim();
            if name.is_empty() || name.contains(" -> ") {
                return None;
            }
            let (kind, name) = if let Some(name) = name.strip_prefix("remotes/") {
                (GitBranchKind::Remote, name)
            } else {
                (GitBranchKind::Local, name)
            };
            Some(GitBranch {
                name: name.to_string(),
                kind,
                current,
                upstream: None,
                upstream_status: None,
            })
        })
        .collect::<Vec<_>>();

    log::info!("Loaded {} branches for {}", branches.len(), repo_path);
    if branches.is_empty() {
        log::warn!("Empty branch output for {}: {:?}", repo_path, output);
    }

    Ok(branches)
}

pub fn create_branch(payload: &GitBranchPayload) -> Result<GitCommandResult, String> {
    let branch = validate_branch_name(&payload.branch)?;
    let result = run_git_capture(&payload.repo_path, &["checkout", "-b", branch])?;
    Ok(GitCommandResult { message: format!("已创建并切换到分支: {}", branch), ..result })
}

pub fn switch_branch(payload: &GitBranchPayload) -> Result<GitCommandResult, String> {
    let branch = validate_branch_name(&payload.branch)?;
    let result = run_git_capture(&payload.repo_path, &["checkout", branch])?;
    Ok(GitCommandResult { message: format!("已切换到分支: {}", branch), ..result })
}

pub fn checkout_remote_branch(payload: &GitRemoteBranchCheckoutPayload) -> Result<GitCommandResult, String> {
    let remote_branch = payload.remote_branch.trim();
    let local_branch = validate_branch_name(&payload.local_branch)?;
    if !remote_branch.contains('/') || remote_branch.starts_with('-') {
        return Err("远端分支无效。".to_string());
    }
    if run_git(&payload.repo_path, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", local_branch)]).is_ok() {
        return switch_branch(&GitBranchPayload {
            repo_path: payload.repo_path.clone(),
            branch: local_branch.to_string(),
        });
    }
    let result = run_git_capture(
        &payload.repo_path,
        &["checkout", "--track", "-b", local_branch, remote_branch],
    )?;
    Ok(GitCommandResult {
        message: format!("已检出远端分支 {} 到本地分支 {}", remote_branch, local_branch),
        ..result
    })
}

pub fn delete_branch(payload: &GitBranchPayload) -> Result<GitCommandResult, String> {
    let branch = validate_branch_name(&payload.branch)?;
    let current = run_git(&payload.repo_path, &["branch", "--show-current"])?;
    if current.trim() == branch {
        return Err("不能删除当前分支。请先切换到其他分支。".to_string());
    }
    let result = run_git_capture(&payload.repo_path, &["branch", "-d", branch])?;
    Ok(GitCommandResult { message: format!("已删除分支: {}", branch), ..result })
}

pub fn set_branch_upstream(payload: &GitBranchUpstreamPayload) -> Result<GitCommandResult, String> {
    let branch = validate_branch_name(&payload.branch)?;
    let result = match payload.upstream.as_deref().map(str::trim).filter(|upstream| !upstream.is_empty()) {
        Some(upstream) if upstream.contains('/') && !upstream.starts_with('-') => {
            let option = format!("--set-upstream-to={}", upstream);
            run_git_capture(&payload.repo_path, &["branch", &option, branch])?
        }
        Some(_) => return Err("上游分支无效。".to_string()),
        None => run_git_capture(&payload.repo_path, &["branch", "--unset-upstream", branch])?,
    };
    Ok(GitCommandResult { message: format!("已更新分支 {} 的上游设置", branch), ..result })
}

fn validate_branch_name(branch: &str) -> Result<&str, String> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err("分支名不能为空。".to_string());
    }
    if branch.starts_with('-') {
        return Err("分支名不能以连字符开头。".to_string());
    }
    Ok(branch)
}
