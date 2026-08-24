use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiProviderType {
    OpenaiCompatible,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelConfig {
    pub id: String,
    pub name: String,
    pub provider: AiProviderType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub default_model_id: String,
    pub models: Vec<AiModelConfig>,
    pub task_model_map: HashMap<String, String>,
}

const AI_SETTINGS_FILE: &str = "ai-settings.json";

fn ai_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| format!("解析应用配置目录失败: {}", e))?
        .join(AI_SETTINGS_FILE))
}

use crate::storage::{
    history_repository::{load_ai_settings_from_db, save_ai_settings_to_db},
    AppDatabase,
};

#[tauri::command]
pub fn load_ai_settings(app: AppHandle) -> Result<Option<AiSettings>, String> {
    if let Some(db) = app.try_state::<AppDatabase>() {
        if let Ok(Some(settings)) = load_ai_settings_from_db(&db) {
            return Ok(Some(settings));
        }
    }

    let path = ai_settings_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取 AI 设置失败 {}: {}", path.display(), e))?;
    let settings = serde_json::from_str::<AiSettings>(&content)
        .map_err(|e| format!("AI 设置文件不是合法 JSON {}: {}", path.display(), e))?;

    if let Some(db) = app.try_state::<AppDatabase>() {
        let _ = save_ai_settings_to_db(&db, &settings);
        let _ = fs::remove_file(&path);
    }

    Ok(Some(settings))
}

#[tauri::command]
pub fn save_ai_settings(app: AppHandle, settings: AiSettings) -> Result<(), String> {
    if let Some(db) = app.try_state::<AppDatabase>() {
        save_ai_settings_to_db(&db, &settings)?;
    }

    // 移除旧明文 JSON 文件，统一收口至 SQLite 数据库
    if let Ok(path) = ai_settings_path(&app) {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}
