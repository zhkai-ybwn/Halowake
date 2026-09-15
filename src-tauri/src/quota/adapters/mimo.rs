use reqwest::Client;
use serde_json::Value;
use std::{env, fs, path::PathBuf, time::Duration};

use crate::quota::adapters::deepseek::{chrono_now_ms, parse_amount};
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType, QuotaKind};

const DEFAULT_BASE_URL: &str = "https://api.xiaomimimo.com/v1";

pub fn has_mimo_credentials() -> bool {
    load_local_mimo_auth().is_some()
}

pub fn load_local_mimo_auth() -> Option<(String, Option<String>)> {
    for path in candidate_auth_paths() {
        if !path.exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json_val) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        // auth.json: { "xiaomi": { "type": "api", "key": "sk-...", "metadata": { "base_url": "..." } } }
        if let Some(xiaomi) = json_val.get("xiaomi") {
            if let Some(key) = xiaomi.get("key").and_then(Value::as_str) {
                let key = key.trim();
                if !key.is_empty() {
                    let base_url = xiaomi
                        .pointer("/metadata/base_url")
                        .and_then(Value::as_str)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                    return Some((key.to_string(), base_url));
                }
            }
        }
    }
    None
}

fn candidate_auth_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
        let home = PathBuf::from(home);
        // Linux / macOS XDG
        paths.push(home.join(".local/share/mimocode/auth.json"));
        paths.push(home.join(".config/mimocode/auth.json"));
        // Windows %LOCALAPPDATA% under home
        paths.push(home.join("AppData/Local/mimocode/auth.json"));
    }

    if let Ok(local_app) = env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(local_app).join("mimocode/auth.json"));
    }

    // MiMo Desktop 引擎数据目录（Windows Roaming）
    if let Ok(app_data) = env::var("APPDATA") {
        paths.push(PathBuf::from(app_data).join("Xiaomi MiMo/auth.json"));
    }

    paths
}

fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else if trimmed.is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        format!("{}/v1", trimmed)
    }
}

fn resolve_api_key(account: &AccountConfig) -> Option<String> {
    if let Some(key) = &account.api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(env_key) = env::var("MIMO_API_KEY") {
        let trimmed = env_key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    load_local_mimo_auth().map(|(key, _)| key)
}

fn resolve_base_url(account: &AccountConfig) -> String {
    if let Some(base) = &account.base_url {
        let trimmed = base.trim();
        if !trimmed.is_empty() {
            return normalize_base_url(trimmed);
        }
    }
    if let Some((_, Some(base))) = load_local_mimo_auth() {
        return normalize_base_url(&base);
    }
    DEFAULT_BASE_URL.to_string()
}

pub async fn fetch_mimo_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Mimo,
        name: account.name.clone(),
        plan: Some("Xiaomi MiMo 开放平台".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Mimo.default_dashboard_url().map(String::from),
    };

    let api_key = match resolve_api_key(account) {
        Some(k) => k,
        None => {
            quota.error_message =
                Some("未配置小米 MiMo API Key，也未在本地 ~/.local/share/mimocode/auth.json 检测到登录凭据".to_string());
            return quota;
        }
    };

    let base_url = resolve_base_url(account);

    let client = match Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    // 1. 连通性校验：GET /v1/models（已验证可用）
    let models_url = format!("{}/models", base_url);
    let models_res = client
        .get(&models_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await;

    let mut model_count: Option<usize> = None;
    match models_res {
        Ok(res) if res.status().is_success() => {
            if let Ok(json_val) = res.json::<Value>().await {
                model_count = json_val
                    .get("data")
                    .and_then(Value::as_array)
                    .map(|models| models.len());
            }
            quota.is_healthy = true;
        }
        Ok(res) => {
            let status = res.status();
            if status.as_u16() == 401 || status.as_u16() == 403 {
                quota.error_message =
                    Some("MiMo API Key 无效或已过期，请在开放平台重新生成".to_string());
            } else {
                quota.error_message = Some(format!("MiMo API 返回错误 ({})", status));
            }
            return quota;
        }
        Err(e) => {
            quota.error_message = Some(format!("连接小米 MiMo 服务失败: {}", e));
            return quota;
        }
    }

    // 2. 尝试常见余额端点（开放平台当前多为 404，有则解析）
    let balance_paths = [
        "/user/balance",
        "/users/me/balance",
        "/account/balance",
        "/billing/balance",
        "/user/info",
    ];
    for path in balance_paths {
        let url = format!("{}{}", base_url, path);
        if let Ok(res) = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/json")
            .send()
            .await
        {
            if !res.status().is_success() {
                continue;
            }
            if let Ok(json_val) = res.json::<Value>().await {
                if let Some(parsed) = parse_balance_payload(&json_val) {
                    quota.quotas.push(parsed);
                    quota.plan = Some("Xiaomi MiMo 开放平台 · 按量计费".to_string());
                    quota.is_healthy = true;
                    return quota;
                }
            }
        }
    }

    // 3. 无余额接口时：仅展示连通性 + 可用模型数，不编造数字
    quota.is_healthy = true;
    quota.plan = Some(match model_count {
        Some(count) => format!("Xiaomi MiMo · {} 个可用模型 · 额度见控制台", count),
        None => "Xiaomi MiMo 开放平台 · 额度见控制台".to_string(),
    });

    quota
}

/// 尝试从多种常见响应结构中解析余额。
fn parse_balance_payload(json_val: &Value) -> Option<QuotaKind> {
    let data_node = json_val.get("data").unwrap_or(json_val);

    // DeepSeek 风格
    if let Some(balance_infos) = json_val.get("balance_infos").and_then(Value::as_array) {
        if let Some(info) = balance_infos.first() {
            let currency = info
                .get("currency")
                .and_then(Value::as_str)
                .unwrap_or("CNY")
                .to_string();
            return Some(QuotaKind::Balance {
                currency,
                topped_up: parse_amount(info.get("topped_up_balance")),
                granted: parse_amount(info.get("granted_balance")),
                total_remaining: parse_amount(info.get("total_balance")),
            });
        }
    }

    // SiliconFlow 风格
    if data_node.get("totalBalance").is_some() {
        return Some(QuotaKind::Balance {
            currency: "CNY".to_string(),
            topped_up: parse_amount(data_node.get("chargeBalance")),
            granted: parse_amount(data_node.get("balance")),
            total_remaining: parse_amount(data_node.get("totalBalance")),
        });
    }

    // Moonshot 风格
    if data_node.get("available_balance").is_some() {
        return Some(QuotaKind::Balance {
            currency: "CNY".to_string(),
            topped_up: parse_amount(data_node.get("cash_balance")),
            granted: parse_amount(data_node.get("voucher_balance")),
            total_remaining: parse_amount(data_node.get("available_balance")),
        });
    }

    // 通用 total_balance / balance 字段
    let total = data_node
        .get("total_balance")
        .or_else(|| data_node.get("totalBalance"))
        .or_else(|| data_node.get("balance"));
    if total.is_some() {
        let currency = data_node
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("CNY")
            .to_string();
        return Some(QuotaKind::Balance {
            currency,
            topped_up: parse_amount(
                data_node
                    .get("topped_up_balance")
                    .or_else(|| data_node.get("chargeBalance")),
            ),
            granted: parse_amount(
                data_node
                    .get("granted_balance")
                    .or_else(|| data_node.get("voucherBalance")),
            ),
            total_remaining: parse_amount(total),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{normalize_base_url, parse_balance_payload};
    use serde_json::json;

    #[test]
    fn normalizes_base_url_with_v1_suffix() {
        assert_eq!(
            normalize_base_url("https://api.xiaomimimo.com"),
            "https://api.xiaomimimo.com/v1"
        );
        assert_eq!(
            normalize_base_url("https://api.xiaomimimo.com/v1/"),
            "https://api.xiaomimimo.com/v1"
        );
        assert_eq!(
            normalize_base_url("https://custom.example.com/openai/v1"),
            "https://custom.example.com/openai/v1"
        );
    }

    #[test]
    fn parses_deepseek_style_balance() {
        let payload = json!({
            "balance_infos": [{
                "currency": "CNY",
                "total_balance": "12.34",
                "granted_balance": "1.00",
                "topped_up_balance": "11.34"
            }]
        });
        match parse_balance_payload(&payload) {
            Some(crate::quota::models::QuotaKind::Balance {
                currency,
                total_remaining,
                ..
            }) => {
                assert_eq!(currency, "CNY");
                assert!((total_remaining - 12.34).abs() < 1e-9);
            }
            _ => panic!("expected balance"),
        }
    }

    #[test]
    fn rejects_empty_payload() {
        assert!(parse_balance_payload(&json!({ "ok": true })).is_none());
    }
}
