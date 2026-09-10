use reqwest::Client;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType, QuotaKind};

pub fn get_candidate_cursor_db_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = env::var("APPDATA") {
            paths.push(
                PathBuf::from(app_data)
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("state.vscdb"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            paths.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("state.vscdb"),
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = env::var("HOME") {
            paths.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("state.vscdb"),
            );
        }
    }

    paths
}

pub fn has_cursor_installation() -> bool {
    get_candidate_cursor_db_paths().iter().any(|p| p.exists())
}

#[derive(Debug, Default)]
struct CursorLocalInfo {
    access_token: Option<String>,
    email: Option<String>,
    membership_type: Option<String>,
}

fn normalize_stored_value(value: Option<String>) -> Option<String> {
    let value = value?.trim().to_string();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        serde_json::from_str::<String>(&value).ok()
    } else if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn read_cursor_local_db(db_path: &Path) -> Option<CursorLocalInfo> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .ok()?;

    let mut stmt = conn
        .prepare(
            "SELECT key, value FROM ItemTable WHERE key IN (
                'cursorAuth/accessToken',
                'cursorAuth/cachedEmail',
                'cursorAuth/stripeMembershipType'
            )",
        )
        .ok()?;

    let mut info = CursorLocalInfo::default();
    let rows = stmt
        .query_map([], |row| {
            let k: String = row.get(0)?;
            let v: Option<String> = row.get(1)?;
            Ok((k, v))
        })
        .ok()?;

    for row in rows.flatten() {
        match (row.0.as_str(), row.1) {
            ("cursorAuth/accessToken", v) => info.access_token = normalize_stored_value(v),
            ("cursorAuth/cachedEmail", v) => info.email = normalize_stored_value(v),
            ("cursorAuth/stripeMembershipType", v) => {
                info.membership_type = normalize_stored_value(v)
            }
            _ => {}
        }
    }

    Some(info)
}

fn cursor_number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
    })
}

fn cursor_timestamp(value: Option<&Value>) -> Option<i64> {
    cursor_number(value)
        .map(|value| value as i64)
        .map(|value| {
            if value > 10_000_000_000 {
                value / 1000
            } else {
                value
            }
        })
        .filter(|value| *value > 0)
}

fn parse_cursor_usage(value: &Value) -> Result<Vec<QuotaKind>, String> {
    if value.get("enabled").and_then(Value::as_bool) == Some(false) {
        return Err("Cursor 当前没有启用中的订阅额度".to_string());
    }
    let plan = value
        .get("planUsage")
        .or_else(|| value.get("plan_usage"))
        .and_then(Value::as_object)
        .ok_or_else(|| "Cursor 用量响应缺少 planUsage".to_string())?;
    let resets_at = cursor_timestamp(
        value
            .get("billingCycleEnd")
            .or_else(|| value.get("billing_cycle_end")),
    );
    let mut quotas = Vec::new();
    for (key, fallback_key, label) in [
        ("autoPercentUsed", "auto_percent_used", "Cursor Models"),
        ("apiPercentUsed", "api_percent_used", "Other Models"),
    ] {
        if let Some(percent) = cursor_number(plan.get(key).or_else(|| plan.get(fallback_key))) {
            quotas.push(QuotaKind::RateLimit {
                period_label: label.to_string(),
                used_percent: percent.clamp(0.0, 100.0),
                resets_at,
                resets_in_seconds: None,
            });
        }
    }
    if quotas.is_empty() {
        let used_percent = cursor_number(
            plan.get("totalPercentUsed")
                .or_else(|| plan.get("total_percent_used")),
        )
        .or_else(|| {
            let limit = cursor_number(plan.get("limit"))?;
            if limit <= 0.0 {
                return None;
            }
            let spend = cursor_number(plan.get("totalSpend"))
                .or_else(|| cursor_number(plan.get("total_spend")))
                .or_else(|| {
                    cursor_number(plan.get("remaining")).map(|remaining| limit - remaining)
                })?;
            Some(spend / limit * 100.0)
        })
        .ok_or_else(|| "Cursor 用量响应没有可识别的额度字段".to_string())?;
        quotas.push(QuotaKind::RateLimit {
            period_label: "Billing Cycle".to_string(),
            used_percent: used_percent.clamp(0.0, 100.0),
            resets_at,
            resets_in_seconds: None,
        });
    }
    Ok(quotas)
}

pub async fn fetch_cursor_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Cursor,
        name: account.name.clone(),
        plan: Some("Cursor".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Cursor
            .default_dashboard_url()
            .map(String::from),
    };

    let mut local_info = None;
    for path in get_candidate_cursor_db_paths() {
        if path.exists() {
            if let Some(info) = read_cursor_local_db(&path) {
                local_info = Some(info);
                break;
            }
        }
    }

    let membership = local_info
        .as_ref()
        .and_then(|i| i.membership_type.clone())
        .unwrap_or_else(|| "free".to_string());
    let email = local_info.as_ref().and_then(|i| i.email.clone());

    let plan_display = match membership.to_lowercase().as_str() {
        "pro" => "Cursor Pro".to_string(),
        "business" | "enterprise" => "Cursor Business".to_string(),
        "free" | "hobby" => "Cursor Free".to_string(),
        other => format!("Cursor ({})", other),
    };

    if let Some(em) = &email {
        quota.plan = Some(format!("{} · {}", plan_display, em));
    } else {
        quota.plan = Some(plan_display.clone());
    }

    let token = account
        .api_key
        .as_deref()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| local_info.as_ref().and_then(|i| i.access_token.as_deref()));

    if let Some(tok) = token {
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            Ok(client) => client,
            Err(error) => {
                quota.error_message = Some(format!("初始化 Cursor HTTP 客户端失败: {error}"));
                return quota;
            }
        };
        let response = match client
            .post("https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage")
            .header("Authorization", format!("Bearer {}", tok.trim_matches('"')))
            .header("Content-Type", "application/json")
            .header("Connect-Protocol-Version", "1")
            .body("{}")
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                quota.error_message = Some(format!("Cursor 用量请求失败: {error}"));
                return quota;
            }
        };
        let status = response.status();
        if !status.is_success() {
            quota.error_message = Some(if matches!(status.as_u16(), 401 | 403) {
                "Cursor 登录凭据已过期，请打开 Cursor 让客户端刷新登录状态".to_string()
            } else {
                format!("Cursor 用量接口返回 {status}")
            });
            return quota;
        }
        let value = match response.json::<Value>().await {
            Ok(value) => value,
            Err(error) => {
                quota.error_message = Some(format!("解析 Cursor 用量响应失败: {error}"));
                return quota;
            }
        };
        match parse_cursor_usage(&value) {
            Ok(quotas) => {
                quota.quotas = quotas;
                quota.is_healthy = true;
            }
            Err(error) => quota.error_message = Some(error),
        }
    } else if local_info.is_some() {
        quota.error_message = Some("已检测到 Cursor 客户端，但没有可用的访问 Token".to_string());
    } else {
        quota.error_message =
            Some("未在本地发现 Cursor 客户端存储，请确保已安装并登录 Cursor".to_string());
    }

    quota
}

#[cfg(test)]
mod tests {
    use super::{normalize_stored_value, parse_cursor_usage};
    use crate::quota::models::QuotaKind;
    use serde_json::json;

    #[test]
    fn parses_connect_rpc_quota_pools() {
        let quotas = parse_cursor_usage(&json!({
            "enabled": true,
            "billingCycleEnd": "1800000000000",
            "planUsage": {
                "autoPercentUsed": 12.5,
                "apiPercentUsed": 43.25,
                "totalPercentUsed": 20
            }
        }))
        .expect("parse cursor usage");

        assert_eq!(quotas.len(), 2);
        assert!(matches!(
            &quotas[0],
            QuotaKind::RateLimit { period_label, used_percent, resets_at: Some(1_800_000_000), .. }
                if period_label == "Cursor Models" && (*used_percent - 12.5).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn normalizes_json_encoded_cursor_values() {
        assert_eq!(
            normalize_stored_value(Some("\"token-value\"".to_string())),
            Some("token-value".to_string())
        );
    }
}
