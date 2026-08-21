use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, QuotaKind, ProviderQuota, ProviderType};

pub async fn fetch_openrouter_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Openrouter,
        name: account.name.clone(),
        plan: Some("Pay-as-you-go".to_string()),
        quotas: Vec::new(),
        pace: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Openrouter.default_dashboard_url().map(String::from),
    };

    let api_key = match &account.api_key {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => {
            quota.error_message = Some("未配置 OpenRouter API Key".to_string());
            return quota;
        }
    };

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
        .get("https://openrouter.ai/api/v1/auth/key")
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

    // OpenRouter /api/v1/auth/key structure:
    // { "data": { "label": "my-key", "usage": 1.25, "limit": 10.0, "is_free_tier": false, "rate_limit": { "requests": 20, "interval": "10s" } } }
    if let Some(data) = json_val.get("data") {
        let usage = data.get("usage").and_then(Value::as_f64).unwrap_or(0.0);
        let limit = data.get("limit").and_then(Value::as_f64);
        let is_free_tier = data.get("is_free_tier").and_then(Value::as_bool).unwrap_or(false);

        if is_free_tier {
            quota.plan = Some("Free Tier".to_string());
        }

        if let Some(limit_val) = limit {
            let remaining = (limit_val - usage).max(0.0);
            quota.quotas.push(QuotaKind::Balance {
                currency: "USD".to_string(),
                topped_up: limit_val,
                granted: 0.0,
                total_remaining: remaining,
            });
        } else {
            // 没有 limit 时，说明无固定限额或按充值扣费
            quota.quotas.push(QuotaKind::Credits {
                remaining: usage,
                total: None,
            });
        }

        quota.is_healthy = true;
    } else {
        quota.error_message = Some("未能从 OpenRouter 返回中读取到 data 字段".to_string());
    }

    quota
}
