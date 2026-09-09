use serde_json::Value;
use std::{env, fs, path::Path};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType, QuotaKind};

pub async fn fetch_workbuddy_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Workbuddy,
        name: account.name.clone(),
        plan: Some("WorkBuddy 个人版".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Workbuddy
            .default_dashboard_url()
            .map(String::from),
    };

    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    let wb_dir = Path::new(&home).join(".workbuddy");
    let has_local_client = wb_dir.exists();

    // 仅展示本地配置中真实存在的积分数据，不提供猜测或演示值。
    let mut plan_name = "WorkBuddy 个人版".to_string();

    if has_local_client {
        let models_json_path = wb_dir.join("models.json");
        if models_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&models_json_path) {
                if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                    let credits_remaining = json_val.get("credits").and_then(Value::as_f64);
                    let credits_total = json_val
                        .get("total")
                        .or_else(|| json_val.get("totalCredits"))
                        .and_then(Value::as_f64);
                    if let Some(plan) = json_val.get("plan").and_then(Value::as_str) {
                        plan_name = plan.to_string();
                    }

                    if let Some(remaining) = credits_remaining {
                        quota.is_healthy = true;
                        quota.plan = Some(plan_name);
                        quota.quotas.push(QuotaKind::Credits {
                            label: Some("可用积分 (Credits)".to_string()),
                            remaining,
                            total: credits_total,
                        });
                        return quota;
                    }
                }
            }
        }

        quota.plan = Some(plan_name);
        quota.error_message =
            Some("已检测到 WorkBuddy，但本地配置中没有可确认的积分数据".to_string());
        return quota;
    }

    if let Some(key) = &account.api_key {
        if !key.trim().is_empty() {
            quota.plan = Some("WorkBuddy Token".to_string());
            quota.error_message =
                Some("已配置 WorkBuddy Token，但暂时无法可靠查询积分".to_string());
            return quota;
        }
    }

    quota.error_message =
        Some("未在本地 ~/.workbuddy 发现安装目录，且未配置 WorkBuddy 接入 Token".to_string());
    quota
}
