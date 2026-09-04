use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::quota::models::{AccountConfig, QuotaKind, ProviderQuota, ProviderType};

pub async fn fetch_deepseek_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Deepseek,
        name: account.name.clone(),
        plan: Some("Pay-as-you-go".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Deepseek.default_dashboard_url().map(String::from),
    };

    let api_key = match &account.api_key {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => {
            quota.error_message = Some("未配置 DeepSeek API Key".to_string());
            return quota;
        }
    };

    let base_url = account
        .base_url
        .as_deref()
        .unwrap_or("https://api.deepseek.com")
        .trim_end_matches('/');
    let balance_url = format!("{}/user/balance", base_url);

    let client = match Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    let response = match client
        .get(&balance_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            quota.error_message = Some(format!("网络请求失败: {}", e));
            return quota;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        quota.error_message = Some(format!("API 返回错误 ({}): {}", status, error_body));
        return quota;
    }

    let json_val = match response.json::<Value>().await {
        Ok(val) => val,
        Err(e) => {
            quota.error_message = Some(format!("解析响应 JSON 失败: {}", e));
            return quota;
        }
    };

    // DeepSeek response structure:
    // { "is_available": true, "balance_infos": [{ "currency": "CNY", "total_balance": "0.36", "granted_balance": "0.00", "topped_up_balance": "0.36" }] }
    let is_available = json_val
        .get("is_available")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if let Some(balance_infos) = json_val.get("balance_infos").and_then(Value::as_array) {
        for info in balance_infos {
            let currency = info
                .get("currency")
                .and_then(Value::as_str)
                .unwrap_or("CNY")
                .to_string();
            let total = parse_amount(info.get("total_balance"));
            let granted = parse_amount(info.get("granted_balance"));
            let topped_up = parse_amount(info.get("topped_up_balance"));

            quota.quotas.push(QuotaKind::Balance {
                currency,
                topped_up,
                granted,
                total_remaining: total,
            });
        }
    }

    if quota.quotas.is_empty() {
        quota.error_message = Some("未能从 DeepSeek 响应中解析出有效余额数据".to_string());
    } else {
        quota.is_healthy = is_available;
    }

    quota
}

fn parse_amount(val: Option<&Value>) -> f64 {
    match val {
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

pub fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
