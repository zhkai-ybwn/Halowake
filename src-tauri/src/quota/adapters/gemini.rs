use std::time::Duration;
use reqwest::Client;
use serde_json::Value;

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, QuotaKind, ProviderQuota, ProviderType};
use crate::quota::pace::calculate_pace;

pub async fn fetch_gemini_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Gemini,
        name: account.name.clone(),
        plan: Some("Google AI Pro".to_string()),
        quotas: Vec::new(),
        pace: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Gemini.default_dashboard_url().map(String::from),
    };

    // 1. 优先尝试连接本地运行中的 Google AI Pro (Antigravity) 语言服务
    if let Ok((plan_name, quotas, pace_status)) = fetch_antigravity_local_status().await {
        quota.plan = Some(plan_name);
        quota.quotas = quotas;
        quota.pace = pace_status;
        quota.is_healthy = true;
        return quota;
    }

    // 2. 降级：如果配置了 Google AI Studio API Key
    let api_key = match &account.api_key {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => {
            quota.error_message = Some("未检测到运行中的 Google AI Pro 服务，且未配置 Gemini API Key".to_string());
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

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let response = match client.get(&url).send().await {
        Ok(res) => res,
        Err(e) => {
            quota.error_message = Some(format!("网络连接失败: {}", e));
            return quota;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        quota.error_message = Some(format!("Gemini API 返回错误 ({}): {}", status, error_body));
        return quota;
    }

    let json_val = match response.json::<Value>().await {
        Ok(val) => val,
        Err(e) => {
            quota.error_message = Some(format!("解析响应失败: {}", e));
            return quota;
        }
    };

    if let Some(models) = json_val.get("models").and_then(Value::as_array) {
        let model_count = models.len();
        quota.is_healthy = true;
        quota.plan = Some(format!("AI Studio (可用模型: {})", model_count));
        quota.quotas.push(QuotaKind::RateLimit {
            period_label: "Free Tier (15 RPM / 1500 RPD)".to_string(),
            used_percent: 0.0,
            resets_at: None,
            resets_in_seconds: None,
        });
    }

    quota
}

async fn fetch_antigravity_local_status() -> Result<(String, Vec<QuotaKind>, Option<crate::quota::models::PaceStatus>), String> {
    // 1. 通过 powershell 查询运行中的 language_server.exe 的命令行与端口
    let mut cmd = tokio::process::Command::new("powershell");
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let cmd_output = cmd
        .args([
            "-NoProfile",
            "-Command",
            "$proc = Get-CimInstance Win32_Process -Filter \"Name = 'language_server.exe'\" | Select-Object -First 1; if ($proc) { $conns = (Get-NetTCPConnection -OwningProcess $proc.ProcessId -State Listen -ErrorAction SilentlyContinue | Select-Object -ExpandProperty LocalPort) -join ','; \"$($proc.CommandLine)|||$conns\" }",
        ])
        .output()
        .await
        .map_err(|e| format!("执行进程查询失败: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&cmd_output.stdout);
    let parts: Vec<&str> = stdout_str.trim().split("|||").collect();
    if parts.len() < 2 {
        return Err("未找到运行中的 language_server 进程".to_string());
    }

    let command_line = parts[0];
    let ports_str = parts[1];

    // 提取 csrf_token
    let csrf_token = extract_arg(command_line, "--csrf_token")
        .ok_or_else(|| "未从命令行中解析出 csrf_token".to_string())?;

    let ports: Vec<u16> = ports_str
        .split(',')
        .filter_map(|p| p.trim().parse::<u16>().ok())
        .collect();

    if ports.is_empty() {
        return Err("未检测到 language_server 监听端口".to_string());
    }

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let mut response_data: Option<Value> = None;

    for port in ports {
        let url = format!("https://127.0.0.1:{}/exa.language_server_pb.LanguageServerService/GetUserStatus", port);
        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Connect-Protocol-Version", "1")
            .header("X-Codeium-Csrf-Token", &csrf_token)
            .body("{}")
            .send()
            .await;

        if let Ok(res) = resp {
            if res.status().is_success() {
                if let Ok(val) = res.json::<Value>().await {
                    response_data = Some(val);
                    break;
                }
            }
        }
    }

    let val = response_data.ok_or_else(|| "无法连接到本地 language_server RPC 端点".to_string())?;

    let mut plan_name = "Google AI Pro".to_string();
    if let Some(tier) = val.pointer("/userStatus/userTier/name").and_then(Value::as_str) {
        plan_name = tier.to_string();
    }

    let mut quotas = Vec::new();
    let mut pace_status = None;

    // 解析各模型配额池
    if let Some(models) = val.pointer("/userStatus/cascadeModelConfigData/clientModelConfigs").and_then(Value::as_array) {
        let mut gemini_found = false;
        let mut claude_found = false;

        let now_sec = chrono_now_ms() / 1000;

        for m in models {
            let label = m.get("label").and_then(Value::as_str).unwrap_or_default();
            let quota_info = match m.get("quotaInfo") {
                Some(q) if !q.is_null() => q,
                _ => continue,
            };

            let remaining_frac = quota_info.get("remainingFraction").and_then(Value::as_f64).unwrap_or(1.0);
            let used_percent = ((1.0 - remaining_frac) * 100.0).clamp(0.0, 100.0);
            let reset_time_str = quota_info.get("resetTime").and_then(Value::as_str);

            let resets_at = reset_time_str.and_then(parse_rfc3339_seconds);
            let resets_in_seconds = resets_at.map(|ts| (ts - now_sec).max(0));

            if label.contains("Gemini") && !gemini_found {
                gemini_found = true;
                quotas.push(QuotaKind::RateLimit {
                    period_label: "Gemini 模型限额 (5h)".to_string(),
                    used_percent,
                    resets_at,
                    resets_in_seconds,
                });

                if let Some(rem_sec) = resets_in_seconds {
                    pace_status = Some(calculate_pace(used_percent, 18000, rem_sec));
                }
            } else if (label.contains("Claude") || label.contains("GPT")) && !claude_found {
                claude_found = true;
                quotas.push(QuotaKind::RateLimit {
                    period_label: "Claude / GPT 模型限额".to_string(),
                    used_percent,
                    resets_at,
                    resets_in_seconds,
                });
            }
        }
    }

    if quotas.is_empty() {
        return Err("未能解析出模型配额信息".to_string());
    }

    Ok((plan_name, quotas, pace_status))
}

fn extract_arg(cmd: &str, arg_name: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for i in 0..parts.len() {
        if parts[i] == arg_name && i + 1 < parts.len() {
            return Some(parts[i + 1].trim_matches('"').to_string());
        }
        if parts[i].starts_with(&format!("{}=", arg_name)) {
            return Some(parts[i].trim_start_matches(&format!("{}=", arg_name)).trim_matches('"').to_string());
        }
    }
    None
}

fn parse_rfc3339_seconds(s: &str) -> Option<i64> {
    // 简单解析 2026-08-21T05:56:43Z
    let s = s.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_parts: Vec<i64> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 || time_parts.len() < 3 {
        return None;
    }

    let y = date_parts[0];
    let m = date_parts[1];
    let d = date_parts[2];
    let hour = time_parts[0];
    let min = time_parts[1];
    let sec = time_parts[2];

    // 计算简易 Unix timestamp (UTC)
    let mut days = (y - 1970) * 365 + (y - 1969) / 4;
    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for i in 1..m {
        days += month_days[i as usize];
    }
    if m > 2 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)) {
        days += 1;
    }
    days += d - 1;

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}
