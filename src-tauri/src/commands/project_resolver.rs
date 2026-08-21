use crate::commands::{project_config::read_project_config, project_models::*};
use serde_json::Value;
use std::{collections::hash_map::DefaultHasher, fs, hash::{Hash, Hasher}, path::Path};

pub fn resolve_project_commands(project_path: &str) -> Result<Vec<ProjectCommand>, String> {
    let root = Path::new(project_path);
    if !root.is_dir() { return Err("请选择有效的项目目录。".to_string()); }
    let config = read_project_config(project_path)?;
    let revision = project_revision(root);
    let mut commands = config.commands.iter().map(|command| resolved_config_command(command, &config, root, &revision)).collect::<Vec<_>>();
    if let Some(package) = read_package_commands(root)? {
        for mut command in package {
            if let Some(command_override) = config.command_overrides.get(&command.id) {
                if let Some(name) = &command_override.name { command.name = name.clone(); }
            }
            if !commands.iter().any(|item: &ProjectCommand| item.id == command.id) { commands.push(command); }
        }
    }
    commands.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(commands)
}

pub fn resolve_package_project_commands(project_path: &str) -> Result<Vec<ProjectCommand>, String> {
    let root = Path::new(project_path);
    let revision = project_revision(root);
    Ok(read_package_commands(root)?.unwrap_or_default().into_iter().map(|mut command| { command.config_revision = revision.clone(); command }).collect())
}

pub fn resolve_command(project_path: &str, command_id: &str) -> Result<ProjectCommandConfig, String> {
    let root = Path::new(project_path);
    let config = read_project_config(project_path)?;
    if let Some(command) = config.commands.iter().find(|command| format!("config:{}", command.id()) == command_id || command.id() == command_id) {
        return Ok(command.clone());
    }
    let script_name = command_id.strip_prefix("package:").ok_or_else(|| "COMMAND_NOT_FOUND: 未找到目标命令。".to_string())?;
    if !is_safe_package_script_name(script_name) {
        return Err("COMMAND_INVALID: package script 名称包含不支持的字符。".to_string());
    }
    let package = read_package_json(root)?.ok_or_else(|| "COMMAND_NOT_FOUND: 当前项目没有 package.json。".to_string())?;
    let script = package.get("scripts").and_then(Value::as_object).and_then(|scripts| scripts.get(script_name)).and_then(Value::as_str)
        .ok_or_else(|| "COMMAND_NOT_FOUND: package.json 中未找到目标脚本。".to_string())?;
    let command_override = config.command_overrides.get(&format!("package:{script_name}"));
    let common = ProjectCommandCommon { id: script_name.to_string(), name: command_override.and_then(|value| value.name.clone()).unwrap_or_else(|| script_name.to_string()), legacy_kind: None, args: Vec::new(), working_directory: None, environment: Default::default(), run_policy: Default::default() };
    Ok(ProjectCommandConfig::PackageScript { common, script: script.to_string() })
}

fn resolved_config_command(command: &ProjectCommandConfig, config: &LuminaProjectConfig, root: &Path, revision: &str) -> ProjectCommand {
    let common = command.common();
    let executor = executor_name(command);
    let preview = command_preview(root, config, command);
    let working_directory = common.working_directory.as_deref()
        .or(config.working_directory.as_deref())
        .unwrap_or(".");
    let mut environment_keys = config.environment.keys().chain(common.environment.keys()).cloned().collect::<Vec<_>>();
    environment_keys.sort();
    environment_keys.dedup();
    ProjectCommand { id: format!("config:{}", common.id), name: common.name.clone(), executor: executor.to_string(), source: "config".to_string(), source_label: ".lumina/project.json".to_string(), command_preview: preview, working_directory: root.join(working_directory).to_string_lossy().to_string(), run_policy: common.run_policy.clone(), config_revision: revision.to_string(), environment_keys }
}

fn read_package_commands(root: &Path) -> Result<Option<Vec<ProjectCommand>>, String> {
    let Some(package) = read_package_json(root)? else { return Ok(None); };
    let Some(scripts) = package.get("scripts").and_then(Value::as_object) else { return Ok(Some(Vec::new())); };
    let revision = project_revision(root);
    let package_manager = detect_package_manager(root);
    Ok(Some(scripts.iter().filter_map(|(name, value)| {
        if !is_safe_package_script_name(name) { return None; }
        value.as_str().map(|_script| ProjectCommand { id: format!("package:{name}"), name: name.clone(), executor: "package-script".to_string(), source: "package-json".to_string(), source_label: "package.json".to_string(), command_preview: format!("{package_manager} run {name}"), working_directory: root.to_string_lossy().to_string(), run_policy: ProjectRunPolicy::Singleton, config_revision: revision.clone(), environment_keys: Vec::new() })
    }).collect()))
}

pub fn executor_name(command: &ProjectCommandConfig) -> &'static str {
    match command {
        ProjectCommandConfig::PackageScript { .. } => "package-script",
        ProjectCommandConfig::Python { .. } => "python",
        ProjectCommandConfig::PythonModule { .. } => "python-module",
        ProjectCommandConfig::Cmd { .. } => "cmd",
        ProjectCommandConfig::Powershell { .. } => "powershell",
    }
}

pub fn command_preview(root: &Path, config: &LuminaProjectConfig, command: &ProjectCommandConfig) -> String {
    let common = command.common();
    let interpreter = config.runtimes.python.as_ref().map(|runtime| runtime.interpreter.as_str()).unwrap_or("python");
    let preview = match command {
        ProjectCommandConfig::PackageScript { .. } => format!("{} run {}", detect_package_manager(root), common.id),
        ProjectCommandConfig::Python { script, .. } => format!("{interpreter} {script}"),
        ProjectCommandConfig::PythonModule { module, .. } => format!("{interpreter} -m {module}"),
        ProjectCommandConfig::Cmd { script, .. } => format!("cmd.exe /D /S /C {script}"),
        ProjectCommandConfig::Powershell { script, .. } => format!("powershell.exe -NoProfile -File {script}"),
    };
    append_preview_args(preview, &common.args)
}

pub fn detect_package_manager(root: &Path) -> String {
    if let Ok(text) = fs::read_to_string(root.join("package.json")) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(configured) = value.get("packageManager").and_then(Value::as_str) {
                let name = configured.split('@').next().unwrap_or(configured);
                return match name { "pnpm" => "corepack pnpm", "yarn" => "corepack yarn", "bun" => "bun", _ => "npm" }.to_string();
            }
        }
    }
    if root.join("pnpm-lock.yaml").is_file() { "pnpm" }
    else if root.join("yarn.lock").is_file() { "yarn" }
    else if root.join("bun.lockb").is_file() || root.join("bun.lock").is_file() { "bun" }
    else { "npm" }.to_string()
}

pub fn is_safe_package_script_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-' | '.'))
}

fn append_preview_args(mut preview: String, args: &[String]) -> String {
    for arg in args {
        preview.push(' ');
        if arg.chars().any(char::is_whitespace) { preview.push_str(&format!("\"{}\"", arg.replace('"', "\\\""))); }
        else { preview.push_str(arg); }
    }
    preview
}

pub fn project_revision(root: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    for path in [root.join(".lumina").join("project.json"), root.join("package.json")] {
        path.to_string_lossy().hash(&mut hasher);
        if let Ok(content) = fs::read(&path) { content.hash(&mut hasher); }
    }
    format!("{:016x}", hasher.finish())
}

fn read_package_json(root: &Path) -> Result<Option<Value>, String> {
    let path = root.join("package.json");
    if !path.is_file() { return Ok(None); }
    let text = fs::read_to_string(&path).map_err(|error| format!("读取 package.json 失败: {error}"))?;
    serde_json::from_str(&text).map(Some).map_err(|error| format!("package.json 不是合法 JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_shell_metacharacters_in_package_script_names() {
        assert!(is_safe_package_script_name("dev:api"));
        assert!(!is_safe_package_script_name("safe & calc"));
        assert!(!is_safe_package_script_name("dev|whoami"));
    }

    #[test]
    fn preview_uses_runtime_args_and_project_working_directory() {
        let config: LuminaProjectConfig = serde_json::from_str(r#"{
            "schemaVersion": 1,
            "workingDirectory": "backend",
            "runtimes": { "python": { "interpreter": ".venv\\Scripts\\python.exe" } },
            "commands": [{
                "id": "api",
                "name": "API",
                "kind": "service",
                "executor": "python-module",
                "module": "uvicorn",
                "args": ["app:app", "--port", "8000"]
            }]
        }"#).expect("parse config");
        let command = &config.commands[0];
        assert_eq!(command_preview(Path::new("."), &config, command), ".venv\\Scripts\\python.exe -m uvicorn app:app --port 8000");
        let resolved = resolved_config_command(command, &config, Path::new("C:\\project"), "revision");
        assert!(resolved.working_directory.ends_with("backend"));
        assert_eq!(resolved.config_revision, "revision");
    }
}
