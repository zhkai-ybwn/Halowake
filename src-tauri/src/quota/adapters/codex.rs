use std::{env, fs, path::Path, time::Duration};
use reqwest::Client;
use serde_json::Value;

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, QuotaKind, ProviderQuota, ProviderType};
use crate::quota::pace::calculate_pace;

pub async fn fetch_codex_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Codex,
        name: account.name.clone(),
        plan: Some("Plus / Pro".to_string()),
        quotas: Vec::new(),
        pace: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Codex.default_dashboard_url().map(String::from),
    };

    // 1. 获取 access_token 和 account_id
    let (access_token, account_id) = get_codex_auth_tokens(account);

    let token_str = match &access_token {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => {
            quota.error_message = Some("未在本地 ~/.codex/auth.json 找到有效登录凭证".to_string());
            return quota;
        }
    };

    // 2. 发起 HTTP GET 请求至 https://chatgpt.com/backend-api/wham/usage
    let mut client_builder = Client::builder().timeout(Duration::from_secs(12));

    // 检查是否有系统代理环境变量
    if let Ok(proxy_url) = env::var("HTTPS_PROXY").or_else(|_| env::var("https_proxy")).or_else(|_| env::var("HTTP_PROXY")).or_else(|_| env::var("http_proxy")).or_else(|_| env::var("ALL_PROXY")).or_else(|_| env::var("all_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            client_builder = client_builder.proxy(proxy);
        }
    }

    let client = match client_builder.build() {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    let mut request = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .header("Authorization", format!("Bearer {}", token_str))
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

    if let Some(acc_id) = &account_id {
        if !acc_id.is_empty() {
            request = request.header("chatgpt-account-id", acc_id);
        }
    }

    let response = match request.send().await {
        Ok(res) => res,
        Err(e) => {
            quota.error_message = Some(format!("连接 Codex 官方服务超时或失败: {}", e));
            return quota;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            quota.error_message = Some("Codex 登录 Token 已失效，请在本地重新登录 Codex".to_string());
        } else {
            quota.error_message = Some(format!("Codex API 返回错误 ({}): {}", status, error_body));
        }
        return quota;
    }

    let json_val = match response.json::<Value>().await {
        Ok(val) => val,
        Err(e) => {
            quota.error_message = Some(format!("解析响应数据失败: {}", e));
            return quota;
        }
    };

    // 3. 解析真实使用情况与计费数据
    // 3.1 账号与套餐
    if let Some(plan) = json_val.get("plan_type").and_then(Value::as_str) {
        quota.plan = Some(format!("{} 套餐", capitalize_first(plan)));
    }
    if let Some(email) = json_val.get("email").and_then(Value::as_str) {
        if account.auto_discovered || account.name.contains("Codex") {
            quota.name = format!("Codex ({})", email);
        }
    }

    // 3.2 点数余额 (credits: 25 credits = 1 USD)
    if let Some(credits) = json_val.get("credits") {
        let has_credits = credits.get("has_credits").and_then(Value::as_bool).unwrap_or(false);
        let balance_str = credits.get("balance").and_then(Value::as_str).unwrap_or("0");
        let credit_points = balance_str.parse::<f64>().unwrap_or(0.0);

        if has_credits || credit_points > 0.0 {
            let usd_val = credit_points / 25.0;
            quota.quotas.push(QuotaKind::Balance {
                currency: "USD".to_string(),
                topped_up: usd_val,
                granted: 0.0,
                total_remaining: (usd_val * 100.0).round() / 100.0,
            });
        }
    }

    // 3.3 限额与使用周期 (rate_limit)
    if let Some(rate_limit) = json_val.get("rate_limit") {
        let now_sec = chrono_now_ms() / 1000;

        if let Some(primary) = rate_limit.get("primary_window") {
            let used_percent = primary.get("used_percent").and_then(Value::as_f64).unwrap_or(0.0);
            let limit_window_seconds = primary.get("limit_window_seconds").and_then(Value::as_i64).unwrap_or(604800);
            let reset_after_seconds = primary.get("reset_after_seconds").and_then(Value::as_i64);
            let reset_at = primary.get("reset_at").and_then(Value::as_i64);

            let label = if limit_window_seconds <= 18000 {
                "5 小时限额".to_string()
            } else if limit_window_seconds <= 604800 {
                "每周限额".to_string()
            } else {
                "周期限额".to_string()
            };

            quota.quotas.push(QuotaKind::RateLimit {
                period_label: label,
                used_percent,
                resets_at: reset_at.or_else(|| reset_after_seconds.map(|s| now_sec + s)),
                resets_in_seconds: reset_after_seconds,
            });

            if let Some(rem_sec) = reset_after_seconds {
                quota.pace = Some(calculate_pace(used_percent, limit_window_seconds, rem_sec));
            }
        }

        if let Some(secondary) = rate_limit.get("secondary_window") {
            if !secondary.is_null() {
                let used_percent = secondary.get("used_percent").and_then(Value::as_f64).unwrap_or(0.0);
                let reset_after_seconds = secondary.get("reset_after_seconds").and_then(Value::as_i64);
                let reset_at = secondary.get("reset_at").and_then(Value::as_i64);

                quota.quotas.push(QuotaKind::RateLimit {
                    period_label: "短期使用限额".to_string(),
                    used_percent,
                    resets_at: reset_at.or_else(|| reset_after_seconds.map(|s| now_sec + s)),
                    resets_in_seconds: reset_after_seconds,
                });
            }
        }
    }

    quota.is_healthy = true;
    quota
}

fn get_codex_auth_tokens(account: &AccountConfig) -> (Option<String>, Option<String>) {
    // 优先使用账号配置中的 api_key (如果用户手动输入了 token)
    if let Some(k) = &account.api_key {
        if !k.trim().is_empty() {
            return (Some(k.clone()), None);
        }
    }

    // 默认从本地 ~/.codex/auth.json 读取
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    if home.is_empty() {
        return (None, None);
    }

    let auth_path = Path::new(&home).join(".codex").join("auth.json");
    if !auth_path.exists() {
        return (None, None);
    }

    if let Ok(content) = fs::read_to_string(auth_path) {
        if let Ok(val) = serde_json::from_str::<Value>(&content) {
            if let Some(tokens) = val.get("tokens") {
                let access_token = tokens.get("access_token").and_then(Value::as_str).map(String::from);
                let account_id = tokens.get("account_id").and_then(Value::as_str).map(String::from);
                return (access_token, account_id);
            }
        }
    }

    (None, None)
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
