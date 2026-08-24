use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path};

use super::{
    project_config::{read_project_config, save_project_config, validate_config},
    project_discovery::{discover_commands, has_python_project_markers, validate_project_directory},
    project_models::{LuminaProjectConfig, ProjectCommand, ProjectCommandCandidate},
    project_resolver::{resolve_package_project_commands, resolve_project_commands},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub project_path: String,
    pub package_json_path: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub package_manager: String,
    pub scripts: Vec<ProjectScript>,
    pub commands: Vec<ProjectCommand>,
    pub candidates: Vec<ProjectCommandCandidate>,
    pub detected_types: Vec<String>,
    pub config_state: String,
    pub config_error: Option<String>,
    pub default_command_id: Option<String>,
    pub dependencies_count: usize,
    pub dev_dependencies_count: usize,
    pub detected_stack: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScript {
    pub name: String,
    pub command: String,
}

#[tauri::command]
pub async fn load_project_manifest(project_path: String) -> Result<ProjectManifest, String> {
    tokio::task::spawn_blocking(move || read_project_manifest(&project_path))
        .await
        .map_err(|e| format!("加载项目配置任务异常: {}", e))?
}

#[tauri::command]
pub async fn load_project_config(project_path: String) -> Result<LuminaProjectConfig, String> {
    tokio::task::spawn_blocking(move || read_project_config(&project_path))
        .await
        .map_err(|error| format!("加载项目配置任务异常: {error}"))?
}

#[tauri::command]
pub async fn validate_project_config(config: LuminaProjectConfig) -> Result<(), String> {
    validate_config(&config)
}

#[tauri::command]
pub async fn save_project_config_command(project_path: String, config: LuminaProjectConfig) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_project_config(&project_path, &config))
        .await
        .map_err(|error| format!("保存项目配置任务异常: {error}"))?
}

#[tauri::command]
pub async fn discover_project_commands(project_path: String) -> Result<Vec<ProjectCommandCandidate>, String> {
    tokio::task::spawn_blocking(move || discover_commands(&project_path))
        .await
        .map_err(|error| format!("发现项目命令任务异常: {error}"))?
}

#[tauri::command]
pub async fn load_devdock_projects(
    database: tauri::State<'_, crate::storage::AppDatabase>,
) -> Result<Vec<crate::storage::history_repository::DevDockProjectRecord>, String> {
    crate::storage::history_repository::list_devdock_projects(&database)
}

#[tauri::command]
pub async fn save_devdock_project(
    database: tauri::State<'_, crate::storage::AppDatabase>,
    project: crate::storage::history_repository::DevDockProjectRecord,
) -> Result<(), String> {
    crate::storage::history_repository::save_devdock_project_record(&database, &project)
}

#[tauri::command]
pub async fn remove_devdock_project(
    database: tauri::State<'_, crate::storage::AppDatabase>,
    path: String,
) -> Result<(), String> {
    crate::storage::history_repository::remove_devdock_project_record(&database, &path)
}

#[tauri::command]
pub async fn load_devdock_run_history(
    database: tauri::State<'_, crate::storage::AppDatabase>,
    project_path: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<crate::storage::history_repository::DevDockRunHistoryRecord>, String> {
    let project_path = project_path.filter(|p| !p.trim().is_empty());
    let limit = limit.unwrap_or(50);
    crate::storage::history_repository::list_devdock_run_history_records(&database, project_path.as_deref(), limit)
}

#[tauri::command]
pub async fn clear_devdock_run_history(
    database: tauri::State<'_, crate::storage::AppDatabase>,
    project_path: Option<String>,
) -> Result<(), String> {
    let project_path = project_path.filter(|p| !p.trim().is_empty());
    crate::storage::history_repository::clear_devdock_run_history_records(&database, project_path.as_deref())
}

pub(crate) fn read_project_manifest(project_path: &str) -> Result<ProjectManifest, String> {
    let project_dir = Path::new(project_path);
    validate_project_directory(project_dir)?;

    let package_json_path = project_dir.join("package.json");
    let package_json = if package_json_path.is_file() {
        let content = fs::read_to_string(&package_json_path)
            .map_err(|e| format!("读取 package.json 失败: {}", e))?;
        Some(serde_json::from_str::<Value>(&content)
            .map_err(|e| format!("package.json 不是合法 JSON: {}", e))?)
    } else { None };
    let has_lumina_config = project_dir.join(".lumina").join("project.json").is_file();
    let (config, config_error) = match read_project_config(project_path) {
        Ok(config) => (config, None),
        Err(error) => (LuminaProjectConfig::default(), Some(error)),
    };
    let commands = if config_error.is_some() { resolve_package_project_commands(project_path)? } else { resolve_project_commands(project_path)? };

    let scripts = package_json.as_ref().map(|package| package
        .get("scripts")
        .and_then(Value::as_object)
        .map(|script_map| {
            let mut scripts = script_map
                .iter()
                .filter_map(|(name, command)| {
                    command.as_str().map(|command| ProjectScript {
                        name: name.to_string(),
                        command: command.to_string(),
                    })
                })
                .collect::<Vec<_>>();
            scripts.sort_by(|left, right| left.name.cmp(&right.name));
            scripts
        })
        .unwrap_or_default()).unwrap_or_default();

    let dependencies_count = package_json.as_ref().map(|package| object_len(package.get("dependencies"))).unwrap_or(0);
    let dev_dependencies_count = package_json.as_ref().map(|package| object_len(package.get("devDependencies"))).unwrap_or(0);
    let name = package_json.as_ref().and_then(|package| string_field(package, "name")).or(config.name.clone());
    let version = package_json.as_ref().and_then(|package| string_field(package, "version"));
    let package_manager = package_json.as_ref().map(|package| detect_package_manager(project_dir, package)).unwrap_or_else(|| "none".to_string());
    let candidates = discover_commands(project_path)?;
    let has_py_markers = has_python_project_markers(project_dir);

    // 有效项目判定：必须具备 package.json、已配置命令、候选命令、或 Python 项目特征之一
    if package_json.is_none() && !has_lumina_config && candidates.is_empty() && !has_py_markers && commands.is_empty() {
        return Err("NO_PROJECT_MANIFEST: 所选目录未检测到有效项目（需包含 package.json、Python 入口或启动脚本）。".to_string());
    }

    let mut detected_types = config.types.clone();
    if package_json.is_some() && !detected_types.iter().any(|item| item == "frontend") { detected_types.push("frontend".to_string()); }
    if (config.runtimes.python.is_some() || has_py_markers) && !detected_types.iter().any(|item| item == "python") { detected_types.push("python".to_string()); }
    if candidates.iter().any(|candidate| candidate.executor == "python" || candidate.executor == "python-module") && !detected_types.iter().any(|item| item == "python") { detected_types.push("python".to_string()); }

    Ok(ProjectManifest {
        project_path: project_path.to_string(),
        package_json_path: package_json_path.is_file().then(|| package_json_path.to_string_lossy().to_string()),
        name,
        version,
        scripts,
        commands,
        candidates,
        detected_types,
        config_state: if config_error.is_some() { "invalid".to_string() } else if config.commands.is_empty() { "default".to_string() } else { "configured".to_string() },
        config_error,
        default_command_id: config.defaults.command_id.as_ref().map(|id| {
            if id.starts_with("package:") { id.clone() } else { format!("config:{id}") }
        }),
        package_manager,
        dependencies_count,
        dev_dependencies_count,
        detected_stack: package_json.as_ref().map(detect_stack).unwrap_or_default(),
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn object_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_object).map(|object| object.len()).unwrap_or(0)
}

fn detect_package_manager(project_dir: &Path, package_json: &Value) -> String {
    if let Some(package_manager) = string_field(package_json, "packageManager") {
        return package_manager;
    }

    if project_dir.join("pnpm-lock.yaml").is_file() {
        return "pnpm".to_string();
    }
    if project_dir.join("yarn.lock").is_file() {
        return "yarn".to_string();
    }
    if project_dir.join("bun.lockb").is_file() || project_dir.join("bun.lock").is_file() {
        return "bun".to_string();
    }
    if project_dir.join("package-lock.json").is_file() {
        return "npm".to_string();
    }

    "npm".to_string()
}

fn detect_stack(package_json: &Value) -> Vec<String> {
    let mut stack = Vec::new();
    let has_dep = |name: &str| dependency_exists(package_json, name);

    for (dep, label) in [
        ("@tauri-apps/api", "Tauri"),
        ("vue", "Vue"),
        ("react", "React"),
        ("next", "Next.js"),
        ("vite", "Vite"),
        ("svelte", "Svelte"),
        ("typescript", "TypeScript"),
    ] {
        if has_dep(dep) {
            stack.push(label.to_string());
        }
    }

    stack
}

fn dependency_exists(package_json: &Value, name: &str) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .any(|section| package_json.get(section).and_then(Value::as_object).is_some_and(|deps| deps.contains_key(name)))
}
