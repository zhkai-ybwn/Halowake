use std::collections::HashMap;

use crate::git::models::{
    GitCommitChangedFile, GitCommitDetail, GitCommitDetailPayload, GitCommitFileDiffPayload,
    GitCommitFileDiffResponse, GitLogEntry, GitLogPayload,
};

use super::process::{run_git, run_git_raw, run_git_raw_owned};

pub fn load_git_log(payload: &GitLogPayload) -> Result<Vec<GitLogEntry>, String> {
    if run_git(&payload.repo_path, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Ok(vec![]);
    }

    let mut args = vec![
        "log".to_string(),
        "--max-count=1000".to_string(),
        "--date=iso-strict".to_string(),
        "--pretty=format:%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s".to_string(),
    ];

    if let Some(file_path) = payload
        .file_path
        .as_ref()
        .map(|file| file.trim())
        .filter(|file| !file.is_empty())
    {
        args.push("--follow".to_string());
        args.push("--".to_string());
        args.push(file_path.to_string());
    }

    let raw = run_git_raw_owned(&payload.repo_path, &args)?;

    Ok(raw
        .lines()
        .filter_map(parse_log_line)
        .collect::<Vec<_>>())
}

pub fn load_git_commit_detail(payload: &GitCommitDetailPayload) -> Result<GitCommitDetail, String> {
    let hash = payload.hash.trim();
    if hash.is_empty() {
        return Err("Commit hash cannot be empty".to_string());
    }

    let meta = run_git_raw(
        &payload.repo_path,
        &[
            "show",
            "-s",
            "--date=iso-strict",
            "--pretty=format:%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s%x1f%b",
            hash,
        ],
    )?;
    let mut parts = meta.splitn(7, '\x1f');
    let changed_files_raw = run_git_raw(&payload.repo_path, &["show", "--format=", "--name-status", "-M", hash])?;
    let numstat_raw = run_git_raw(&payload.repo_path, &["show", "--format=", "--numstat", "-M", hash])?;
    let file_stats = parse_commit_numstat(&numstat_raw);
    let short_stat = run_git_raw(&payload.repo_path, &["show", "--format=", "--shortstat", hash])?;

    Ok(GitCommitDetail {
        hash: parts.next().unwrap_or_default().to_string(),
        short_hash: parts.next().unwrap_or_default().to_string(),
        author_name: parts.next().unwrap_or_default().to_string(),
        author_email: parts.next().unwrap_or_default().to_string(),
        date: parts.next().unwrap_or_default().to_string(),
        subject: parts.next().unwrap_or_default().to_string(),
        body: parts.next().unwrap_or_default().trim().to_string(),
        short_stat: short_stat.trim().to_string(),
        changed_files: changed_files_raw
            .lines()
            .filter_map(|line| parse_commit_changed_file(line, &file_stats))
            .collect::<Vec<_>>(),
    })
}

pub fn load_git_commit_file_diff(payload: &GitCommitFileDiffPayload) -> Result<GitCommitFileDiffResponse, String> {
    let hash = payload.hash.trim();
    let file_path = payload.file_path.trim();
    if hash.is_empty() {
        return Err("Commit hash cannot be empty".to_string());
    }
    if file_path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }

    let diff = run_git_raw(
        &payload.repo_path,
        &[
            "show",
            "--format=",
            if payload.full_context.unwrap_or(false) { "--unified=2147483647" } else { "--unified=3" },
            "--no-color",
            hash,
            "--",
            file_path,
        ],
    )?;

    Ok(GitCommitFileDiffResponse {
        hash: hash.to_string(),
        file_path: file_path.to_string(),
        diff,
    })
}

fn parse_log_line(line: &str) -> Option<GitLogEntry> {
    let mut parts = line.splitn(6, '\x1f');
    Some(GitLogEntry {
        hash: parts.next()?.to_string(),
        short_hash: parts.next()?.to_string(),
        author_name: parts.next()?.to_string(),
        author_email: parts.next()?.to_string(),
        date: parts.next()?.to_string(),
        subject: parts.next()?.to_string(),
    })
}

fn parse_commit_numstat(raw: &str) -> HashMap<String, (Option<usize>, Option<usize>)> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let added = parse_optional_line_count(parts.next()?);
            let removed = parse_optional_line_count(parts.next()?);
            let path = parts.next()?.to_string();
            Some((path, (added, removed)))
        })
        .collect()
}

fn parse_optional_line_count(value: &str) -> Option<usize> {
    value.parse::<usize>().ok()
}

pub(super) fn parse_commit_changed_file(
    line: &str,
    file_stats: &HashMap<String, (Option<usize>, Option<usize>)>,
) -> Option<GitCommitChangedFile> {
    let parts = line.split('\t').collect::<Vec<_>>();
    let status = parts.first()?.trim();
    if status.is_empty() {
        return None;
    }

    if status.starts_with('R') || status.starts_with('C') {
        let path = parts.get(2).unwrap_or(parts.get(1)?).to_string();
        let (added, removed) = file_stats.get(&path).copied().unwrap_or((None, None));
        return Some(GitCommitChangedFile {
            status: status.to_string(),
            original_path: parts.get(1).map(|path| (*path).to_string()),
            path,
            added,
            removed,
        });
    }

    let path = parts.get(1)?.to_string();
    let (added, removed) = file_stats.get(&path).copied().unwrap_or((None, None));
    Some(GitCommitChangedFile {
        status: status.to_string(),
        path,
        original_path: None,
        added,
        removed,
    })
}
