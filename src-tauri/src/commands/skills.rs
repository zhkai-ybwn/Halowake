use std::{
    collections::HashMap,
    env, fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use ring::digest::{Context as DigestContext, SHA256};
use tauri::ipc::Channel;
use tauri::State;
use rusqlite::params;

use crate::git::runner::run_git;
use crate::commands::markdown::read_file_bytes_with_dlp_fallback;
use crate::storage::AppDatabase;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub agent_type: String,
    pub agent_label: String,
    pub associated_agents: Vec<String>,
    pub path: String,
    pub installed_at: Option<u64>,
    pub is_git_repo: bool,
    pub git_remote_url: Option<String>,
    pub git_branch: Option<String>,
    pub current_commit: Option<String>,
    pub is_dirty: bool,
    pub can_uninstall: bool,
    pub externally_managed: bool,
    pub has_update: bool,
    pub latest_commit: Option<String>,
    pub latest_version: Option<String>,
    pub update_summary: Option<String>,
    pub commits_behind: usize,
    pub last_checked_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateResult {
    pub skill_id: String,
    pub success: bool,
    pub message: String,
    pub previous_commit: Option<String>,
    pub new_commit: Option<String>,
}

const MANAGED_SOURCE_FILE: &str = ".lumina-skill-source.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedSkillSource {
    git_url: String,
    sub_path: String,
    commit: String,
    content_sha256: String,
}

fn read_managed_source(path: &Path) -> Option<ManagedSkillSource> {
    let text = fs::read_to_string(path.join(MANAGED_SOURCE_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

fn read_skill_markdown(path: &Path) -> Option<String> {
    String::from_utf8(read_file_bytes_with_dlp_fallback(path).ok()?).ok()
}

fn hash_skill_files(root: &Path) -> Result<String, String> {
    fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|error| format!("读取技能目录失败: {error}"))? {
            let entry = entry.map_err(|error| format!("读取技能文件失败: {error}"))?;
            let ty = entry.file_type().map_err(|error| format!("检查技能文件失败: {error}"))?;
            if ty.is_symlink() { return Err("技能目录包含符号链接".to_string()); }
            if ty.is_dir() { collect(&entry.path(), files)?; }
            else if entry.file_name() != MANAGED_SOURCE_FILE { files.push(entry.path()); }
        }
        Ok(())
    }
    let mut files = Vec::new();
    collect(root, &mut files)?;
    files.sort();
    let mut context = DigestContext::new(&SHA256);
    for file in files {
        let relative = file.strip_prefix(root).map_err(|error| format!("技能路径无效: {error}"))?;
        context.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        context.update(&[0]);
        context.update(&read_file_bytes_with_dlp_fallback(&file)?);
        context.update(&[0]);
    }
    Ok(context.finish().as_ref().iter().map(|byte| format!("{byte:02x}")).collect())
}

fn get_home_dir() -> Result<PathBuf, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|_| "无法定位当前用户目录".to_string())?;
    Ok(PathBuf::from(home))
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 判断 YAML 值是否为折叠/字面块标量引导符（如 >, >-, >+, |, |-, |+ 或空引导）
fn is_yaml_block_scalar_indicator(val: &str) -> bool {
    let clean = val.split('#').next().unwrap_or("").trim();
    if clean.is_empty() {
        return true;
    }
    clean.starts_with('>') || clean.starts_with('|')
}

/// 解析 SKILL.md 中的 YAML frontmatter
/// 返回: (name, description, version)
pub(crate) fn parse_skill_md(content: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut lines = content.lines();
    // 检查第一行是否为 ---
    if let Some(first_line) = lines.next() {
        if first_line.trim() != "---" {
            return (None, None, None);
        }
    } else {
        return (None, None, None);
    }

    let mut name = None;
    let mut description = None;
    let mut version = None;
    let mut in_description = false;
    let mut desc_buf = String::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }

        if in_description {
            if !line.starts_with(' ') && !line.starts_with('\t') && line.contains(':') {
                in_description = false;
                if !desc_buf.is_empty() {
                    description = Some(desc_buf.trim().to_string());
                    desc_buf.clear();
                }
            } else {
                if trimmed.is_empty() {
                    continue;
                }
                if !desc_buf.is_empty() {
                    desc_buf.push(' ');
                }
                desc_buf.push_str(trimmed);
                continue;
            }
        }

        let trimmed_lower = trimmed.to_lowercase();
        if trimmed_lower.starts_with("name:") {
            if let Some((_, rest)) = trimmed.split_once(':') {
                name = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        } else if trimmed_lower.starts_with("version:") {
            if let Some((_, rest)) = trimmed.split_once(':') {
                version = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        } else if trimmed_lower.starts_with("description:") {
            if let Some((_, rest)) = trimmed.split_once(':') {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if is_yaml_block_scalar_indicator(val) {
                    in_description = true;
                } else {
                    description = Some(val.to_string());
                }
            }
        }
    }

    if in_description && !desc_buf.is_empty() && description.is_none() {
        description = Some(desc_buf.trim().to_string());
    }

    // 终极防御校验：如果描述仅剩下 YAML 标量引导字符（如 >- 或 | 等），置为 None
    if let Some(ref d) = description {
        let dt = d.trim();
        if dt == ">-" || dt == ">" || dt == "|-" || dt == "|" || dt == ">+" || dt == "|+" {
            description = None;
        }
    }

    (name, description, version)
}

struct AgentTarget {
    agent_type: &'static str,
    agent_label: &'static str,
    dir_path: PathBuf,
}

fn is_uninstallable_root(path: &Path) -> bool {
    matches!(path.file_name().and_then(|name| name.to_str()), Some("skills" | "skills-cursor" | ".skills"))
}

fn skills_cli_lock_contains(lock_path: &Path, folder_name: &str) -> bool {
    let Ok(content) = fs::read_to_string(lock_path) else {
        return lock_path.exists();
    };
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(&content) else {
        return true;
    };
    lock.get("skills")
        .and_then(|skills| skills.as_object())
        .is_some_and(|skills| {
            skills.keys().any(|name| {
                let sanitized = name
                    .to_ascii_lowercase()
                    .chars()
                    .map(|character| {
                        if character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '.'
                            || character == '_'
                        {
                            character
                        } else {
                            '-'
                        }
                    })
                    .collect::<String>();
                let sanitized = sanitized
                    .split('-')
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("-");
                name == folder_name || sanitized.trim_matches(&['.', '-'][..]) == folder_name
            })
        })
}

fn is_skills_cli_managed(path: &Path) -> bool {
    let Some(skill_root) = path.parent() else {
        return false;
    };
    let Some(agents_dir) = skill_root.parent() else {
        return false;
    };
    if skill_root.file_name().and_then(|name| name.to_str()) != Some("skills")
        || agents_dir.file_name().and_then(|name| name.to_str()) != Some(".agents")
    {
        return false;
    }
    let Some(folder_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let mut locks = vec![agents_dir.join(".skill-lock.json")];
    if let Some(project_root) = agents_dir.parent() {
        locks.push(project_root.join("skills-lock.json"));
    }
    if get_home_dir()
        .ok()
        .is_some_and(|home| agents_dir == home.join(".agents"))
    {
        if let Some(xdg_state) = env::var_os("XDG_STATE_HOME") {
            locks.push(PathBuf::from(xdg_state).join("skills").join(".skill-lock.json"));
        }
    }
    locks
        .iter()
        .any(|lock| skills_cli_lock_contains(lock, folder_name))
}

fn is_link_or_reparse(path: &Path) -> Result<bool, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("读取目录信息失败 {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_no_links_in_tree(path: &Path) -> Result<(), String> {
    if is_link_or_reparse(path)? {
        return Err("技能目录包含链接或重解析点，无法安全卸载".to_string());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|error| format!("读取技能目录失败: {error}"))? {
            let entry = entry.map_err(|error| format!("读取技能目录项失败: {error}"))?;
            ensure_no_links_in_tree(&entry.path())?;
        }
    }
    Ok(())
}

fn validate_uninstall_target(path: &Path, targets: &[AgentTarget]) -> Result<(), String> {
    if !path.is_dir() || is_link_or_reparse(path)? {
        return Err("技能目录不存在或是链接".to_string());
    }
    if !path.join("SKILL.md").is_file() {
        return Err("目录中没有 SKILL.md，无法作为独立技能卸载".to_string());
    }
    let canonical = fs::canonicalize(path).map_err(|error| format!("解析技能目录失败: {error}"))?;
    let approved = targets.iter().filter(|target| is_uninstallable_root(&target.dir_path)).any(|target| {
        fs::canonicalize(&target.dir_path).ok().is_some_and(|root| canonical.parent() == Some(root.as_path()))
    });
    if !approved {
        return Err("技能目录不在允许卸载的位置".to_string());
    }
    if is_skills_cli_managed(path) {
        return Err("该技能由 skills CLI 管理，请使用原安装器卸载以同步锁文件".to_string());
    }
    ensure_no_links_in_tree(path)
}

fn collect_agent_targets(project_cwd: Option<&str>) -> Vec<AgentTarget> {
    let mut targets = Vec::new();
    let home = match get_home_dir() {
        Ok(h) => h,
        Err(_) => return targets,
    };

    // 0. 通用 Agents 生态 (~/.agents/skills 与 ~/.skills)
    let agents_candidates = [
        home.join(".agents").join("skills"),
        home.join(".skills"),
    ];
    for dir in agents_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "agents",
                agent_label: "Agents",
                dir_path: dir,
            });
        }
    }

    // 1. Codex CLI
    let codex_skills_dir = if let Ok(codex_home) = env::var("CODEX_HOME") {
        PathBuf::from(codex_home).join("skills")
    } else {
        home.join(".codex").join("skills")
    };
    targets.push(AgentTarget {
        agent_type: "codex",
        agent_label: "Codex",
        dir_path: codex_skills_dir,
    });

    // 2. Antigravity / Gemini CLI
    let gemini_skills_dir = home.join(".gemini").join("config").join("skills");
    targets.push(AgentTarget {
        agent_type: "antigravity",
        agent_label: "Antigravity",
        dir_path: gemini_skills_dir,
    });

    // 3. Cursor
    let cursor_candidates = [
        home.join(".cursor").join("skills-cursor"),
        home.join(".cursor").join("skills"),
        home.join(".cursor").join("rules"),
    ];
    for dir in cursor_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "cursor",
                agent_label: "Cursor",
                dir_path: dir,
            });
        }
    }

    // 4. Trae (字节跳动)
    let trae_candidates = [
        home.join(".trae").join("builtin_skills"),
        home.join(".trae").join("skills"),
        home.join(".trae-cn").join("skills"),
        home.join(".trae-aicc").join("skills"),
    ];
    for dir in trae_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "trae",
                agent_label: "Trae",
                dir_path: dir,
            });
        }
    }

    // 5. Claude Code
    let claude_candidates = [
        home.join(".claude").join("skills"),
        home.join(".claude").join("plugins"),
        home.join(".claude").join("commands"),
    ];
    for dir in claude_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "claude",
                agent_label: "Claude Code",
                dir_path: dir,
            });
        }
    }

    // 6. Qoder / 阿里灵码
    let qoder_candidates = [
        home.join(".qoder-cli").join("skills"),
        home.join(".qoder-cn").join("skills"),
        home.join(".qoder").join("skills"),
        home.join(".lingma").join("skills"),
    ];
    for dir in qoder_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "qoder",
                agent_label: "Qoder (灵码)",
                dir_path: dir,
            });
        }
    }

    // 7. OpenCode
    let opencode_candidates = [
        home.join(".opencode").join("skills"),
        home.join(".opencode").join("tools"),
        home.join(".local").join("share").join("opencode").join("skills"),
    ];
    for dir in opencode_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "opencode",
                agent_label: "OpenCode",
                dir_path: dir,
            });
        }
    }

    // 8. Windsurf (Codeium)
    let windsurf_candidates = [
        home.join(".windsurf").join("skills"),
        home.join(".codeium").join("windsurf").join("skills"),
    ];
    for dir in windsurf_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "windsurf",
                agent_label: "Windsurf",
                dir_path: dir,
            });
        }
    }

    // 9. Cline / Roo Code
    let cline_candidates = [
        home.join(".cline").join("skills"),
        home.join(".roomodes"),
    ];
    for dir in cline_candidates {
        if dir.exists() {
            targets.push(AgentTarget {
                agent_type: "cline",
                agent_label: "Cline",
                dir_path: dir,
            });
        }
    }

    // 10. 项目工作区目录
    if let Some(cwd) = project_cwd {
        let p = PathBuf::from(cwd);
        if p.exists() {
            let project_candidates = [
                p.join(".agents").join("skills"),
                p.join(".skills"),
                p.join(".claude").join("skills"),
                p.join(".gemini").join("skills"),
                p.join(".cursor").join("rules"),
                p.join(".trae").join("rules"),
            ];
            for dir in project_candidates {
                if dir.exists() {
                    targets.push(AgentTarget {
                        agent_type: "project",
                        agent_label: "项目专属",
                        dir_path: dir,
                    });
                }
            }
        }
    }

    targets
}

fn read_git_remote_url_fast(git_dir: &Path) -> Option<String> {
    let content = fs::read_to_string(git_dir.join("config")).ok()?;
    let mut in_remote_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_remote_origin = trimmed.eq_ignore_ascii_case("[remote \"origin\"]");
            continue;
        }
        if in_remote_origin {
            if let Some(rest) = trimmed.strip_prefix("url") {
                let rest = rest.trim();
                if let Some(url_part) = rest.strip_prefix('=') {
                    let url = url_part.trim();
                    if !url.is_empty() {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }
    None
}

fn read_commit_from_packed_refs(git_dir: &Path, target_ref: &str) -> Option<String> {
    let packed = fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    for line in packed.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('^') || trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let hash = parts.next()?;
        let ref_name = parts.next()?;
        if ref_name == target_ref {
            return if hash.len() >= 7 {
                Some(hash[..7].to_string())
            } else {
                Some(hash.to_string())
            };
        }
    }
    None
}

fn read_git_head_fast(git_dir: &Path) -> (Option<String>, Option<String>) {
    let head_content = match fs::read_to_string(git_dir.join("HEAD")) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let trimmed = head_content.trim();
    if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
        let branch = branch_ref.trim().to_string();
        let ref_file = git_dir.join("refs").join("heads").join(&branch);
        let commit = if let Ok(ref_content) = fs::read_to_string(ref_file) {
            let hash = ref_content.trim();
            if hash.len() >= 7 {
                Some(hash[..7].to_string())
            } else if !hash.is_empty() {
                Some(hash.to_string())
            } else {
                None
            }
        } else {
            read_commit_from_packed_refs(git_dir, &format!("refs/heads/{}", branch))
        };
        (Some(branch), commit)
    } else if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        (None, Some(trimmed[..7].to_string()))
    } else {
        (None, None)
    }
}

fn scan_single_skill_dir(
    entry_path: &Path,
    agent_type: &str,
    agent_label: &str,
) -> Option<AgentSkill> {
    if !entry_path.is_dir() {
        return None;
    }

    let folder_name = entry_path.file_name()?.to_string_lossy().to_string();

    // 跳过隐藏目录、系统预置目录与备份目录
    if folder_name.starts_with('.')
        || folder_name == "_shared"
        || folder_name == ".system"
        || folder_name == ".git"
        || folder_name == ".backup"
        || folder_name.starts_with(".tmp")
    {
        return None;
    }

    let skill_md_path = entry_path.join("SKILL.md");
    let has_skill_md = skill_md_path.exists();
    let git_dir = entry_path.join(".git");
    let managed_source = read_managed_source(entry_path);
    let is_git_repo = git_dir.exists() || managed_source.is_some();

    // 既没有 SKILL.md 也不是 git 仓库，跳过
    if !has_skill_md && !is_git_repo {
        return None;
    }

    let mut name = folder_name.clone();
    let mut description = String::new();
    let mut version = None;

    if has_skill_md {
        if let Some(content) = read_skill_markdown(&skill_md_path) {
            let (parsed_name, parsed_desc, parsed_ver) = parse_skill_md(&content);
            if let Some(n) = parsed_name {
                if !n.trim().is_empty() {
                    name = n;
                }
            }
            if let Some(d) = parsed_desc {
                description = d;
            }
            version = parsed_ver;
        }
    }

    // Some installed SKILL.md files are unreadable to this process. The source
    // metadata remains available, so retain the skill's actual directory name
    // instead of showing the generated catalog ID.
    if name == folder_name {
        if let Some(source) = managed_source.as_ref() {
            if let Some(source_name) = source.sub_path.replace('\\', "/").rsplit('/').find(|part| !part.is_empty()) {
                name = source_name.to_string();
            }
        }
    }

    let path_str = entry_path.to_string_lossy().to_string();
    let installed_at = fs::metadata(entry_path)
        .and_then(|metadata| metadata.created().or_else(|_| metadata.modified()))
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    let mut git_remote_url = None;
    let mut git_branch = None;
    let mut current_commit = None;
    let mut is_dirty = false;

    if git_dir.exists() {
        // Fast-path: read directly from .git/config and .git/HEAD
        git_remote_url = read_git_remote_url_fast(&git_dir);
        let (fast_branch, fast_commit) = read_git_head_fast(&git_dir);
        git_branch = fast_branch;
        current_commit = fast_commit;

        // Fallback to git CLI if fast-path did not resolve
        if git_remote_url.is_none() {
            if let Ok(url) = run_git(&path_str, &["config", "--get", "remote.origin.url"]) {
                let trimmed = url.trim().to_string();
                if !trimmed.is_empty() {
                    git_remote_url = Some(trimmed);
                }
            }
        }

        if git_branch.is_none() {
            if let Ok(branch) = run_git(&path_str, &["rev-parse", "--abbrev-ref", "HEAD"]) {
                let trimmed = branch.trim().to_string();
                if !trimmed.is_empty() {
                    git_branch = Some(trimmed);
                }
            }
        }

        if current_commit.is_none() {
            if let Ok(commit) = run_git(&path_str, &["rev-parse", "--short", "HEAD"]) {
                let trimmed = commit.trim().to_string();
                if !trimmed.is_empty() {
                    current_commit = Some(trimmed);
                }
            }
        }

        if let Ok(status) = run_git(&path_str, &["status", "--porcelain"]) {
            is_dirty = !status.trim().is_empty();
        }
    }

    if let Some(source) = managed_source {
        git_remote_url = Some(source.git_url);
        current_commit = Some(source.commit.chars().take(7).collect());
        is_dirty = hash_skill_files(entry_path)
            .map(|digest| digest != source.content_sha256)
            .unwrap_or(true);
    }

    let id = format!("{}:{}", agent_type, folder_name);

    Some(AgentSkill {
        id,
        name,
        description,
        version,
        agent_type: agent_type.to_string(),
        agent_label: agent_label.to_string(),
        associated_agents: vec![agent_type.to_string()],
        path: path_str,
        installed_at,
        is_git_repo,
        git_remote_url,
        git_branch,
        current_commit,
        is_dirty,
        can_uninstall: false,
        externally_managed: false,
        has_update: false,
        latest_commit: None,
        latest_version: None,
        update_summary: None,
        commits_behind: 0,
        last_checked_at: None,
    })
}

fn scan_agent_skills_sync(project_cwd: Option<&str>) -> Vec<AgentSkill> {
    scan_agent_targets(collect_agent_targets(project_cwd))
}

fn scan_agent_targets(targets: Vec<AgentTarget>) -> Vec<AgentSkill> {
    let mut result: Vec<AgentSkill> = Vec::new();
    let mut path_to_index: HashMap<PathBuf, usize> = HashMap::new();

    for target in targets {
        if !target.dir_path.exists() {
            continue;
        }

        let entries = match fs::read_dir(&target.dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

            if let Some(&existing_idx) = path_to_index.get(&canonical) {
                // 已存在该物理目录，避免重复条目，同时关联当前 Agent
                if !result[existing_idx]
                    .associated_agents
                    .contains(&target.agent_type.to_string())
                {
                    result[existing_idx]
                        .associated_agents
                        .push(target.agent_type.to_string());
                    if !result[existing_idx].agent_label.contains(target.agent_label) {
                        result[existing_idx].agent_label = format!(
                            "{} / {}",
                            result[existing_idx].agent_label, target.agent_label
                        );
                    }
                }
                continue;
            }

            if let Some(mut skill) =
                scan_single_skill_dir(&path, target.agent_type, target.agent_label)
            {
                skill.externally_managed = is_skills_cli_managed(&path);
                skill.can_uninstall = is_uninstallable_root(&target.dir_path)
                    && path.join("SKILL.md").is_file()
                    && !is_link_or_reparse(&path).unwrap_or(true)
                    && !skill.externally_managed;
                let idx = result.len();
                result.push(skill);
                path_to_index.insert(canonical, idx);
            }
        }
    }

    result
}

#[tauri::command]
pub async fn scan_agent_skills(project_cwd: Option<String>) -> Result<Vec<AgentSkill>, String> {
    tokio::task::spawn_blocking(move || scan_agent_skills_sync(project_cwd.as_deref()))
        .await
        .map_err(|e| format!("扫描技能失败: {}", e))
}

fn check_agent_skills_updates_sync(project_cwd: Option<&str>) -> Vec<AgentSkill> {
    let mut skills = scan_agent_skills_sync(project_cwd);
    let now = current_timestamp_millis();

    for skill in &mut skills {
        skill.last_checked_at = Some(now);

        if let Some(source) = read_managed_source(Path::new(&skill.path)) {
            if let Ok(remote_commit) = remote_head_commit(&source.git_url) {
                if remote_commit != source.commit {
                    skill.has_update = true;
                    skill.commits_behind = 1;
                    skill.latest_commit = Some(remote_commit.chars().take(7).collect());
                    skill.update_summary = Some("来源仓库已有新提交".to_string());
                }
            }
            continue;
        }

        if !skill.is_git_repo || skill.git_remote_url.is_none() {
            continue;
        }

        // 后台 fetch 远端（忽略非致命错误，例如网络离线）
        let _ = run_git(&skill.path, &["fetch", "--quiet"]);

        // 优先检查 tracking branch
        let mut upstream_ref = None;
        if let Ok(u) = run_git(&skill.path, &["rev-parse", "@{u}"]) {
            let trimmed = u.trim().to_string();
            if !trimmed.is_empty() {
                upstream_ref = Some(trimmed);
            }
        }

        // 若没有默认 tracking branch，尝试以当前分支名字拼装 origin/<branch>
        if upstream_ref.is_none() {
            if let Some(ref b) = skill.git_branch {
                let remote_b = format!("origin/{}", b);
                if let Ok(sha) = run_git(&skill.path, &["rev-parse", &remote_b]) {
                    let trimmed = sha.trim().to_string();
                    if !trimmed.is_empty() {
                        upstream_ref = Some(remote_b);
                    }
                }
            }
        }

        if let Some(ref target_upstream) = upstream_ref {
            let rev_range = format!("HEAD..{}", target_upstream);
            if let Ok(count_str) = run_git(&skill.path, &["rev-list", &rev_range, "--count"]) {
                if let Ok(count) = count_str.trim().parse::<usize>() {
                    if count > 0 {
                        skill.has_update = true;
                        skill.commits_behind = count;

                        // 获取最新提交简要
                        if let Ok(log) = run_git(
                            &skill.path,
                            &["log", "-1", "--pretty=format:%h %s", target_upstream],
                        ) {
                            skill.update_summary = Some(log.trim().to_string());
                        }

                        // 获取最新 short commit sha
                        if let Ok(sha) =
                            run_git(&skill.path, &["rev-parse", "--short", target_upstream])
                        {
                            skill.latest_commit = Some(sha.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    skills
}

#[tauri::command]
pub async fn check_agent_skills_updates(
    project_cwd: Option<String>,
) -> Result<Vec<AgentSkill>, String> {
    tokio::task::spawn_blocking(move || check_agent_skills_updates_sync(project_cwd.as_deref()))
        .await
        .map_err(|e| format!("检查技能更新失败: {}", e))
}

fn update_agent_skill_sync(skill_path: &str, force: bool) -> Result<SkillUpdateResult, String> {
    let p = Path::new(skill_path);
    if !p.exists() || !p.is_dir() {
        return Err(format!("技能目录不存在: {}", skill_path));
    }

    if let Some(source) = read_managed_source(p) {
        validate_skill_git_url(&source.git_url)?;
        let dirty = hash_skill_files(p)
            .map(|digest| digest != source.content_sha256)
            .unwrap_or(true);
        if dirty && !force {
            return Err("技能文件已在本地修改；请确认后使用强制更新".to_string());
        }
        let parent = p.parent().ok_or_else(|| "技能目录无效".to_string())?;
        let replacement = install_temp_path(parent, "lumina-skill-replacement");
        let backup = install_temp_path(parent, "lumina-skill-backup");
        let silent_progress: Channel<InstallSkillProgress> = Channel::new(|_| Ok(()));
        deploy_skill_files(&source.git_url, Some(&source.sub_path), &replacement, &silent_progress)?;
        let new_commit = read_managed_source(&replacement)
            .map(|value| value.commit.chars().take(7).collect());
        if let Err(error) = fs::rename(p, &backup) {
            let _ = fs::remove_dir_all(&replacement);
            return Err(format!("备份当前技能失败: {error}"));
        }
        if let Err(error) = fs::rename(&replacement, p) {
            let _ = fs::rename(&backup, p);
            let _ = fs::remove_dir_all(&replacement);
            return Err(format!("替换技能失败: {error}"));
        }
        let _ = fs::remove_dir_all(&backup);
        return Ok(SkillUpdateResult {
            skill_id: p.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_else(|| "skill".to_string()),
            success: true,
            message: "技能已更新到来源仓库的最新版本".to_string(),
            previous_commit: Some(source.commit.chars().take(7).collect()),
            new_commit,
        });
    }

    if !p.join(".git").exists() {
        return Err("当前技能非 Git 仓库管理，暂不支持直接执行 git pull".to_string());
    }

    let folder_name = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "skill".to_string());

    let previous_commit = run_git(skill_path, &["rev-parse", "--short", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string());

    // 检查本地是否有未提交的改动
    let dirty = run_git(skill_path, &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if dirty && !force {
        return Err("检测到本地文件已修改，为防止覆盖您的改动，请在确认后重试或勾选强制更新。".to_string());
    }

    if dirty && force {
        // 强制更新前，保存临时 stash
        let _ = run_git(
            skill_path,
            &["stash", "save", "halowake-auto-backup-before-update"],
        );
    }

    // 尝试快进合并拉取
    let pull_result = run_git(skill_path, &["pull", "--ff-only"]);

    match pull_result {
        Ok(_) => {
            let new_commit = run_git(skill_path, &["rev-parse", "--short", "HEAD"])
                .ok()
                .map(|s| s.trim().to_string());
            Ok(SkillUpdateResult {
                skill_id: folder_name,
                success: true,
                message: "更新成功".to_string(),
                previous_commit,
                new_commit,
            })
        }
        Err(err) => {
            if force {
                // 若用户要求强制更新且普通 pull 失败，尝试 fetch 后 hard reset
                let _ = run_git(skill_path, &["fetch", "origin"]);
                if let Ok(_) = run_git(skill_path, &["reset", "--hard", "@{u}"]) {
                    let new_commit = run_git(skill_path, &["rev-parse", "--short", "HEAD"])
                        .ok()
                        .map(|s| s.trim().to_string());
                    return Ok(SkillUpdateResult {
                        skill_id: folder_name,
                        success: true,
                        message: "强制同步重置至最新版本成功".to_string(),
                        previous_commit,
                        new_commit,
                    });
                }
            }
            Err(format!("拉取更新失败: {}", err))
        }
    }
}

#[tauri::command]
pub async fn update_agent_skill(
    skill_path: String,
    force: bool,
) -> Result<SkillUpdateResult, String> {
    tokio::task::spawn_blocking(move || update_agent_skill_sync(&skill_path, force))
        .await
        .map_err(|e| format!("执行技能更新失败: {}", e))?
}

#[tauri::command]
pub async fn open_skill_folder(path: String) -> Result<(), String> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err(format!("目录不存在: {}", path));
    }

    let mut command = if cfg!(target_os = "windows") {
        Command::new("explorer")
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };

    command
        .arg(target)
        .spawn()
        .map_err(|e| format!("打开目录失败 ({}): {}", path, e))?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSkillArgs {
    pub skill_id: String,
    pub git_url: String,
    pub target_mode: Option<String>, // "project" | "specific_agent" | "global"
    pub scope: Option<String>,       // fallback for backwards compatibility
    pub target_agents: Vec<String>,
    pub project_cwd: Option<String>,
    pub add_to_gitignore: bool,
    pub sub_skill_id: Option<String>, // 单独装配的子技能 ID
    pub sub_path: Option<String>,     // 仓库中子技能相对路径 (如 "skills/canvas")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSkillResult {
    pub success: bool,
    pub message: String,
    pub installed_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstallHistoryEntry {
    id: i64,
    skill_id: String,
    display_name: String,
    sub_skill_id: Option<String>,
    git_url: String,
    sub_path: Option<String>,
    target_mode: String,
    target_agents: Vec<String>,
    project_cwd: Option<String>,
    status: String,
    message: String,
    installed_paths: Vec<String>,
    existing_paths: Vec<String>,
    created_at: u64,
}

fn save_install_history(database: &AppDatabase, args: &InstallSkillArgs, result: &Result<InstallSkillResult, String>, completed_paths: &[String]) -> Result<(), String> {
    let connection = database.connect()?;
    let (status, message, paths): (&str, &str, &[String]) = match result {
        Ok(value) if value.installed_paths.is_empty() => ("skipped", value.message.as_str(), &value.installed_paths),
        Ok(value) => ("success", value.message.as_str(), &value.installed_paths),
        Err(error) if !completed_paths.is_empty() => ("partial", error.as_str(), completed_paths),
        Err(error) => ("failed", error.as_str(), &[]),
    };
    let agents = serde_json::to_string(&args.target_agents).map_err(|error| format!("序列化安装目标失败: {error}"))?;
    let paths = serde_json::to_string(paths).map_err(|error| format!("序列化安装路径失败: {error}"))?;
    let display_name = completed_paths.first()
        .and_then(|path| read_skill_markdown(&Path::new(path).join("SKILL.md")))
        .and_then(|content| parse_skill_md(&content).0)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| args.sub_skill_id.clone().unwrap_or_else(|| args.skill_id.clone()));
    connection.execute(
        "INSERT INTO skill_install_history (skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, target_agents_json, project_cwd, status, message, installed_paths_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![&args.skill_id, display_name, args.sub_skill_id.as_deref(), &args.git_url, args.sub_path.as_deref(),
            args.target_mode.as_deref().or(args.scope.as_deref()).unwrap_or("project"), agents,
            args.project_cwd.as_deref(), status, message, paths, current_timestamp_millis() as i64],
    ).map_err(|error| format!("写入技能安装记录失败: {error}"))?;
    Ok(())
}

fn load_install_history(database: &AppDatabase) -> Result<Vec<SkillInstallHistoryEntry>, String> {
    let connection = database.connect()?;
    let mut statement = connection.prepare(
        "SELECT id, skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, target_agents_json,
                project_cwd, status, message, installed_paths_json, created_at
         FROM skill_install_history ORDER BY created_at DESC, id DESC"
    ).map_err(|error| format!("准备读取技能安装记录失败: {error}"))?;
    let rows = statement.query_map([], |row| Ok((
        row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?,
        row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, Option<String>>(8)?,
        row.get::<_, String>(9)?, row.get::<_, String>(10)?, row.get::<_, String>(11)?, row.get::<_, i64>(12)?,
    ))).map_err(|error| format!("查询技能安装记录失败: {error}"))?;
    let mut history = Vec::new();
    for row in rows {
        let (id, skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, agents_json,
            project_cwd, status, message, paths_json, created_at) = row.map_err(|error| format!("读取技能安装记录失败: {error}"))?;
        let target_agents: Vec<String> = serde_json::from_str(&agents_json).map_err(|error| format!("解析安装目标失败: {error}"))?;
        let installed_paths: Vec<String> = serde_json::from_str(&paths_json).map_err(|error| format!("解析安装路径失败: {error}"))?;
        let existing_paths = installed_paths.iter().filter(|path| Path::new(path).is_dir()).cloned().collect();
        history.push(SkillInstallHistoryEntry {
            id, skill_id, display_name, sub_skill_id, git_url, sub_path, target_mode, target_agents,
            project_cwd, status, message, installed_paths, existing_paths,
            created_at: created_at.max(0) as u64,
        });
    }
    Ok(history)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSkillProgress {
    phase: &'static str,
    message: String,
    percent: Option<u8>,
}

fn send_install_progress(progress: &Channel<InstallSkillProgress>, phase: &'static str, message: impl Into<String>, percent: Option<u8>) {
    let _ = progress.send(InstallSkillProgress { phase, message: message.into(), percent });
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

static INSTALL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn install_temp_path(parent: &Path, prefix: &str) -> PathBuf {
    let sequence = INSTALL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{}-{}-{}-{}",
        prefix,
        std::process::id(),
        current_timestamp_millis(),
        sequence
    ))
}

fn validate_skill_git_url(git_url: &str) -> Result<(), String> {
    let repo = git_url
        .trim()
        .strip_prefix("https://github.com/")
        .ok_or_else(|| "仅支持 GitHub HTTPS 技能仓库地址".to_string())?
        .trim_end_matches(".git");
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty() || *part == "." || *part == ".."
                || !part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        })
    {
        return Err("GitHub 仓库地址格式无效".to_string());
    }
    Ok(())
}

fn remote_head_commit(git_url: &str) -> Result<String, String> {
    validate_skill_git_url(git_url)?;
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.args(["ls-remote", git_url, "HEAD"])
        .output().map_err(|error| format!("检查技能远端失败: {error}"))?;
    if !output.status.success() { return Err("检查技能远端失败".to_string()); }
    let text = String::from_utf8_lossy(&output.stdout);
    let sha = text.split_whitespace().next().unwrap_or("");
    if sha.len() < 7 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("远端未返回有效提交号".to_string());
    }
    Ok(sha.to_string())
}

fn clone_percent(line: &str) -> Option<u8> {
    let stage_start = if line.contains("Receiving objects:") { 10 }
        else if line.contains("Resolving deltas:") { 80 }
        else if line.contains("Updating files:") { 95 }
        else { return None };
    let stage_weight = match stage_start { 10 => 70, 80 => 15, _ => 3 };
    let number = line.split('%').next()?.rsplit(|character: char| !character.is_ascii_digit()).next()?.parse::<u8>().ok()?;
    Some(stage_start + (number.min(100) as u16 * stage_weight / 100) as u8)
}

fn is_transient_git_network_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    ["unexpected eof", "tls connect error", "ssl_error_syscall", "connection reset", "curl 56", "curl 35"]
        .iter().any(|signal| lower.contains(signal))
}

fn git_error_summary(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    text.split(['\r', '\n'])
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with("fatal:") || line.starts_with("error:"))
        .unwrap_or_else(|| text.trim())
        .to_string()
}

fn run_git_clone_attempt(git_url: &str, target_dir: &Path, sub_path: Option<&str>, progress: &Channel<InstallSkillProgress>) -> Result<(), String> {
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args(["-c", "credential.interactive=never", "clone", "--progress", "--depth", "1"]);
    if sub_path.is_some() {
        cmd.args(["--filter=blob:none", "--sparse"]);
    }
    cmd.arg(git_url)
        .arg(target_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|error| format!("无法启动 git 命令: {error}"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| "无法读取 Git 下载进度".to_string())?;
    let mut tail = Vec::new();
    let mut line = Vec::new();
    let mut last_percent = None;
    let mut buffer = [0u8; 4096];
    loop {
        let count = stderr.read(&mut buffer).map_err(|error| format!("读取 Git 下载进度失败: {error}"))?;
        if count == 0 { break; }
        tail.extend_from_slice(&buffer[..count]);
        if tail.len() > 16_384 { tail.drain(..tail.len() - 16_384); }
        for byte in &buffer[..count] {
            if *byte == b'\r' || *byte == b'\n' {
                if !line.is_empty() {
                    let text = String::from_utf8_lossy(&line);
                    if let Some(percent) = clone_percent(&text) {
                        if last_percent != Some(percent) {
                            send_install_progress(progress, "downloading", text.trim().to_string(), Some(percent));
                            last_percent = Some(percent);
                        }
                    }
                    line.clear();
                }
            } else if line.len() < 2048 {
                line.push(*byte);
            }
        }
    }
    let status = child.wait().map_err(|error| format!("等待 Git 下载失败: {error}"))?;
    if !status.success() {
        let message = git_error_summary(&tail);
        return Err(if message.is_empty() { "git clone 失败，请检查网络或仓库地址".to_string() } else { message });
    }
    Ok(())
}

fn execute_git_clone_silent(git_url: &str, target_dir: &Path, sub_path: Option<&str>, progress: &Channel<InstallSkillProgress>) -> Result<(), String> {
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    send_install_progress(progress, "connecting", "正在连接 GitHub…", None);
    let mut result = run_git_clone_attempt(git_url, target_dir, sub_path, progress);
    if let Err(error) = &result {
        if is_transient_git_network_error(error) {
            send_install_progress(progress, "retrying", "连接中断，正在重试下载…", None);
            if target_dir.exists() {
                fs::remove_dir_all(target_dir).map_err(|cleanup_error| format!("重试前清理临时目录失败: {cleanup_error}"))?;
            }
            std::thread::sleep(Duration::from_millis(700));
            result = run_git_clone_attempt(git_url, target_dir, sub_path, progress);
        }
    }
    result.map_err(|error| {
        if is_transient_git_network_error(&error) {
            format!("连接 GitHub 时 TLS/网络连接中断。请检查网络或代理后重试。\n{error}")
        } else {
            error
        }
    })?;
    if let Some(sub) = sub_path {
        send_install_progress(progress, "extracting", "正在定位并提取子技能…", Some(96));
        let mut sparse = Command::new("git");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            sparse.creation_flags(CREATE_NO_WINDOW);
        }
        let output = sparse
            .arg("-C").arg(target_dir)
            .args(["sparse-checkout", "set", "--", sub])
            .output()
            .map_err(|e| format!("无法展开技能目录: {}", e))?;
        if !output.status.success() {
            return Err(format!("无法展开技能目录 {}: {}", sub, String::from_utf8_lossy(&output.stderr)));
        }
    }
    Ok(())
}

fn append_to_gitignore_if_needed(project_cwd: &str) -> Result<(), String> {
    let p = Path::new(project_cwd);
    if !p.exists() {
        return Ok(());
    }
    let gitignore_path = p.join(".gitignore");
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).unwrap_or_default()
    } else {
        String::new()
    };

    let already_ignored = content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == ".agents/skills"
            || trimmed == ".agents/skills/"
            || trimmed == "/.agents/skills/"
            || trimmed == ".agents"
            || trimmed == ".agents/"
    });

    if !already_ignored {
        let mut new_content = content;
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("\n# AI Agent Skills (Halowake)\n.agents/skills/\n");
        fs::write(&gitignore_path, new_content).map_err(|e| format!("更新 .gitignore 失败: {}", e))?;
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "技能目录包含符号链接"));
        }
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn deploy_skill_files(git_url: &str, sub_path: Option<&str>, target_dir: &Path, progress: &Channel<InstallSkillProgress>) -> Result<(), String> {
    let parent = target_dir.parent().ok_or_else(|| "安装目标目录无效".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("创建安装目录失败: {}", e))?;
    let staging_dir = install_temp_path(parent, "lumina-skill-stage");
    let result = if let Some(sub) = sub_path.filter(|s| !s.trim().is_empty()) {
        let relative = Path::new(sub);
        if !relative.components().all(|component| matches!(component, Component::Normal(_))) {
            return Err("子技能路径必须是仓库内的相对目录".to_string());
        }
        let temp_dir = install_temp_path(&std::env::temp_dir(), "lumina-subskill");
        let copy_result = (|| -> Result<(), String> {
            execute_git_clone_silent(git_url, &temp_dir, Some(sub), progress)?;
            let canonical_src = fs::canonicalize(temp_dir.join(sub))
                .map_err(|e| format!("在仓库中未找到子技能目录 {}: {}", sub, e))?;
            let canonical_repo = fs::canonicalize(&temp_dir)
                .map_err(|e| format!("无法检查仓库目录: {}", e))?;
            if !canonical_src.starts_with(&canonical_repo) || !canonical_src.is_dir() {
                return Err("子技能路径不在仓库目录内".to_string());
            }
            if !canonical_src.join("SKILL.md").is_file() {
                return Err(format!("子目录 {} 中没有 SKILL.md", sub));
            }
            send_install_progress(progress, "copying", "正在复制技能文件…", Some(98));
            copy_dir_all(&canonical_src, &staging_dir)
                .map_err(|e| format!("复制子技能文件失败: {}", e))?;
            let commit = run_git(&temp_dir.to_string_lossy(), &["rev-parse", "HEAD"])
                .map_err(|e| format!("读取技能来源提交失败: {}", e))?
                .trim().to_string();
            let source = ManagedSkillSource {
                git_url: git_url.to_string(),
                sub_path: sub.to_string(),
                commit,
                content_sha256: hash_skill_files(&staging_dir)?,
            };
            let metadata = serde_json::to_vec_pretty(&source)
                .map_err(|e| format!("生成技能来源信息失败: {}", e))?;
            fs::write(staging_dir.join(MANAGED_SOURCE_FILE), metadata)
                .map_err(|e| format!("保存技能来源信息失败: {}", e))
        })();
        let _ = fs::remove_dir_all(&temp_dir);
        copy_result
    } else {
        execute_git_clone_silent(git_url, &staging_dir, None, progress).and_then(|_| {
            if staging_dir.join("SKILL.md").is_file() {
                Ok(())
            } else {
                Err("仓库根目录没有 SKILL.md，请选择具体技能目录".to_string())
            }
        })
    };
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error);
    }
    send_install_progress(progress, "saving", "正在写入目标目录…", Some(99));
    fs::rename(&staging_dir, target_dir).map_err(|e| {
        let _ = fs::remove_dir_all(&staging_dir);
        format!("完成技能安装失败: {}", e)
    })
}

fn install_agent_skill_sync(args: InstallSkillArgs, progress: &Channel<InstallSkillProgress>, installed_paths: &mut Vec<String>) -> Result<InstallSkillResult, String> {
    if args.git_url.trim().is_empty() {
        return Err("Git 仓库地址不能为空".to_string());
    }
    validate_skill_git_url(&args.git_url)?;

    let clean_id = args.skill_id.trim();
    if clean_id.is_empty() {
        return Err("技能 ID 不能为空".to_string());
    }

    // 若指定了子技能 ID，安装后目录名采用子技能 ID；否则采用主技能 ID
    let effective_id = match args.sub_skill_id.as_deref() {
        Some(sid) if !sid.trim().is_empty() => sid.trim(),
        _ => clean_id,
    };
    if !effective_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("技能 ID 只能包含字母、数字、横线和下划线".to_string());
    }

    let mode = args
        .target_mode
        .as_deref()
        .or(args.scope.as_deref())
        .unwrap_or("project");

    if mode == "project" {
        let cwd = match args.project_cwd.as_deref() {
            Some(c) if !c.trim().is_empty() => c.trim(),
            _ => return Err("未指定当前项目工作区目录，请先打开或选择项目".to_string()),
        };

        let target_dir = Path::new(cwd).join(".agents").join("skills").join(effective_id);
        if target_dir.exists() {
            return Err(format!("项目目录中已存在该技能: {}", target_dir.display()));
        }

        deploy_skill_files(&args.git_url, args.sub_path.as_deref(), &target_dir, progress)?;
        installed_paths.push(target_dir.to_string_lossy().to_string());

        if args.add_to_gitignore {
            let _ = append_to_gitignore_if_needed(cwd);
        }
    } else if mode == "global" {
        let home = get_home_dir()?;
        let target_dir = home.join(".agents").join("skills").join(effective_id);
        if target_dir.exists() {
            return Ok(InstallSkillResult {
                success: true,
                message: "全局通用技能目录中已存在该技能".to_string(),
                installed_paths: Vec::new(),
            });
        }
        deploy_skill_files(&args.git_url, args.sub_path.as_deref(), &target_dir, progress)?;
        installed_paths.push(target_dir.to_string_lossy().to_string());
    } else {
        // Specific Agent
        let home = get_home_dir()?;
        let agents = if args.target_agents.is_empty() {
            vec!["antigravity".to_string()]
        } else {
            args.target_agents.clone()
        };

        for agent in agents {
            let target_dir = match agent.as_str() {
                "antigravity" => home.join(".gemini").join("config").join("skills").join(effective_id),
                "claude" => home.join(".claude").join("skills").join(effective_id),
                "codex" => env::var("CODEX_HOME").map(PathBuf::from)
                    .unwrap_or_else(|_| home.join(".codex")).join("skills").join(effective_id),
                "cursor" => home.join(".cursor").join("skills").join(effective_id),
                "trae" => home.join(".trae").join("skills").join(effective_id),
                _ => home.join(".agents").join("skills").join(effective_id),
            };

            if target_dir.exists() {
                continue;
            }

            deploy_skill_files(&args.git_url, args.sub_path.as_deref(), &target_dir, progress)?;
            installed_paths.push(target_dir.to_string_lossy().to_string());
        }

        if installed_paths.is_empty() {
            return Ok(InstallSkillResult {
                success: true,
                message: "指定的目标目录中均已存在该技能".to_string(),
                installed_paths: Vec::new(),
            });
        }
    }

    send_install_progress(progress, "complete", "技能安装完成", Some(100));
    Ok(InstallSkillResult {
        success: true,
        message: format!("成功安装技能至 {} 个目标路径", installed_paths.len()),
        installed_paths: installed_paths.clone(),
    })
}

#[tauri::command]
pub async fn install_agent_skill(database: State<'_, AppDatabase>, args: InstallSkillArgs, on_progress: Channel<InstallSkillProgress>) -> Result<InstallSkillResult, String> {
    let database = database.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut completed_paths = Vec::new();
        let result = install_agent_skill_sync(args.clone(), &on_progress, &mut completed_paths)
            .map_err(|error| if completed_paths.is_empty() { error } else { format!("{error}\n已有 {} 个目标目录安装完成，请查看安装记录。", completed_paths.len()) });
        if let Err(error) = save_install_history(&database, &args, &result, &completed_paths) {
            log::error!("{error}");
        }
        result
    })
        .await
        .map_err(|e| format!("安装任务异常: {}", e))?
}

#[tauri::command]
pub async fn list_skill_install_history(database: State<'_, AppDatabase>) -> Result<Vec<SkillInstallHistoryEntry>, String> {
    let database = database.inner().clone();
    tokio::task::spawn_blocking(move || load_install_history(&database))
        .await
        .map_err(|error| format!("读取技能安装记录任务失败: {error}"))?
}

#[tauri::command]
pub async fn uninstall_agent_skill(skill_path: String, project_cwd: Option<String>) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let target = Path::new(&skill_path);
        let roots = collect_agent_targets(project_cwd.as_deref());
        validate_uninstall_target(target, &roots)?;
        fs::remove_dir_all(target).map_err(|e| format!("删除技能目录失败: {}", e))?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("卸载任务异常: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;

    #[test]
    fn scan_finds_same_skill_in_codex_and_antigravity() {
        let root = env::temp_dir().join(format!("lumina-skill-scan-{}-{}", std::process::id(), current_timestamp_millis()));
        let folder = "corpus-vercel-labs-native-skill-data-native-ui-4089298a";
        let codex = root.join(".codex").join("skills");
        let antigravity = root.join(".gemini").join("config").join("skills");
        for parent in [&codex, &antigravity] {
            let skill = parent.join(folder);
            fs::create_dir_all(&skill).unwrap();
            fs::write(skill.join("SKILL.md"), "---\nname: native-ui\ndescription: Native UI\n---\n").unwrap();
        }
        let skills = scan_agent_targets(vec![
            AgentTarget { agent_type: "codex", agent_label: "Codex", dir_path: codex },
            AgentTarget { agent_type: "antigravity", agent_label: "Antigravity", dir_path: antigravity },
        ]);
        assert_eq!(skills.len(), 2);
        assert!(skills.iter().all(|skill| skill.name == "native-ui" && skill.can_uninstall));
        assert!(skills.iter().any(|skill| skill.agent_type == "codex"));
        assert!(skills.iter().any(|skill| skill.agent_type == "antigravity"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scan_uses_managed_source_name_when_skill_file_is_unreadable() {
        let root = env::temp_dir().join(format!("lumina-skill-name-{}-{}", std::process::id(), current_timestamp_millis()));
        let skill_dir = root.join("corpus-vercel-labs-native-skill-data-native-ui-4089298a");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), [0xff, 0xfe, 0x00]).unwrap();
        let source = ManagedSkillSource {
            git_url: "https://github.com/vercel-labs/native".to_string(),
            sub_path: "skill-data/native-ui".to_string(),
            commit: "96943d4a680f27ea5bfa367c99db6cce58368d49".to_string(),
            content_sha256: "missing".to_string(),
        };
        fs::write(skill_dir.join(MANAGED_SOURCE_FILE), serde_json::to_vec(&source).unwrap()).unwrap();

        let skill = scan_single_skill_dir(&skill_dir, "codex", "Codex").unwrap();
        assert_eq!(skill.name, "native-ui");
        assert_eq!(skill.agent_type, "codex");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_history_keeps_source_and_detects_removed_target() {
        let root = env::temp_dir().join(format!("lumina-skill-history-{}-{}", std::process::id(), current_timestamp_millis()));
        fs::create_dir_all(&root).unwrap();
        let database = AppDatabase::from_path(root.join("history.db"));
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        let skill_path = root.join("native-ui");
        fs::create_dir_all(&skill_path).unwrap();
        fs::write(skill_path.join("SKILL.md"), "---\nname: native-ui\n---\n").unwrap();
        let args = InstallSkillArgs {
            skill_id: "catalog-native-ui".to_string(), git_url: "https://github.com/owner/repo".to_string(),
            target_mode: Some("specific_agent".to_string()), scope: None,
            target_agents: vec!["codex".to_string()], project_cwd: None, add_to_gitignore: false,
            sub_skill_id: None, sub_path: Some("skills/native-ui".to_string()),
        };
        let path = skill_path.to_string_lossy().to_string();
        save_install_history(&database, &args, &Ok(InstallSkillResult {
            success: true, message: "安装成功".to_string(), installed_paths: vec![path.clone()],
        }), &[path.clone()]).unwrap();
        let entries = load_install_history(&database).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display_name, "native-ui");
        assert_eq!(entries[0].target_agents, ["codex"]);
        assert_eq!(entries[0].existing_paths, [path.clone()]);
        save_install_history(&database, &args, &Err("第二个目标失败".to_string()), &[path.clone()]).unwrap();
        let entries = load_install_history(&database).unwrap();
        assert_eq!(entries[0].status, "partial");
        assert_eq!(entries[0].installed_paths, [path]);
        fs::remove_dir_all(&skill_path).unwrap();
        assert!(load_install_history(&database).unwrap()[0].existing_paths.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clone_progress_and_tls_errors_are_recognized() {
        assert_eq!(clone_percent("Receiving objects:  50% (10/20)"), Some(45));
        assert_eq!(clone_percent("Resolving deltas: 100% (2/2)"), Some(95));
        assert!(is_transient_git_network_error("TLS connect error: unexpected eof while reading"));
        assert_eq!(git_error_summary(b"Cloning into 'tmp'...\rReceiving objects: 10%\rfatal: unable to access URL: TLS connect error\n"), "fatal: unable to access URL: TLS connect error");
    }

    #[test]
    fn uninstall_only_accepts_direct_skill_children_of_approved_roots() {
        let root = std::env::temp_dir().join(format!(
            "lumina-uninstall-guard-{}-{}",
            std::process::id(), current_timestamp_millis()
        ));
        let skills_root = root.join(".agents").join("skills");
        let skill = skills_root.join("example");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: example\n---\n").unwrap();
        let targets = vec![AgentTarget {
            agent_type: "project",
            agent_label: "Project",
            dir_path: skills_root.clone(),
        }];
        assert!(validate_uninstall_target(&skill, &targets).is_ok());
        assert!(validate_uninstall_target(&skills_root, &targets).is_err());

        let unrelated = root.join("other-skills").join("example");
        fs::create_dir_all(&unrelated).unwrap();
        fs::write(unrelated.join("SKILL.md"), "skill").unwrap();
        assert!(validate_uninstall_target(&unrelated, &targets).is_err());

        fs::remove_file(skill.join("SKILL.md")).unwrap();
        assert!(validate_uninstall_target(&skill, &targets).is_err());
        fs::write(skill.join("SKILL.md"), "skill").unwrap();
        fs::write(
            root.join("skills-lock.json"),
            r#"{"version":1,"skills":{"example":{"source":"owner/repo"}}}"#,
        )
        .unwrap();
        assert!(is_skills_cli_managed(&skill));
        assert!(validate_uninstall_target(&skill, &targets).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn managed_copy_retains_source_and_detects_local_edits() {
        let root = std::env::temp_dir().join(format!(
            "lumina-managed-skill-test-{}-{}",
            std::process::id(), current_timestamp_millis()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("SKILL.md"), "---\nname: Example\ndescription: Example skill\n---\n").unwrap();
        let source = ManagedSkillSource {
            git_url: "https://github.com/owner/repo".to_string(),
            sub_path: "skills/example".to_string(),
            commit: "1234567890abcdef".to_string(),
            content_sha256: hash_skill_files(&root).unwrap(),
        };
        fs::write(root.join(MANAGED_SOURCE_FILE), serde_json::to_vec(&source).unwrap()).unwrap();
        let skill = scan_single_skill_dir(&root, "codex", "Codex").unwrap();
        assert!(skill.is_git_repo);
        assert_eq!(skill.current_commit.as_deref(), Some("1234567"));
        assert!(!skill.is_dirty);
        fs::write(root.join("SKILL.md"), "changed").unwrap();
        assert!(scan_single_skill_dir(&root, "codex", "Codex").unwrap().is_dirty);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_parse_skill_md() {
        let content = r#"---
name: impeccable
description: Use when the user wants to design or improve UI.
version: 4.0.4
---

# Impeccable
Some instructions here...
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("impeccable"));
        assert_eq!(
            desc.as_deref(),
            Some("Use when the user wants to design or improve UI.")
        );
        assert_eq!(ver.as_deref(), Some("4.0.4"));
    }

    #[test]
    fn test_parse_skill_md_multiline_description() {
        let content = r#"---
name: test-skill
description: >
  Line 1 of description
  Line 2 of description
version: 1.0.0
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("test-skill"));
        assert_eq!(
            desc.as_deref(),
            Some("Line 1 of description Line 2 of description")
        );
        assert_eq!(ver.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn test_parse_skill_md_cursor_block_folded_scalar() {
        let content = r#"---
name: autopilot
description: >-
  Keep a PR merge-ready by triaging comments, resolving clear conflicts, and
  fixing CI in a loop.
metadata:
  category: git
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("autopilot"));
        assert_eq!(
            desc.as_deref(),
            Some("Keep a PR merge-ready by triaging comments, resolving clear conflicts, and fixing CI in a loop.")
        );
        assert_eq!(ver, None);
    }

    #[test]
    fn test_parse_skill_md_cursor_single_block_scalar_before_end() {
        let content = r#"---
name: create-rule
description: >-
  Create Cursor rules for persistent AI guidance.
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("create-rule"));
        assert_eq!(
            desc.as_deref(),
            Some("Create Cursor rules for persistent AI guidance.")
        );
        assert_eq!(ver, None);
    }

    #[test]
    fn test_parse_skill_md_strips_stray_block_tokens() {
        let content = r#"---
name: empty-desc-skill
description: >-
---
"#;
        let (name, desc, ver) = parse_skill_md(content);
        assert_eq!(name.as_deref(), Some("empty-desc-skill"));
        assert_eq!(desc, None);
        assert_eq!(ver, None);
    }
}
