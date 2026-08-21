use crate::commands::project_models::{LuminaProjectConfig, ProjectCommandConfig};
use serde_json::to_string_pretty;
use std::{fs, path::{Path, PathBuf}};

pub const CONFIG_NOT_FOUND: &str = "CONFIG_NOT_FOUND";
pub const CONFIG_INVALID: &str = "CONFIG_INVALID";
pub const COMMAND_INVALID: &str = "COMMAND_INVALID";

pub fn config_path(project_path: &str) -> PathBuf {
    Path::new(project_path).join(".lumina").join("project.json")
}

pub fn read_project_config(project_path: &str) -> Result<LuminaProjectConfig, String> {
    let path = config_path(project_path);
    if !path.is_file() {
        return Ok(LuminaProjectConfig::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("{CONFIG_INVALID}: 读取 project.json 失败: {error}"))?;
    let mut config: LuminaProjectConfig = serde_json::from_str(&text)
        .map_err(|error| format!("{CONFIG_INVALID}: project.json 不是合法配置: {error}"))?;
    migrate_config(&mut config)?;
    validate_config(&config)?;
    Ok(config)
}

fn migrate_config(config: &mut LuminaProjectConfig) -> Result<(), String> {
    match config.schema_version {
        1 => {
            if config.defaults.command_id.is_none() {
                config.defaults.command_id = config.defaults.legacy_service_command_id.take();
            }
            config.schema_version = 2;
            Ok(())
        }
        2 => {
            if config.defaults.command_id.is_none() {
                config.defaults.command_id = config.defaults.legacy_service_command_id.take();
            } else {
                config.defaults.legacy_service_command_id = None;
            }
            Ok(())
        }
        version => Err(format!("{CONFIG_INVALID}: 不支持的 schemaVersion {version}")),
    }
}

pub fn validate_config(config: &LuminaProjectConfig) -> Result<(), String> {
    if config.schema_version != 2 {
        return Err(format!("{CONFIG_INVALID}: 不支持的 schemaVersion {}", config.schema_version));
    }
    let mut ids = std::collections::HashSet::new();
    for command in &config.commands {
        let id = command.id();
        if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')) {
            return Err(format!("{COMMAND_INVALID}: 非法 command id: {id}"));
        }
        if !ids.insert(id.to_string()) {
            return Err(format!("{COMMAND_INVALID}: command id 重复: {id}"));
        }
        if command.common().name.trim().is_empty() {
            return Err(format!("{COMMAND_INVALID}: command {id} 缺少名称"));
        }
        match command {
            crate::commands::project_models::ProjectCommandConfig::PythonModule { module, .. }
                if module.trim().is_empty() => return Err(format!("{COMMAND_INVALID}: command {id} 缺少 module")),
            _ => {}
        }
    }
    if let Some(default_id) = &config.defaults.command_id {
        let configured = config.commands.iter().any(|command| command.id() == default_id);
        let package_script = default_id.strip_prefix("package:").is_some_and(|name| {
            !name.is_empty() && name.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        });
        if !configured && !package_script {
            return Err(format!("{COMMAND_INVALID}: 默认 command 不存在: {default_id}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_config_does_not_require_runtime_revision() {
        let config: LuminaProjectConfig = serde_json::from_str(r#"{
            "schemaVersion": 2,
            "commands": [{
                "id": "api",
                "name": "API",
                "executor": "python-module",
                "module": "uvicorn",
                "args": ["app:app"]
            }]
        }"#).expect("parse project config without configRevision");

        validate_config(&config).expect("validate config");
        assert_eq!(config.commands[0].id(), "api");
    }

    #[test]
    fn v1_default_service_is_migrated_to_a_regular_default_command() {
        let mut config: LuminaProjectConfig = serde_json::from_str(r#"{
            "schemaVersion": 1,
            "commands": [{
                "id": "build",
                "name": "Build",
                "kind": "task",
                "executor": "powershell",
                "script": "build.ps1"
            }],
            "defaults": { "serviceCommandId": "build" }
        }"#).expect("parse project config");

        migrate_config(&mut config).expect("migrate v1 config");
        validate_config(&config).expect("allow any command as default");
        assert_eq!(config.schema_version, 2);
        assert_eq!(config.defaults.command_id.as_deref(), Some("build"));
        assert!(config.defaults.legacy_service_command_id.is_none());
    }
}

pub fn save_project_config(project_path: &str, config: &LuminaProjectConfig) -> Result<(), String> {
    let mut config = config.clone();
    migrate_config(&mut config)?;
    validate_config(&config)?;
    validate_config_paths(project_path, &config)?;
    let directory = Path::new(project_path).join(".lumina");
    fs::create_dir_all(&directory).map_err(|error| format!("保存配置失败: {error}"))?;
    let path = directory.join("project.json");
    let temporary = directory.join("project.json.tmp");
    fs::write(&temporary, format!("{}\n", to_string_pretty(&config).map_err(|error| format!("保存配置失败: {error}"))?))
        .map_err(|error| format!("保存配置失败: {error}"))?;
    if !path.exists() {
        return fs::rename(&temporary, &path).map_err(|error| format!("保存配置失败: {error}"));
    }
    let backup = directory.join("project.json.bak");
    if backup.exists() { fs::remove_file(&backup).map_err(|error| format!("保存配置失败: {error}"))?; }
    fs::rename(&path, &backup).map_err(|error| format!("保存配置失败: {error}"))?;
    match fs::rename(&temporary, &path) {
        Ok(()) => { let _ = fs::remove_file(backup); Ok(()) }
        Err(error) => {
            let _ = fs::rename(&backup, &path);
            Err(format!("保存配置失败，已恢复原配置: {error}"))
        }
    }
}

fn validate_config_paths(project_path: &str, config: &LuminaProjectConfig) -> Result<(), String> {
    let root = Path::new(project_path).canonicalize().map_err(|error| format!("WORKING_DIRECTORY_INVALID: {error}"))?;
    validate_inside(&root, config.working_directory.as_deref().unwrap_or("."), true)?;
    for command in &config.commands {
        validate_inside(&root, command.common().working_directory.as_deref().unwrap_or(config.working_directory.as_deref().unwrap_or(".")), true)?;
        match command {
            ProjectCommandConfig::Python { script, .. }
            | ProjectCommandConfig::Cmd { script, .. }
            | ProjectCommandConfig::Powershell { script, .. } => validate_inside(&root, script, false)?,
            ProjectCommandConfig::PythonModule { .. } | ProjectCommandConfig::PackageScript { .. } => {}
        }
    }
    if let Some(runtime) = &config.runtimes.python {
        let value = Path::new(&runtime.interpreter);
        if value.is_absolute() && !value.is_file() {
            return Err("EXECUTOR_UNAVAILABLE: Python 解释器不存在。".to_string());
        }
        if !value.is_absolute() && (runtime.interpreter.contains('/') || runtime.interpreter.contains('\\')) && !root.join(value).is_file() {
            return Err("EXECUTOR_UNAVAILABLE: Python 解释器不存在。".to_string());
        }
    }
    Ok(())
}

fn validate_inside(root: &Path, value: &str, directory: bool) -> Result<(), String> {
    let resolved = root.join(value).canonicalize().map_err(|error| format!("COMMAND_INVALID: 路径 {value} 无效: {error}"))?;
    if !resolved.starts_with(root) { return Err("SCRIPT_OUTSIDE_PROJECT: 路径必须位于项目目录内。".to_string()); }
    if directory && !resolved.is_dir() { return Err("WORKING_DIRECTORY_INVALID: 工作目录不存在。".to_string()); }
    if !directory && !resolved.is_file() { return Err("COMMAND_INVALID: 脚本文件不存在。".to_string()); }
    Ok(())
}
