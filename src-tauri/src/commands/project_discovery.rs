use crate::commands::project_models::ProjectCommandCandidate;
use serde_json::json;
use std::{fs, path::Path};

pub const IGNORED_DIRECTORIES: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
    "out",
    "target",
    ".output",
    ".next",
    ".nuxt",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    ".pytest_cache",
    ".idea",
    ".vscode",
    "vendor",
    "bin",
    "obj",
];

pub fn validate_project_directory(root: &Path) -> Result<(), String> {
    if !root.is_dir() {
        return Err("请选择有效的项目目录。".to_string());
    }

    // 校验是否为驱动器根目录（例如 C:\、D:\ 或 /）
    let normal_components = root
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();

    if normal_components.is_empty() {
        return Err("禁止选择磁盘根目录作为项目。".to_string());
    }

    // 检查 Windows 系统目录与敏感根目录
    let lower_components: Vec<String> = normal_components
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();

    if lower_components.len() == 1 {
        let first = lower_components[0].as_str();
        if matches!(
            first,
            "windows"
                | "program files"
                | "program files (x86)"
                | "programdata"
                | "system volume information"
                | "$recycle.bin"
                | "recovery"
                | "users"
        ) {
            return Err("禁止选择系统目录作为项目。".to_string());
        }
    } else if lower_components.len() == 2 && lower_components[0] == "users" {
        // C:\Users\<username> 用户主目录
        return Err("禁止直接选择用户主目录，请选择具体的项目子目录。".to_string());
    }

    Ok(())
}

pub fn has_python_project_markers(root: &Path) -> bool {
    for marker in [
        "pyproject.toml",
        "requirements.txt",
        "Pipfile",
        "poetry.lock",
        "uv.lock",
        "setup.py",
        "setup.cfg",
    ] {
        if root.join(marker).is_file() {
            return true;
        }
    }
    false
}

pub fn discover_commands(project_path: &str) -> Result<Vec<ProjectCommandCandidate>, String> {
    let root = Path::new(project_path);
    validate_project_directory(root)?;
    let mut candidates = Vec::new();
    let interpreter = detected_interpreter(root);
    for file in ["manage.py", "main.py", "app.py", "server.py", "run.py"] {
        if root.join(file).is_file() {
            let (name, args, confidence, reason) = if file == "manage.py" {
                ("Django 开发服务", vec!["runserver"], "high", "检测到 manage.py")
            } else {
                ("Python 服务", Vec::new(), "medium", "检测到常见 Python 入口文件")
            };
            candidates.push(ProjectCommandCandidate {
                suggested_id: file.trim_end_matches(".py").replace('_', "-").to_string(),
                name: format!("{} ({})", name, file),
                executor: "python".to_string(),
                confidence: confidence.to_string(),
                reason: reason.to_string(),
                source: file.to_string(),
                draft: json!({ "executor": "python", "script": file, "args": args, "interpreter": interpreter.to_string() }),
            });
        }
    }
    append_pyproject_candidates(root, &interpreter, &mut candidates);
    for directory in [root.to_path_buf(), root.join("scripts")] {
        let Ok(entries) = fs::read_dir(&directory) else { continue; };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { continue; }
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if IGNORED_DIRECTORIES.contains(&file_name) {
                    continue;
                }
            }
            let Some(extension) = path.extension().and_then(|value| value.to_str()).map(str::to_ascii_lowercase) else { continue; };
            let executor = match extension.as_str() { "cmd" | "bat" => "cmd", "ps1" => "powershell", _ => continue };
            let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
            let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("script");
            candidates.push(ProjectCommandCandidate {
                suggested_id: stem.replace('_', "-").to_ascii_lowercase(),
                name: stem.to_string(),
                executor: executor.to_string(),
                confidence: "medium".to_string(),
                reason: format!("检测到 {} 脚本", extension.to_ascii_uppercase()),
                source: relative.clone(),
                draft: json!({ "executor": executor, "script": relative, "args": [] }),
            });
        }
    }
    candidates.sort_by(|left, right| left.source.cmp(&right.source));
    candidates.dedup_by(|left, right| left.executor == right.executor && left.source == right.source);
    Ok(candidates)
}

fn detected_interpreter(root: &Path) -> String {
    for relative in [".venv/Scripts/python.exe", "venv/Scripts/python.exe"] {
        if root.join(relative).is_file() { return relative.replace('/', "\\"); }
    }
    "python".to_string()
}

fn append_pyproject_candidates(root: &Path, interpreter: &str, candidates: &mut Vec<ProjectCommandCandidate>) {
    let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) else { return; };
    let mut in_scripts = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_scripts = matches!(trimmed, "[project.scripts]" | "[tool.poetry.scripts]");
            continue;
        }
        if !in_scripts || trimmed.starts_with('#') || trimmed.is_empty() { continue; }
        let Some((name, target)) = trimmed.split_once('=') else { continue; };
        let name = name.trim().trim_matches(|ch| ch == '"' || ch == '\'');
        let target = target.trim().trim_matches(|ch| ch == '"' || ch == '\'');
        let Some(module) = pyproject_script_module(target) else { continue; };
        if name.is_empty() || !module.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) { continue; }
        candidates.push(ProjectCommandCandidate {
            suggested_id: name.replace('_', "-"),
            name: name.to_string(),
            executor: "python-module".to_string(),
            confidence: "high".to_string(),
            reason: "检测到可作为 Python 模块运行的 pyproject.toml script".to_string(),
            source: "pyproject.toml".to_string(),
            draft: json!({ "executor": "python-module", "module": module, "args": [], "interpreter": interpreter.to_string() }),
        });
    }
}

fn pyproject_script_module(target: &str) -> Option<&str> {
    let target = target.trim();
    if target.contains(':') { return None; }
    (!target.is_empty() && target.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))).then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_misrepresent_console_entry_points_as_python_modules() {
        assert_eq!(pyproject_script_module("pkg.cli:main"), None);
        assert_eq!(pyproject_script_module("pkg.cli"), Some("pkg.cli"));
    }

    #[test]
    fn rejects_drive_roots_and_system_directories() {
        assert!(validate_project_directory(Path::new("C:\\")).is_err());
        assert!(validate_project_directory(Path::new("D:\\")).is_err());
        assert!(validate_project_directory(Path::new("C:\\Windows")).is_err());
        assert!(validate_project_directory(Path::new("C:\\Program Files")).is_err());
        assert!(validate_project_directory(Path::new("C:\\Users")).is_err());
        assert!(validate_project_directory(Path::new("C:\\Users\\alice")).is_err());
    }
}
