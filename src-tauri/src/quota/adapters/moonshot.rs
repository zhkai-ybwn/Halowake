use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::quota::adapters::deepseek::{chrono_now_ms, parse_amount};
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType, QuotaKind};

pub async fn fetch_moonshot_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Moonshot,
        name: account.name.clone(),
        plan: Some("Moonshot 开发者平台".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Moonshot.default_dashboard_url().map(String::from),
    };

    let api_key = match &account.api_key {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => {
            quota.error_message = Some("未配置 Moonshot (Kimi) API Key".to_string());
            return quota;
        }
    };

    let base_url = account
        .base_url
        .as_deref()
        .unwrap_or("https://api.moonshot.cn/v1")
        .trim_end_matches('/');
    let balance_url = format!("{}/users/me/balance", base_url);

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
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

    let data_node = json_val.get("data").unwrap_or(&json_val);
    let total = parse_amount(data_node.get("available_balance"));
    let topped_up = parse_amount(data_node.get("cash_balance"));
    let granted = parse_amount(data_node.get("voucher_balance"));

    let currency = if base_url.contains(".ai") { "USD" } else { "CNY" };

    quota.quotas.push(QuotaKind::Balance {
        currency: currency.to_string(),
        topped_up,
        granted,
        total_remaining: total,
    });

    quota.is_healthy = true;
    quota.plan = Some("Kimi 开放平台".to_string());

    quota
}
