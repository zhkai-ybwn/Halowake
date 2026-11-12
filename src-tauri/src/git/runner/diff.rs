use std::{collections::{HashMap, HashSet}, fs, io::Write, path::PathBuf, thread};

use crate::git::models::{
    GitFileActionPayload, GitFileDiffResponse, GitFileStat, GitSnapshot,
};

use super::branches::load_branches;
use super::process::{
    git_command, run_command_with_timeout, run_git, run_git_raw, run_git_with_paths,
    DEFAULT_GIT_COMMAND_TIMEOUT,
};
use super::sync::{load_repository_state, status_has_unmerged_files};

pub fn load_git_status(repo_path: &str) -> Result<String, String> {
    load_status_with_unchanged_paths(repo_path, "--untracked-files=all").map(|(status, _)| status)
}

fn load_status_with_unchanged_paths(repo_path: &str, untracked: &str) -> Result<(String, Vec<String>), String> {
    let status = run_git_raw(
        repo_path,
        &["--no-optional-locks", "status", "--porcelain=v1", untracked],
    )?;
    if !status.lines().any(|line| line.starts_with(" M ")) {
        return Ok((status, vec![]));
    }

    // Status can report a stat/EOL mismatch even when Git's clean conversion
    // produces the indexed content. Use names, not numstat: binary files and
    // mode-only changes have no added/removed lines but must remain visible.
    let changed = run_git_raw(
        repo_path,
        &[
            "diff", "--name-only", "-z", "--no-renames", "--no-ext-diff",
            "--no-textconv", "--ignore-submodules=none", "--",
        ],
    )?;
    let changed: HashSet<&str> = changed.split('\0').filter(|path| !path.is_empty()).collect();
    let mut unchanged_paths = Vec::new();
    let filtered = status
        .lines()
        .filter(|line| {
            if !line.starts_with(" M ") {
                return true;
            }
            let raw_path = line.get(3..).unwrap_or_default();
            let path = if raw_path.starts_with('"') {
                // Keep unfamiliar Git C-style escapes conservatively.
                match serde_json::from_str::<String>(raw_path) {
                    Ok(path) => path,
                    Err(_) => return true,
                }
            } else {
                raw_path.to_string()
            };
            if changed.contains(path.as_str()) {
                return true;
            }
            unchanged_paths.push(path);
            false
        })
        .map(|line| format!("{line}\n"))
        .collect();
    Ok((filtered, unchanged_paths))
}

struct IndexRefreshLock(PathBuf);

impl IndexRefreshLock {
    fn path_for(index: &std::path::Path) -> PathBuf {
        let mut path = index.as_os_str().to_os_string();
        path.push(".lock");
        PathBuf::from(path)
    }
}

impl Drop for IndexRefreshLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) fn refresh_unchanged_index_entries(repo_path: &str) -> Result<(), String> {
    let (_, unchanged_paths) = load_status_with_unchanged_paths(repo_path, "--untracked-files=no")?;
    if unchanged_paths.is_empty() {
        return Ok(());
    }

    let index_path = PathBuf::from(run_git(repo_path, &["rev-parse", "--git-path", "index"])?);
    let index_path = if index_path.is_absolute() {
        index_path
    } else {
        std::env::current_dir().map_err(|error| format!("读取当前目录失败: {}", error))?
            .join(repo_path).join(index_path)
    };
    let original_index = fs::read(&index_path).map_err(|error| format!("读取 Git 索引失败: {}", error))?;
    let index_dir = index_path.parent().ok_or("无法确定 Git 索引目录。")?;
    let mut candidate = tempfile::Builder::new().prefix("halowake-index-").tempfile_in(index_dir)
        .map_err(|error| format!("创建临时 Git 索引失败: {error}"))?;
    candidate.write_all(&original_index).map_err(|error| format!("复制 Git 索引失败: {error}"))?;
    let candidate = candidate.into_temp_path();
    let _candidate_lock = IndexRefreshLock(IndexRefreshLock::path_for(&candidate));

    // Validate in a private index. Lock the real index only when publishing
    // and only if it is still the one we copied.
    let run_in_index = |args: &[&str]| -> Result<Vec<u8>, String> {
        let mut command = git_command(repo_path);
        command.env("GIT_INDEX_FILE", candidate.as_os_str()).args(args);
        let output = run_command_with_timeout(command, DEFAULT_GIT_COMMAND_TIMEOUT, "git index refresh")?;
        if !output.status.success() {
            return Err(format!("校准 Git 索引失败: {}", String::from_utf8_lossy(&output.stderr).trim()));
        }
        Ok(output.stdout)
    };
    let entries_before = run_in_index(&["ls-files", "--stage", "-z"])?;
    let mut command = git_command(repo_path);
    command.env("GIT_INDEX_FILE", candidate.as_os_str());
    let added = run_git_with_paths(
        command,
        &["--literal-pathspecs", "add", "--pathspec-from-file=-", "--pathspec-file-nul"],
        unchanged_paths.iter().map(String::as_str),
        DEFAULT_GIT_COMMAND_TIMEOUT,
    )?;
    if !added.status.success() {
        return Err(format!("校准 Git 索引失败: {}", String::from_utf8_lossy(&added.stderr).trim()));
    }
    let entries_after = run_in_index(&["ls-files", "--stage", "-z"])?;
    if entries_before != entries_after {
        // The candidate now has an actual content/mode change. Discard the
        // temporary index and let checkout apply Git's normal protections.
        return Ok(());
    }
    let lock_path = IndexRefreshLock::path_for(&index_path);
    let lock_file = fs::OpenOptions::new().write(true).create_new(true).open(&lock_path)
        .map_err(|error| format!("校准 Git 索引失败，无法锁定索引: {error}"))?;
    let _index_lock = IndexRefreshLock(lock_path);
    drop(lock_file);
    if fs::read(&index_path).map_err(|error| format!("读取 Git 索引失败: {}", error))? != original_index {
        return Err("Git 索引已被其他程序修改，请刷新后重试。".to_string());
    }
    fs::rename(candidate.as_os_str(), &index_path).map_err(|error| format!("更新 Git 索引失败: {}", error))?;
    Ok(())
}

pub fn load_git_snapshot(repo_path: &str, include_stats: bool) -> Result<GitSnapshot, String> {
    let (repo_root, branch, repository_state, status_raw, staged_files_raw, file_stats, branches) =
        thread::scope(|scope| {
            let repo_root =
                scope.spawn(|| run_git(repo_path, &["rev-parse", "--show-toplevel"]));
            let branch = scope.spawn(|| run_git(repo_path, &["branch", "--show-current"]));
            let repository_state = scope.spawn(|| load_repository_state(repo_path));
            let status_raw = scope.spawn(|| load_git_status(repo_path));
            let staged_files_raw =
                scope.spawn(|| run_git(repo_path, &["diff", "--cached", "--name-only", "--no-ext-diff", "--no-textconv"]));
            let branches = scope.spawn(|| load_branches(repo_path));
            let status_raw = status_raw
                .join()
                .map_err(|_| "读取文件状态任务异常".to_string())??;
            let file_stats = if include_stats && !status_has_unmerged_files(&status_raw) {
                load_file_stats(repo_path)?
            } else {
                vec![]
            };

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
                status_raw,
                staged_files_raw
                    .join()
                    .map_err(|_| "读取暂存文件任务异常".to_string())??,
                file_stats,
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
        vec!["--literal-pathspecs", "diff", "--cached", unified, "--no-color", "--no-ext-diff", "--no-textconv", "--", file_path]
    } else {
        vec!["--literal-pathspecs", "diff", unified, "--no-color", "--no-ext-diff", "--no-textconv", "--", file_path]
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

// Keep Windows argv bounded for commands that do not support --pathspec-from-file.
pub(crate) fn load_selected_file_diffs<'a>(
    repo_path: &str,
    paths: &[&'a str],
) -> Result<HashMap<&'a str, String>, String> {
    let mut diffs = HashMap::new();
    let mut start = 0;
    while start < paths.len() {
        let mut end = start;
        let mut command_units = 0;
        for path in &paths[start..] {
            command_units += 2 * path.encode_utf16().count() + 3;
            if command_units > 24_000 && end > start {
                break;
            }
            end += 1;
        }
        let batch = &paths[start..end];
        let staged = load_patch_batch(repo_path, batch, true)?;
        let unstaged = load_patch_batch(repo_path, batch, false)?;
        if let (Some(staged), Some(unstaged)) = (staged, unstaged) {
            for (path, patch) in staged.into_iter().chain(unstaged) {
                let diff: &mut String = diffs.entry(path).or_default();
                if !diff.is_empty() {
                    diff.push('\n');
                }
                diff.push_str(&patch);
            }
        } else {
            // A type change or diff configuration can emit multiple patch
            // sections for one raw entry. Resolve those paths individually.
            for path in batch {
                diffs.insert(*path, load_selected_file_diff(repo_path, path)?);
            }
        }
        start = end;
    }
    Ok(diffs)
}

fn load_patch_batch<'a>(
    repo_path: &str,
    paths: &[&'a str],
    staged: bool,
) -> Result<Option<HashMap<&'a str, String>>, String> {
    let mut args = vec![
        "--literal-pathspecs", "diff", "--raw", "--patch", "-z", "--no-renames",
        "--unified=3", "--no-color", "--no-ext-diff", "--no-textconv",
    ];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.extend(paths.iter().copied());
    let output = run_git_raw(repo_path, &args)?;
    if output.is_empty() {
        return Ok(Some(HashMap::new()));
    }
    let Some((raw, patch)) = output.split_once("\0\0") else {
        return Ok(None);
    };
    let raw_fields: Vec<_> = raw.split('\0').collect();
    if raw_fields.len() % 2 != 0 || !patch.starts_with("diff --git ") {
        return Ok(None);
    }
    let patch_starts: Vec<_> = std::iter::once(0)
        .chain(patch.match_indices("\ndiff --git ").map(|(index, _)| index + 1))
        .chain(std::iter::once(patch.len()))
        .collect();
    if patch_starts.len() != raw_fields.len() / 2 + 1 {
        return Ok(None);
    }
    let path_set: HashSet<_> = paths.iter().copied().collect();
    let mut diffs = HashMap::new();
    for (fields, bounds) in raw_fields.chunks_exact(2).zip(patch_starts.windows(2)) {
        let Some(&path) = path_set.get(fields[1]) else {
            return Ok(None);
        };
        if !fields[0].starts_with(':') || diffs.contains_key(path) {
            return Ok(None);
        }
        diffs.insert(path, patch[bounds[0]..bounds[1]].to_string());
    }
    Ok(Some(diffs))
}

pub fn load_file_stats(repo_path: &str) -> Result<Vec<GitFileStat>, String> {
    if run_git(repo_path, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Ok(vec![]);
    }

    let unstaged_raw = run_git_raw(repo_path, &["diff", "--numstat", "--no-ext-diff", "--no-textconv", "--"])?;
    let staged_raw = run_git_raw(repo_path, &["diff", "--cached", "--numstat", "--no-ext-diff", "--no-textconv", "--"])?;

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
        &["--literal-pathspecs", "diff", unified, "--no-color", "--no-ext-diff", "--no-textconv", "--", file_path],
    )?;
    let staged = run_git_raw(
        repo_path,
        &["--literal-pathspecs", "diff", "--cached", unified, "--no-color", "--no-ext-diff", "--no-textconv", "--", file_path],
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
        let untracked = run_git_raw(
            repo_path,
            &["--literal-pathspecs", "ls-files", "--others", "--exclude-standard", "-z", "--", file_path],
        )?;
        if !untracked.split('\0').any(|path| path == file_path) {
            return Ok(String::new());
        }
        return load_untracked_file_diff(repo_path, file_path, unified);
    }

    Ok(result)
}

pub(crate) fn load_untracked_file_diff(
    repo_path: &str,
    file_path: &str,
    unified: &str,
) -> Result<String, String> {
    let mut command = git_command(repo_path);
    command.args([
            "diff",
            "--no-index",
            unified,
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--",
            "/dev/null",
            file_path,
        ]);
    let output = run_command_with_timeout(command, DEFAULT_GIT_COMMAND_TIMEOUT, "git diff --no-index")?;

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
