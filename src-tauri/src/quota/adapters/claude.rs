use reqwest::Client;
use serde_json::Value;
use std::{env, fs, path::Path, time::Duration};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType};

pub async fn fetch_claude_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Claude,
        name: account.name.clone(),
        plan: Some("Claude Code Pro".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Claude
            .default_dashboard_url()
            .map(String::from),
    };

    // 1. 获取 Token: 优先使用配置中的 API Key，若无则尝试从本地 ~/.claude.json 或 ~/.claude/.credentials.json 读取
    let token = get_claude_auth_token(account);

    let token_str = match &token {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => {
            // 如果未提供任何凭证，但本地存在 .claude 目录，提供就绪状态提示
            let home = env::var("USERPROFILE")
                .or_else(|_| env::var("HOME"))
                .unwrap_or_default();
            let claude_dir = Path::new(&home).join(".claude");
            if claude_dir.exists() {
                quota.plan = Some("Claude Code CLI (Local)".to_string());
                quota.error_message =
                    Some("已检测到 Claude Code，但当前没有可可靠读取的本地额度数据".to_string());
                return quota;
            }

            quota.error_message =
                Some("未检测到有效 Claude Code Token 或 Anthropic API Key".to_string());
            return quota;
        }
    };

    // 2. 如果是 Anthropic API Key (形如 sk-ant-...)
    if token_str.starts_with("sk-ant-") {
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            Ok(c) => c,
            Err(e) => {
                quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
                return quota;
            }
        };

        // 尝试请求 models 校验可用性
        let res = client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", token_str)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await;

        match res {
            Ok(resp) => {
                if resp.status().is_success() {
                    quota.is_healthy = true;
                    quota.plan = Some("Anthropic API Tier".to_string());
                } else {
                    let status = resp.status();
                    quota.error_message = Some(format!("Anthropic API 认证失败 ({})", status));
                }
            }
            Err(e) => {
                quota.error_message = Some(format!("连接 Anthropic API 失败: {}", e));
            }
        }

        return quota;
    }

    // 3. OAuth Token 或 Claude CLI Session：没有稳定的公开额度接口，不能伪造用量。
    quota.plan = Some("Claude Code Pro / Max".to_string());
    quota.error_message = Some("已读取 Claude 登录态，但暂时无法可靠查询订阅额度".to_string());

    quota
}

fn get_claude_auth_token(account: &AccountConfig) -> Option<String> {
    if let Some(key) = &account.api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(env_token) =
        env::var("CLAUDE_CODE_OAUTH_TOKEN").or_else(|_| env::var("ANTHROPIC_API_KEY"))
    {
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

    // 检查 ~/.claude.json
    let claude_json_path = Path::new(&home).join(".claude.json");
    if claude_json_path.exists() {
        if let Ok(content) = fs::read_to_string(&claude_json_path) {
            if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                if let Some(token) = json_val.get("oauthToken").and_then(Value::as_str) {
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }
        }
    }

    // 检查 ~/.claude/.credentials.json
    let cred_path = Path::new(&home).join(".claude").join(".credentials.json");
    if cred_path.exists() {
        if let Ok(content) = fs::read_to_string(&cred_path) {
            if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                if let Some(token) = json_val
                    .get("accessToken")
                    .or_else(|| json_val.get("token"))
                    .and_then(Value::as_str)
                {
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }
        }
    }

    None
}
