use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::git::runner::run_git;

const PROFILE_DIR: &str = ".lumina";
const PROFILE_FILE: &str = "git-profile.json";
const LOCAL_EXCLUDE_PATTERN: &str = ".lumina/";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitProjectProfileFile {
    pub repo_root: String,
    pub profile_path: String,
    pub content: String,
    pub created: bool,
}

fn profile_path(repo_root: &str) -> PathBuf {
    Path::new(repo_root).join(PROFILE_DIR).join(PROFILE_FILE)
}

fn path_exists(repo_root: &str, relative: &str) -> bool {
    Path::new(repo_root).join(relative).exists()
}

fn project_name(repo_root: &str) -> String {
    Path::new(repo_root)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_string()
}

fn detect_package_manager(repo_root: &str) -> String {
    if path_exists(repo_root, "pnpm-lock.yaml") {
        return "pnpm".to_string();
    }

    if path_exists(repo_root, "yarn.lock") {
        return "yarn".to_string();
    }

    if path_exists(repo_root, "package-lock.json") {
        return "npm".to_string();
    }

    if path_exists(repo_root, "Cargo.lock") {
        return "cargo".to_string();
    }

    "unknown".to_string()
}

fn detect_project_types(repo_root: &str) -> Vec<String> {
    let mut types = Vec::new();

    if path_exists(repo_root, "src-tauri/tauri.conf.json") || path_exists(repo_root, "src-tauri/Cargo.toml") {
        types.push("tauri".to_string());
    }

    if path_exists(repo_root, "vite.config.ts") || path_exists(repo_root, "vite.config.js") {
        types.push("vite".to_string());
    }

    if path_exists(repo_root, "package.json") {
        types.push("node".to_string());
    }

    if path_exists(repo_root, "Cargo.toml") || path_exists(repo_root, "src-tauri/Cargo.toml") {
        types.push("rust".to_string());
    }

    if types.is_empty() {
        types.push("generic".to_string());
    }

    types
}

fn default_profile(repo_root: &str) -> Value {
    let project_types = detect_project_types(repo_root);
    let has_tauri = project_types.iter().any(|item| item == "tauri");
    let has_node = project_types.iter().any(|item| item == "node");
    let has_rust = project_types.iter().any(|item| item == "rust");

    let mut languages = Vec::new();
    if has_node {
        languages.push(serde_json::json!({
            "id": "typescript",
            "extensions": [".ts", ".tsx", ".vue"],
            "adapter": "typescript-vue"
        }));
    }
    if has_rust {
        languages.push(serde_json::json!({
            "id": "rust",
            "extensions": [".rs"],
            "adapter": "rust"
        }));
    }
    if languages.is_empty() {
        languages.push(serde_json::json!({
            "id": "generic",
            "extensions": [],
            "adapter": "generic"
        }));
    }

    let mut scopes = vec![
        serde_json::json!({ "name": "frontend", "patterns": ["src/**"] }),
        serde_json::json!({ "name": "config", "patterns": ["*.config.*", "tsconfig*.json", "package.json"] }),
    ];
    if has_tauri {
        scopes.push(serde_json::json!({ "name": "tauri-backend", "patterns": ["src-tauri/src/**"] }));
        scopes.push(serde_json::json!({ "name": "tauri-config", "patterns": ["src-tauri/tauri.conf.json", "src-tauri/capabilities/**"] }));
    }

    serde_json::json!({
        "version": 1,
        "project": {
            "name": project_name(repo_root),
            "type": project_types,
            "packageManager": detect_package_manager(repo_root)
        },
        "languages": languages,
        "scopes": scopes,
        "roles": {
            "primary": ["src/views/**", "src/components/**", "src/services/**", "src/stores/**", "src-tauri/src/commands/**", "src-tauri/src/git/**"],
            "secondary": ["src/i18n/**", "src/styles/**", "src/router/**", "src/types/**"],
            "generated": ["dist/**", "target/**", "node_modules/**", "package-lock.json", "pnpm-lock.yaml", "yarn.lock"],
            "tooling": ["vite.config.*", "tsconfig*.json", "src-tauri/Cargo.toml", "src-tauri/tauri.conf.json"]
        },
        "commit": {
            "defaultLanguage": "en",
            "style": "conventional",
            "scopeStrategy": "dominant-scope"
        },
        "review": {
            "localizationPatterns": [],
            "attentionWeights": {
                "source": 10,
                "config": 8,
                "style": 3,
                "lockfile": -5
            }
        }
    })
}

fn repo_root(repo_path: &str) -> Result<String, String> {
    run_git(repo_path, &["rev-parse", "--show-toplevel"])
}

fn ensure_local_exclude(repo_root: &str) -> Result<(), String> {
    let exclude_path = run_git(repo_root, &["rev-parse", "--git-path", "info/exclude"])?;
    let exclude_path = PathBuf::from(exclude_path.trim());
    let exclude_path = if exclude_path.is_absolute() {
        exclude_path
    } else {
        Path::new(repo_root).join(exclude_path)
    };
    let content = fs::read_to_string(&exclude_path).unwrap_or_default();
    if content.lines().any(|line| line.trim() == LOCAL_EXCLUDE_PATTERN) {
        return Ok(());
    }

    if let Some(parent) = exclude_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建 Git 本地忽略目录失败 {}: {}", parent.display(), e))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude_path)
        .map_err(|e| format!("打开 Git 本地忽略文件失败 {}: {}", exclude_path.display(), e))?;
    if !content.is_empty() && !content.ends_with('\n') {
        writeln!(file).map_err(|e| format!("更新 Git 本地忽略文件失败 {}: {}", exclude_path.display(), e))?;
    }
    writeln!(file, "{}", LOCAL_EXCLUDE_PATTERN)
        .map_err(|e| format!("更新 Git 本地忽略文件失败 {}: {}", exclude_path.display(), e))
}

fn read_profile(repo_root: &str, created: bool) -> Result<GitProjectProfileFile, String> {
    let path = profile_path(repo_root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取 Lumina 项目配置失败 {}: {}", path.display(), e))?;

    Ok(GitProjectProfileFile {
        repo_root: repo_root.to_string(),
        profile_path: path.to_string_lossy().to_string(),
        content,
        created,
    })
}

pub fn ensure_project_profile(repo_path: &str) -> Result<GitProjectProfileFile, String> {
    let root = repo_root(repo_path)?;
    ensure_local_exclude(&root)?;
    let path = profile_path(&root);

    if path.exists() {
        return read_profile(&root, false);
    }

    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析 Lumina 项目配置目录: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("创建 Lumina 项目配置目录失败 {}: {}", parent.display(), e))?;

    let content = serde_json::to_string_pretty(&default_profile(&root))
        .map_err(|e| format!("生成 Lumina 项目配置失败: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("写入 Lumina 项目配置失败 {}: {}", path.display(), e))?;

    read_profile(&root, true)
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn project_profile_is_ignored_in_local_git_exclude() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let repo = std::env::temp_dir().join(format!("lumina-profile-test-{unique}"));
        fs::create_dir_all(&repo).unwrap();
        assert!(Command::new("git").arg("init").arg(&repo).status().unwrap().success());

        ensure_project_profile(repo.to_str().unwrap()).unwrap();
        ensure_project_profile(repo.to_str().unwrap()).unwrap();

        let exclude = fs::read_to_string(repo.join(".git").join("info").join("exclude")).unwrap();
        assert_eq!(exclude.lines().filter(|line| line.trim() == LOCAL_EXCLUDE_PATTERN).count(), 1);
        let status = run_git(repo.to_str().unwrap(), &["status", "--porcelain"]).unwrap();
        assert!(status.is_empty());

        fs::remove_dir_all(repo).unwrap();
    }
}

pub fn load_project_profile(repo_path: &str) -> Result<GitProjectProfileFile, String> {
    let root = repo_root(repo_path)?;
    read_profile(&root, false)
}

pub fn save_project_profile(repo_path: &str, content: &str) -> Result<GitProjectProfileFile, String> {
    let root = repo_root(repo_path)?;
    let path = profile_path(&root);
    let parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("Lumina 项目配置不是合法 JSON: {}", e))?;
    let pretty = serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("格式化 Lumina 项目配置失败: {}", e))?;

    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析 Lumina 项目配置目录: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("创建 Lumina 项目配置目录失败 {}: {}", parent.display(), e))?;
    fs::write(&path, pretty)
        .map_err(|e| format!("保存 Lumina 项目配置失败 {}: {}", path.display(), e))?;

    read_profile(&root, false)
}
