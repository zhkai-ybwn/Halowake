use crate::commands::{project_config::read_project_config, project_models::ProjectCommandConfig, project_resolver::{command_preview, detect_package_manager, project_revision, resolve_command}};
use std::{collections::HashMap, ffi::OsString, path::{Path, PathBuf}, process::Command};

pub struct ResolvedCommand {
    pub command_id: String,
    pub command_name: String,
    pub executor: String,
    pub program: OsString,
    pub args: Vec<OsString>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub command_preview: String,
    pub config_revision: String,
}

pub fn resolve_executable(project_path: &str, command_id: &str) -> Result<ResolvedCommand, String> {
    let root = Path::new(project_path).canonicalize().map_err(|error| format!("WORKING_DIRECTORY_INVALID: {error}"))?;
    let config = read_project_config(project_path)?;
    let command = resolve_command(project_path, command_id)?;
    let common = command.common();
    let working = resolve_inside(&root, common.working_directory.as_deref().or(config.working_directory.as_deref()).unwrap_or("."), "WORKING_DIRECTORY_INVALID")?;
    if !working.is_dir() { return Err("WORKING_DIRECTORY_INVALID: 工作目录不存在。".to_string()); }
    let working = process_compatible_path(working);
    let mut environment = config.environment.clone();
    environment.extend(common.environment.clone());
    let mut args = Vec::<OsString>::new();
    let (executor, program, preview) = match &command {
        ProjectCommandConfig::PackageScript { .. } => {
            if !common.args.is_empty() {
                return Err("COMMAND_INVALID: 第一版 package-script 暂不支持附加 args。".to_string());
            }
            let package_manager = detect_package_manager(&root);
            let package_preview = command_preview(&root, &config, &command);
            #[cfg(windows)]
            {
                let invocation = format!("{} run {}", package_manager, common.id);
                args.extend([OsString::from("/D"), OsString::from("/S"), OsString::from("/C"), OsString::from(invocation)]);
                ("package-script", OsString::from("cmd.exe"), package_preview)
            }
            #[cfg(not(windows))]
            {
                let mut parts = package_manager.split_whitespace();
                let program = parts.next().unwrap_or("npm");
                args.extend(parts.map(OsString::from));
                args.extend(["run", common.id.as_str()].into_iter().map(OsString::from));
                ("package-script", OsString::from(program), package_preview)
            }
        }
        ProjectCommandConfig::Python { script, .. } => {
            let script_path = process_compatible_path(resolve_file_inside(&root, script)?);
            args.push(script_path.into_os_string());
            args.extend(common.args.iter().map(OsString::from));
            let interpreter = python_interpreter(&root, config.runtimes.python.as_ref().map(|runtime| runtime.interpreter.as_str()))?;
            ("python", interpreter.clone(), format!("{} {}", interpreter.to_string_lossy(), script))
        }
        ProjectCommandConfig::PythonModule { module, .. } => {
            if !module.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) { return Err("COMMAND_INVALID: Python module 名称非法。".to_string()); }
            args.extend([OsString::from("-m"), OsString::from(module)]);
            args.extend(common.args.iter().map(OsString::from));
            let interpreter = python_interpreter(&root, config.runtimes.python.as_ref().map(|runtime| runtime.interpreter.as_str()))?;
            ("python-module", interpreter.clone(), format!("{} -m {}", interpreter.to_string_lossy(), module))
        }
        ProjectCommandConfig::Cmd { script, .. } => {
            let script_path = process_compatible_path(resolve_file_inside(&root, script)?);
            let extension = script_path.extension().and_then(|value| value.to_str()).unwrap_or("");
            if !matches!(extension.to_ascii_lowercase().as_str(), "cmd" | "bat") { return Err("COMMAND_INVALID: cmd executor 只支持 .cmd 或 .bat。".to_string()); }
            args.extend([OsString::from("/D"), OsString::from("/S"), OsString::from("/C"), script_path.as_os_str().to_owned()]);
            args.extend(common.args.iter().map(OsString::from));
            ("cmd", OsString::from("cmd.exe"), script_path.to_string_lossy().to_string())
        }
        ProjectCommandConfig::Powershell { script, .. } => {
            let script_path = process_compatible_path(resolve_file_inside(&root, script)?);
            if !script_path.extension().and_then(|value| value.to_str()).is_some_and(|value| value.eq_ignore_ascii_case("ps1")) { return Err("COMMAND_INVALID: powershell executor 只支持 .ps1。".to_string()); }
            args.extend([OsString::from("-NoProfile"), OsString::from("-File"), script_path.as_os_str().to_owned()]);
            args.extend(common.args.iter().map(OsString::from));
            ("powershell", OsString::from("powershell.exe"), script_path.to_string_lossy().to_string())
        }
    };
    let preview = if matches!(command, ProjectCommandConfig::PackageScript { .. }) { preview } else { command_preview(&root, &config, &command) };
    Ok(ResolvedCommand { command_id: if command_id.contains(':') { command_id.to_string() } else { format!("config:{command_id}") }, command_name: common.name.clone(), executor: executor.to_string(), program, args, working_directory: working, environment, command_preview: preview, config_revision: project_revision(&root) })
}

pub fn build_process_command(resolved: &ResolvedCommand) -> Command {
    let mut command = Command::new(&resolved.program);
    command.args(&resolved.args).current_dir(&resolved.working_directory).envs(&resolved.environment);
    command
}

fn resolve_inside(root: &Path, value: &str, code: &str) -> Result<PathBuf, String> {
    let path = root.join(value).canonicalize().map_err(|error| format!("{code}: {error}"))?;
    if !path.starts_with(root) { return Err(format!("{code}: 路径必须位于项目目录内。")); }
    Ok(path)
}

fn resolve_file_inside(root: &Path, value: &str) -> Result<PathBuf, String> {
    let path = resolve_inside(root, value, "SCRIPT_OUTSIDE_PROJECT")?;
    if !path.is_file() { return Err("COMMAND_INVALID: 脚本文件不存在。".to_string()); }
    Ok(path)
}

fn python_interpreter(root: &Path, configured: Option<&str>) -> Result<OsString, String> {
    let value = configured.unwrap_or("python");
    let path = Path::new(value);
    if path.is_absolute() {
        if !path.is_file() { return Err("EXECUTOR_UNAVAILABLE: Python 解释器不存在。".to_string()); }
        return Ok(process_compatible_path(path.to_path_buf()).into_os_string());
    }
    if value.contains('/') || value.contains('\\') {
        let resolved = root.join(path);
        if !resolved.is_file() { return Err("EXECUTOR_UNAVAILABLE: Python 解释器不存在。".to_string()); }
        return Ok(process_compatible_path(resolved).into_os_string());
    }
    Ok(OsString::from(value))
}

fn process_compatible_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn removes_windows_verbatim_prefix_before_spawning_cmd() {
        assert_eq!(
            process_compatible_path(PathBuf::from(r"\\?\D:\ly_project\ami-insight")),
            PathBuf::from(r"D:\ly_project\ami-insight")
        );
    }
}
