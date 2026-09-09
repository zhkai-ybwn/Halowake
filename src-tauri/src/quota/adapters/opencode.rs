use reqwest::Client;
use serde_json::Value;
use std::{env, fs, path::Path, time::Duration};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType};

pub async fn fetch_opencode_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Opencode,
        name: account.name.clone(),
        plan: Some("OpenCode Go".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Opencode
            .default_dashboard_url()
            .map(String::from),
    };

    let token = get_opencode_token(account);

    let token_str = match &token {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => {
            // 本地探测兜底
            let home = env::var("USERPROFILE")
                .or_else(|_| env::var("HOME"))
                .unwrap_or_default();
            let opencode_dir = Path::new(&home).join(".opencode");
            if opencode_dir.exists() {
                quota.plan = Some("OpenCode Local CLI".to_string());
                quota.error_message =
                    Some("已检测到 OpenCode，但当前没有可可靠读取的本地积分数据".to_string());
                return quota;
            }

            quota.error_message =
                Some("未配置 OpenCode API Key，且未在本地发现 ~/.opencode 客户端".to_string());
            return quota;
        }
    };

    let base_url = account
        .base_url
        .as_deref()
        .unwrap_or("https://opencode.ai/zen/v1")
        .trim_end_matches('/');

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    // 尝试请求 models 校验
    let models_url = format!("{}/models", base_url);
    let res = client
        .get(&models_url)
        .header("Authorization", format!("Bearer {}", token_str))
        .header("Accept", "application/json")
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                quota.is_healthy = true;
                let val = resp.json::<Value>().await.unwrap_or_default();
                let model_count = val
                    .get("data")
                    .and_then(Value::as_array)
                    .map(|models| models.len());
                quota.plan = Some(match model_count {
                    Some(count) => format!("Zen Gateway · {} 个可用模型", count),
                    None => "Zen Gateway".to_string(),
                });
            } else {
                let status = resp.status();
                quota.error_message = Some(format!("OpenCode API 验证失败 ({})", status));
            }
        }
        Err(e) => {
            quota.error_message = Some(format!("连接 OpenCode 失败: {}", e));
        }
    }

    quota
}

fn get_opencode_token(account: &AccountConfig) -> Option<String> {
    if let Some(key) = &account.api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(env_token) = env::var("OPENCODE_API_KEY") {
        if !env_token.trim().is_empty() {
            return Some(env_token.trim().to_string());
        }
    }

    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    if home.is_empty() {
        return None;
    }

    let config_path = Path::new(&home).join(".opencode").join("config.json");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                if let Some(key) = json_val
                    .get("apiKey")
                    .or_else(|| json_val.get("zenKey"))
                    .and_then(Value::as_str)
                {
                    if !key.is_empty() {
                        return Some(key.to_string());
                    }
                }
            }
        }
    }

    None
}
