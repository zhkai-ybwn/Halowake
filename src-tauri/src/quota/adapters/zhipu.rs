use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType, QuotaKind};
use crate::quota::pace::calculate_pace;

pub async fn fetch_zhipu_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Zhipu,
        name: account.name.clone(),
        plan: Some("GLM Coding Plan".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Zhipu
            .default_dashboard_url()
            .map(String::from),
    };

    let api_key = match &account.api_key {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => {
            quota.error_message = Some("未配置智谱 GLM API Key".to_string());
            return quota;
        }
    };

    let base_url = account
        .base_url
        .as_deref()
        .unwrap_or("https://open.bigmodel.cn")
        .trim_end_matches('/');

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    // 1. 优先尝试查询 GLM Coding Plan 监控配额
    let monitor_url = format!("{}/api/monitor/usage/quota/limit", base_url);
    let monitor_res = client
        .get(&monitor_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await;

    match monitor_res {
        Ok(res) if res.status().is_success() => {
            if let Ok(json_val) = res.json::<Value>().await {
                // 成功解析 GLM Coding 周期配额
                let mut has_limits = false;
                if let Some(five_hour) =
                    json_val.get("fiveHour").or_else(|| json_val.get("cycle5h"))
                {
                    if let Some(used) = five_hour.get("usedPercent").and_then(Value::as_f64) {
                        let resets_in = five_hour.get("resetsInSeconds").and_then(Value::as_i64);
                        quota.quotas.push(QuotaKind::RateLimit {
                            period_label: "5 小时限额".to_string(),
                            used_percent: used,
                            resets_at: resets_in.map(|seconds| (chrono_now_ms() / 1000) + seconds),
                            resets_in_seconds: resets_in,
                        });
                        if let Some(seconds) = resets_in {
                            quota.pace = Some(calculate_pace(used, seconds, 18000));
                        }
                        has_limits = true;
                    }
                }

                if let Some(weekly) = json_val
                    .get("weekly")
                    .or_else(|| json_val.get("cycleWeekly"))
                {
                    if let Some(used) = weekly.get("usedPercent").and_then(Value::as_f64) {
                        let resets_in = weekly.get("resetsInSeconds").and_then(Value::as_i64);
                        quota.quotas.push(QuotaKind::RateLimit {
                            period_label: "每周限额".to_string(),
                            used_percent: used,
                            resets_at: resets_in.map(|seconds| (chrono_now_ms() / 1000) + seconds),
                            resets_in_seconds: resets_in,
                        });
                        has_limits = true;
                    }
                }

                if has_limits {
                    quota.is_healthy = true;
                    quota.plan = Some("GLM Coding Plan (5h/周)".to_string());
                    return quota;
                }
            }
            quota.error_message = Some("智谱额度接口响应中没有可识别的限额数据".to_string());
        }
        Ok(res) => {
            quota.error_message = Some(format!("智谱额度接口返回错误 ({})", res.status()));
        }
        Err(e) => {
            quota.error_message = Some(format!("连接智谱额度接口失败: {}", e));
        }
    }

    quota
}
