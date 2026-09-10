use std::{env, fs, path::Path};
use tauri::{AppHandle, Manager};

use crate::commands::ai_settings::AiSettings;
use crate::quota::models::{AccountConfig, ProviderType};

pub fn discover_local_accounts(app: &AppHandle) -> Vec<AccountConfig> {
    let mut accounts = Vec::new();

    // 1. 探测本地 CLI 客户端
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    if !home.is_empty() {
        let home_path = Path::new(&home);

        // Codex (~/.codex)
        let codex_dir = home_path.join(".codex");
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

        // Claude Code (~/.claude 或 ~/.claude.json)
        let claude_dir = home_path.join(".claude");
        let claude_json = home_path.join(".claude.json");
        let has_claude_token = claude_json.exists()
            || claude_dir.join(".credentials.json").exists()
            || claude_dir.join("settings.json").exists();
        if has_claude_token {
            accounts.push(AccountConfig {
                id: "discovered-claude-local".to_string(),
                provider_type: ProviderType::Claude,
                name: "Claude Code 本地默认账号".to_string(),
                api_key: None,
                base_url: None,
                enabled: true,
                auto_discovered: true,
            });
        }

        // WorkBuddy (~/.workbuddy)
        let workbuddy_dir = home_path.join(".workbuddy");
        if workbuddy_dir.exists() {
            accounts.push(AccountConfig {
                id: "discovered-workbuddy-local".to_string(),
                provider_type: ProviderType::Workbuddy,
                name: "WorkBuddy 智能体工作台".to_string(),
                api_key: None,
                base_url: None,
                enabled: true,
                auto_discovered: true,
            });
        }

        // OpenCode (~/.opencode)
        let opencode_dir = home_path.join(".opencode");
        if opencode_dir.exists() {
            accounts.push(AccountConfig {
                id: "discovered-opencode-local".to_string(),
                provider_type: ProviderType::Opencode,
                name: "OpenCode 本地账号".to_string(),
                api_key: None,
                base_url: None,
                enabled: true,
                auto_discovered: true,
            });
        }

        // Gemini / Antigravity 作为默认支持项提供
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

    // 2. 探测 Halowake 现有的 ai-settings.json
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
                        } else if base_url.contains("siliconflow") || name_lower.contains("siliconflow") || name_lower.contains("硅基流动") {
                            accounts.push(AccountConfig {
                                id: format!("discovered-siliconflow-{}", model.id),
                                provider_type: ProviderType::Siliconflow,
                                name: format!("硅基流动 ({})", model.name),
                                api_key: model.api_key.clone(),
                                base_url: Some(model.base_url.clone()),
                                enabled: true,
                                auto_discovered: true,
                            });
                        } else if base_url.contains("moonshot") || name_lower.contains("moonshot") || name_lower.contains("kimi") {
                            accounts.push(AccountConfig {
                                id: format!("discovered-moonshot-{}", model.id),
                                provider_type: ProviderType::Moonshot,
                                name: format!("Moonshot Kimi ({})", model.name),
                                api_key: model.api_key.clone(),
                                base_url: Some(model.base_url.clone()),
                                enabled: true,
                                auto_discovered: true,
                            });
                        } else if base_url.contains("bigmodel") || base_url.contains("zhipu") || name_lower.contains("智谱") || name_lower.contains("glm") {
                            accounts.push(AccountConfig {
                                id: format!("discovered-zhipu-{}", model.id),
                                provider_type: ProviderType::Zhipu,
                                name: format!("智谱 GLM ({})", model.name),
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
