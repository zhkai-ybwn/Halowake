use std::{env, fs, path::Path};
use tauri::{AppHandle, Manager};

use crate::commands::ai_settings::AiSettings;
use crate::quota::models::{AccountConfig, ProviderType};

pub fn discover_local_accounts(app: &AppHandle) -> Vec<AccountConfig> {
    let mut accounts = Vec::new();

    // 1. 探测本地 ~/.codex
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    if !home.is_empty() {
        let codex_dir = Path::new(&home).join(".codex");
        if codex_dir.exists() {
            accounts.push(AccountConfig {
                id: "discovered-codex-local".to_string(),
                provider_type: ProviderType::Codex,
                name: "本地 Codex 默认账号".to_string(),
                api_key: None,
                base_url: None,
                enabled: true,
                auto_discovered: true,
            });
        }

        let gemini_paths = [
            Path::new(&home).join(".gemini"),
            Path::new(&home).join(".antigravity"),
            Path::new(&home).join("AppData").join("Roaming").join("Google").join("Antigravity"),
            Path::new(&home).join("AppData").join("Local").join("Google").join("Antigravity"),
            Path::new(&home).join(".config").join("antigravity"),
        ];

        let has_gemini_env = env::var("GEMINI_HOME").is_ok() || env::var("ANTIGRAVITY_HOME").is_ok();
        let has_gemini_path = gemini_paths.iter().any(|p| p.exists());

        // 只要存在常见目录、环境变量，或者作为默认支持项提供
        if has_gemini_path || has_gemini_env || true {
            accounts.push(AccountConfig {
                id: "discovered-gemini-antigravity".to_string(),
                provider_type: ProviderType::Gemini,
                name: "Google AI Pro (Antigravity / Gemini)".to_string(),
                api_key: None,
                base_url: None,
                enabled: true,
                auto_discovered: true,
            });
        }
    }

    // 2. 探测 Lumina 现有的 ai-settings.json
    if let Ok(config_dir) = app.path().app_config_dir() {
        let ai_settings_file = config_dir.join("ai-settings.json");
        if ai_settings_file.exists() {
            if let Ok(content) = fs::read_to_string(&ai_settings_file) {
                if let Ok(settings) = serde_json::from_str::<AiSettings>(&content) {
                    for model in settings.models {
                        if !model.enabled {
                            continue;
                        }
                        let base_url = model.base_url.to_lowercase();
                        let name_lower = model.name.to_lowercase();

                        if base_url.contains("deepseek") || name_lower.contains("deepseek") {
                            accounts.push(AccountConfig {
                                id: format!("discovered-deepseek-{}", model.id),
                                provider_type: ProviderType::Deepseek,
                                name: format!("DeepSeek ({})", model.name),
                                api_key: model.api_key.clone(),
                                base_url: Some(model.base_url.clone()),
                                enabled: true,
                                auto_discovered: true,
                            });
                        } else if base_url.contains("openrouter") || name_lower.contains("openrouter") {
                            accounts.push(AccountConfig {
                                id: format!("discovered-openrouter-{}", model.id),
                                provider_type: ProviderType::Openrouter,
                                name: format!("OpenRouter ({})", model.name),
                                api_key: model.api_key.clone(),
                                base_url: Some(model.base_url.clone()),
                                enabled: true,
                                auto_discovered: true,
                            });
                        }
                    }
                }
            }
        }
    }

    accounts
}
