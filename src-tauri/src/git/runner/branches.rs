use crate::git::models::{
    GitBranch, GitBranchKind, GitBranchPayload, GitBranchUpstreamPayload, GitCommandProgressEvent,
    GitCommandResult, GitConfigureRemotePayload, GitRemoteBranchCheckoutPayload,
    GitRepairUpstreamPayload,
};

use super::diff::refresh_unchanged_index_entries;
use super::process::{emit_manual_git_progress, run_git, run_git_capture, run_git_checkout, run_git_raw};

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
    let output = read_branch_refs(repo_path, &["refs/heads/", "refs/remotes/"])?;
    let branches = output
        .lines()
        .filter_map(parse_branch_ref)
        .filter_map(|branch| {
            let (kind, name) = if let Some(name) = branch.name.strip_prefix("refs/heads/") {
                (GitBranchKind::Local, name)
            } else {
                if !branch.symbolic_target.is_empty() {
                    return None;
                }
                (GitBranchKind::Remote, branch.name.strip_prefix("refs/remotes/")?)
            };
            Some(GitBranch {
                name: name.to_string(),
                kind,
                current: branch.current,
                upstream: display_upstream(branch.upstream).map(str::to_string),
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

struct BranchRef<'a> {
    name: &'a str,
    upstream: &'a str,
    symbolic_target: &'a str,
    current: bool,
}

fn read_branch_refs(repo_path: &str, refs: &[&str]) -> Result<String, String> {
    let mut args = vec!["for-each-ref", "--format=%(refname)%09%(upstream)%09%(symref)%09%(HEAD)"];
    args.extend_from_slice(refs);
    run_git_raw(repo_path, &args)
}

fn parse_branch_ref(line: &str) -> Option<BranchRef<'_>> {
    let mut fields = line.split('\t');
    Some(BranchRef {
        name: fields.next()?,
        upstream: fields.next()?,
        symbolic_target: fields.next()?,
        current: fields.next()? == "*",
    })
}

fn display_upstream(upstream: &str) -> Option<&str> {
    (!upstream.is_empty()).then(|| upstream.strip_prefix("refs/remotes/").unwrap_or(upstream))
}

pub fn create_branch<F>(payload: &GitBranchPayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let branch = validate_branch_name(&payload.branch)?;
    let result = checkout_worktree(&payload.repo_path, &["checkout", "--progress", "-b", branch], &mut on_progress)?;
    Ok(GitCommandResult { message: format!("已创建并切换到分支: {}", branch), ..result })
}

pub fn switch_branch<F>(payload: &GitBranchPayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let branch = validate_branch_name(&payload.branch)?;
    let result = checkout_worktree(&payload.repo_path, &["switch", "--progress", "--no-guess", branch], &mut on_progress)?;
    Ok(GitCommandResult { message: format!("已切换到分支: {}", branch), ..result })
}

pub fn checkout_remote_branch<F>(payload: &GitRemoteBranchCheckoutPayload, mut on_progress: F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let remote_branch = payload.remote_branch.trim();
    let local_branch = validate_branch_name(&payload.local_branch)?;
    if !remote_branch.contains('/') || remote_branch.starts_with('-') {
        return Err("远端分支无效。".to_string());
    }

    let remote_ref = format!("refs/remotes/{remote_branch}");
    let local_ref = format!("refs/heads/{local_branch}");
    let refs = read_branch_refs(&payload.repo_path, &[&remote_ref, &local_ref])?;
    let refs = refs.lines().filter_map(parse_branch_ref).collect::<Vec<_>>();
    if !refs.iter().any(|branch| branch.name == remote_ref && branch.symbolic_target.is_empty()) {
        return Err(format!("本地没有远端跟踪分支“{remote_branch}”。请 Fetch 后重新选择。"));
    }

    if let Some(existing) = refs.iter().find(|branch| branch.name == local_ref) {
        if existing.upstream != remote_ref {
            let upstream = display_upstream(existing.upstream).unwrap_or("未设置");
            return Err(format!(
                "本地分支“{local_branch}”已存在（上游：{upstream}）。请使用其他本地分支名检出“{remote_branch}”，或直接选择该本地分支。"
            ));
        }

        return switch_branch(
            &GitBranchPayload { repo_path: payload.repo_path.clone(), branch: local_branch.to_string() },
            on_progress,
        );
    }

    let result = checkout_worktree(
        &payload.repo_path,
        &["checkout", "--progress", "--track", "-b", local_branch, &remote_ref],
        &mut on_progress,
    )?;
    Ok(GitCommandResult {
        message: format!("已检出远端分支 {} 到本地分支 {}", remote_branch, local_branch),
        ..result
    })
}

fn checkout_worktree<F>(repo_path: &str, args: &[&str], on_progress: &mut F) -> Result<GitCommandResult, String>
where
    F: FnMut(GitCommandProgressEvent),
{
    let command = format!("git {}", args.join(" "));
    emit_manual_git_progress(repo_path, &command, "Preparing index", None, None, on_progress);
    refresh_unchanged_index_entries(repo_path)?;
    emit_manual_git_progress(repo_path, &command, "Updating worktree", None, None, on_progress);
    run_git_checkout(repo_path, args, on_progress)
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
