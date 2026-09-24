use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::git::runner::run_git;

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
    pub is_git_repo: bool,
    pub git_remote_url: Option<String>,
    pub git_branch: Option<String>,
    pub current_commit: Option<String>,
    pub is_dirty: bool,
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
    let is_git_repo = git_dir.exists();

    // 既没有 SKILL.md 也不是 git 仓库，跳过
    if !has_skill_md && !is_git_repo {
        return None;
    }

    let mut name = folder_name.clone();
    let mut description = String::new();
    let mut version = None;

    if has_skill_md {
        if let Ok(content) = fs::read_to_string(&skill_md_path) {
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

    let path_str = entry_path.to_string_lossy().to_string();
    let mut git_remote_url = None;
    let mut git_branch = None;
    let mut current_commit = None;
    let mut is_dirty = false;

    if is_git_repo {
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
        is_git_repo,
        git_remote_url,
        git_branch,
        current_commit,
        is_dirty,
        has_update: false,
        latest_commit: None,
        latest_version: None,
        update_summary: None,
        commits_behind: 0,
        last_checked_at: None,
    })
}

fn scan_agent_skills_sync(project_cwd: Option<&str>) -> Vec<AgentSkill> {
    let targets = collect_agent_targets(project_cwd);
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

            if let Some(skill) =
                scan_single_skill_dir(&path, target.agent_type, target.agent_label)
            {
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

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn execute_git_clone_silent(git_url: &str, target_dir: &Path) -> Result<(), String> {
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {}", e))?;
    }

    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args(["clone", "--depth", "1", git_url]);
    cmd.arg(target_dir);

    let output = cmd.output().map_err(|e| format!("无法启动 git 命令: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "git clone 失败，请检查网络或仓库地址".to_string()
        } else {
            stderr
        });
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

fn deploy_skill_files(git_url: &str, sub_path: Option<&str>, target_dir: &Path) -> Result<(), String> {
    if let Some(sub) = sub_path.filter(|s| !s.trim().is_empty()) {
        let temp_dir = std::env::temp_dir().join(format!("lumina-subskill-{}", current_timestamp_millis()));
        execute_git_clone_silent(git_url, &temp_dir)?;
        let sub_src = temp_dir.join(sub);
        if !sub_src.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(format!("在仓库中未找到子技能目录: {}", sub));
        }
        if let Err(e) = copy_dir_all(&sub_src, target_dir) {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(format!("复制子技能文件失败: {}", e));
        }
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    } else {
        execute_git_clone_silent(git_url, target_dir)
    }
}

fn install_agent_skill_sync(args: InstallSkillArgs) -> Result<InstallSkillResult, String> {
    if args.git_url.trim().is_empty() {
        return Err("Git 仓库地址不能为空".to_string());
    }

    let clean_id = args.skill_id.trim();
    if clean_id.is_empty() {
        return Err("技能 ID 不能为空".to_string());
    }

    // 若指定了子技能 ID，安装后目录名采用子技能 ID；否则采用主技能 ID
    let effective_id = match args.sub_skill_id.as_deref() {
        Some(sid) if !sid.trim().is_empty() => sid.trim(),
        _ => clean_id,
    };

    let mode = args
        .target_mode
        .as_deref()
        .or(args.scope.as_deref())
        .unwrap_or("project");

    let mut installed_paths: Vec<String> = Vec::new();

    if mode == "project" {
        let cwd = match args.project_cwd.as_deref() {
            Some(c) if !c.trim().is_empty() => c.trim(),
            _ => return Err("未指定当前项目工作区目录，请先打开或选择项目".to_string()),
        };

        let target_dir = Path::new(cwd).join(".agents").join("skills").join(effective_id);
        if target_dir.exists() {
            return Err(format!("项目目录中已存在该技能: {}", target_dir.display()));
        }

        deploy_skill_files(&args.git_url, args.sub_path.as_deref(), &target_dir)?;
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
                installed_paths: vec![target_dir.to_string_lossy().to_string()],
            });
        }
        deploy_skill_files(&args.git_url, args.sub_path.as_deref(), &target_dir)?;
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
                "cursor" => home.join(".cursor").join("skills").join(effective_id),
                "trae" => home.join(".trae").join("skills").join(effective_id),
                _ => home.join(".agents").join("skills").join(effective_id),
            };

            if target_dir.exists() {
                continue;
            }

            deploy_skill_files(&args.git_url, args.sub_path.as_deref(), &target_dir)?;
            installed_paths.push(target_dir.to_string_lossy().to_string());
        }

        if installed_paths.is_empty() {
            return Ok(InstallSkillResult {
                success: true,
                message: "指定的目标目录中均已存在该技能".to_string(),
                installed_paths,
            });
        }
    }

    Ok(InstallSkillResult {
        success: true,
        message: format!("成功安装技能至 {} 个目标路径", installed_paths.len()),
        installed_paths,
    })
}

#[tauri::command]
pub async fn install_agent_skill(args: InstallSkillArgs) -> Result<InstallSkillResult, String> {
    tokio::task::spawn_blocking(move || install_agent_skill_sync(args))
        .await
        .map_err(|e| format!("安装任务异常: {}", e))?
}

#[tauri::command]
pub async fn uninstall_agent_skill(skill_path: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let target = Path::new(&skill_path);
        if !target.exists() {
            return Err(format!("路径不存在: {}", skill_path));
        }

        let path_str = skill_path.replace('\\', "/").to_lowercase();
        if !path_str.contains("skill") && !path_str.contains("rule") && !path_str.contains(".agents") {
            return Err("出于安全限制，只能删除技能或规则目录".to_string());
        }

        fs::remove_dir_all(target).map_err(|e| format!("删除技能目录失败: {}", e))?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("卸载任务异常: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

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
