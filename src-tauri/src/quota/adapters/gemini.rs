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
        reset_credits: None,
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
            quota.error_message = Some("未检测到运行中的 Google AI Pro (Antigravity) 语言服务，请确保已启动 Antigravity，或在设置中配置 Gemini API Key".to_string());
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

#[derive(Debug)]
struct AntigravityTarget {
    csrf_token: String,
    ports: Vec<u16>,
}

async fn probe_single_port(
    client: &Client,
    port: u16,
    csrf_token: &str,
) -> (Option<Value>, Option<Value>) {
    let base_url = format!("https://127.0.0.1:{}", port);
    let summary_url = format!(
        "{}/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary",
        base_url
    );
    let status_url = format!(
        "{}/exa.language_server_pb.LanguageServerService/GetUserStatus",
        base_url
    );

    let summary_req = client
        .post(&summary_url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .body("{}")
        .send();

    let status_req = client
        .post(&status_url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .body("{}")
        .send();

    let (summary_res, status_res) = tokio::join!(summary_req, status_req);

    let summary_data = match summary_res {
        Ok(resp) if resp.status().is_success() => resp
            .json::<Value>()
            .await
            .ok()
            .filter(|v| v.pointer("/response/groups").is_some()),
        _ => None,
    };

    let status_data = match status_res {
        Ok(resp) if resp.status().is_success() => resp
            .json::<Value>()
            .await
            .ok()
            .filter(|v| v.get("userStatus").is_some()),
        _ => None,
    };

    (summary_data, status_data)
}

async fn fetch_antigravity_local_status() -> Result<(String, Vec<QuotaKind>, Option<crate::quota::models::PaceStatus>), String> {
    let targets = discover_antigravity_targets().await;
    if targets.is_empty() {
        return Err("未找到运行中的 Antigravity language_server 进程".to_string());
    }

    // 关键配置：no_proxy() 强制直连 127.0.0.1，避免被全局/系统代理（VPN/Clash 等）拦截
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .timeout(Duration::from_millis(3000))
        .build()
        .map_err(|e| format!("初始化本地 HTTP 客户端失败: {}", e))?;

    let mut quota_summary_data: Option<Value> = None;
    let mut user_status_data: Option<Value> = None;

    // 并行探测所有候选端口，快速收敛
    for target in &targets {
        let mut set = tokio::task::JoinSet::new();
        for &port in &target.ports {
            let cl = client.clone();
            let token = target.csrf_token.clone();
            set.spawn(async move {
                probe_single_port(&cl, port, &token).await
            });
        }

        while let Some(res) = set.join_next().await {
            if let Ok((summary_opt, status_opt)) = res {
                if quota_summary_data.is_none() && summary_opt.is_some() {
                    quota_summary_data = summary_opt;
                }
                if user_status_data.is_none() && status_opt.is_some() {
                    user_status_data = status_opt;
                }
                if quota_summary_data.is_some() && user_status_data.is_some() {
                    set.abort_all();
                    break;
                }
            }
        }

        if quota_summary_data.is_some() || user_status_data.is_some() {
            break;
        }
    }

    let mut plan_name = "Google AI Pro".to_string();
    if let Some(val) = &user_status_data {
        if let Some(name) = val.pointer("/userStatus/planStatus/planInfo/planName").and_then(Value::as_str) {
            plan_name = format!("Google AI {}", name);
        } else if let Some(tier) = val.pointer("/userStatus/userTier/name").and_then(Value::as_str) {
            plan_name = tier.to_string();
        }
    }

    let mut quotas = Vec::new();
    let mut pace_status = None;
    let now_sec = chrono_now_ms() / 1000;

    // 优先从 RetrieveUserQuotaSummary 解析精准周额度与5小时额度
    if let Some(summary) = &quota_summary_data {
        if let Some(groups) = summary.pointer("/response/groups").and_then(Value::as_array) {
            for g in groups {
                let display_name = g.get("displayName").and_then(Value::as_str).unwrap_or_default();
                let group_prefix = if display_name.to_lowercase().contains("gemini") {
                    "Gemini 模型"
                } else if display_name.to_lowercase().contains("claude") || display_name.to_lowercase().contains("gpt") {
                    "Claude 和 GPT 模型"
                } else {
                    display_name
                };

                if let Some(buckets) = g.get("buckets").and_then(Value::as_array) {
                    for b in buckets {
                        let window = b.get("window").and_then(Value::as_str).unwrap_or_default();
                        let remaining_fraction = b.get("remainingFraction").and_then(Value::as_f64).unwrap_or(1.0);
                        let used_percent = ((1.0 - remaining_fraction) * 100.0).clamp(0.0, 100.0);
                        let reset_time_str = b.get("resetTime").and_then(Value::as_str);
                        let resets_at = reset_time_str.and_then(parse_rfc3339_seconds);
                        let resets_in_seconds = resets_at.map(|ts| (ts - now_sec).max(0));

                        let window_label = match window {
                            "weekly" => "每周限额",
                            "5h" => "5小时限额",
                            _ => b.get("displayName").and_then(Value::as_str).unwrap_or("周期限额"),
                        };

                        let period_label = format!("{} ({})", group_prefix, window_label);

                        quotas.push(QuotaKind::RateLimit {
                            period_label,
                            used_percent,
                            resets_at,
                            resets_in_seconds,
                        });

                        // 针对 Gemini 5h 或 weekly 计算健康配速
                        if group_prefix.contains("Gemini") && window == "5h" {
                            if let Some(rem_sec) = resets_in_seconds {
                                pace_status = Some(calculate_pace(used_percent, 18000, rem_sec));
                            }
                        }
                    }
                }
            }
        }
    }

    // 降级：如果 RetrieveUserQuotaSummary 为空，从 GetUserStatus clientModelConfigs 解析
    if quotas.is_empty() {
        if let Some(val) = &user_status_data {
            if let Some(models) = val.pointer("/userStatus/cascadeModelConfigData/clientModelConfigs").and_then(Value::as_array) {
                let mut gemini_found = false;
                let mut claude_found = false;

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
                            period_label: "Gemini 模型 (5小时限额)".to_string(),
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
                            period_label: "Claude 和 GPT 模型 (5小时限额)".to_string(),
                            used_percent,
                            resets_at,
                            resets_in_seconds,
                        });
                    }
                }
            }
        }
    }

    if quotas.is_empty() {
        return Err("未能解析出配额信息".to_string());
    }

    Ok((plan_name, quotas, pace_status))
}

async fn discover_antigravity_targets() -> Vec<AntigravityTarget> {
    let mut targets = Vec::new();

    #[cfg(windows)]
    {
        let mut cmd = tokio::process::Command::new("powershell");
        cmd.creation_flags(0x08000000);
        if let Ok(cmd_output) = cmd
            .args([
                "-ExecutionPolicy",
                "Bypass",
                "-NonInteractive",
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process | Where-Object { $_.Name -like '*language_server*' -or $_.CommandLine -like '*language_server*' } | ForEach-Object { $p = $_.ProcessId; $c = $_.CommandLine; $conns = (Get-NetTCPConnection -OwningProcess $p -State Listen -ErrorAction SilentlyContinue | Select-Object -ExpandProperty LocalPort) -join ','; \"$p|||$c|||$conns\" }",
            ])
            .output()
            .await
        {
            let stdout_str = String::from_utf8_lossy(&cmd_output.stdout);
            for line in stdout_str.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split("|||").collect();
                if parts.len() >= 2 {
                    let pid_str = parts[0];
                    let command_line = parts[1];
                    let mut ports = Vec::new();
                    if parts.len() >= 3 {
                        for p in parts[2].split(',') {
                            if let Ok(port) = p.trim().parse::<u16>() {
                                if !ports.contains(&port) {
                                    ports.push(port);
                                }
                            }
                        }
                    }

                    if ports.is_empty() {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            ports = get_ports_by_netstat(pid).await;
                        }
                    }

                    if let Some(csrf_token) = extract_csrf_token(command_line) {
                        if !ports.is_empty() {
                            targets.push(AntigravityTarget { csrf_token, ports });
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(cmd_output) = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("ps -eo pid,command | grep -i language_server | grep -v grep")
            .output()
            .await
        {
            let stdout_str = String::from_utf8_lossy(&cmd_output.stdout);
            for line in stdout_str.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(pid) = parts[0].parse::<u32>() {
                        let cmd_line = parts[1..].join(" ");
                        if let Some(csrf_token) = extract_csrf_token(&cmd_line) {
                            let ports = get_unix_listening_ports(pid).await;
                            if !ports.is_empty() {
                                targets.push(AntigravityTarget { csrf_token, ports });
                            }
                        }
                    }
                }
            }
        }
    }

    targets
}

#[cfg(windows)]
async fn get_ports_by_netstat(pid: u32) -> Vec<u16> {
    let mut ports = Vec::new();
    let mut cmd = tokio::process::Command::new("cmd");
    cmd.creation_flags(0x08000000);
    if let Ok(out) = cmd.args(["/C", "netstat -ano"]).output().await {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let pid_str = pid.to_string();
        for line in stdout.lines() {
            let line = line.trim();
            if !line.to_uppercase().contains("LISTENING") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let line_pid = parts[parts.len() - 1];
                if line_pid == pid_str {
                    let local_addr = parts[1];
                    if let Some(idx) = local_addr.rfind(':') {
                        if let Ok(port) = local_addr[idx + 1..].parse::<u16>() {
                            if !ports.contains(&port) {
                                ports.push(port);
                            }
                        }
                    }
                }
            }
        }
    }
    ports
}

#[cfg(not(windows))]
async fn get_unix_listening_ports(pid: u32) -> Vec<u16> {
    let mut ports = Vec::new();
    if let Ok(out) = tokio::process::Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-a", "-p", &pid.to_string()])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if let Some(idx) = part.rfind(':') {
                    if let Ok(port) = part[idx + 1..].parse::<u16>() {
                        if !ports.contains(&port) {
                            ports.push(port);
                        }
                    }
                }
            }
        }
    }
    ports
}

fn extract_csrf_token(cmd: &str) -> Option<String> {
    if let Some(pos) = cmd.find("--csrf_token") {
        let remainder = &cmd[pos + "--csrf_token".len()..];
        let trimmed = remainder.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
        let token: String = trimmed
            .chars()
            .take_while(|&c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            .collect();
        if !token.is_empty() {
            return Some(token);
        }
    }

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for i in 0..parts.len() {
        if parts[i] == "--csrf_token" && i + 1 < parts.len() {
            return Some(parts[i + 1].trim_matches('"').trim_matches('\'').to_string());
        }
        if parts[i].starts_with("--csrf_token=") {
            return Some(parts[i].trim_start_matches("--csrf_token=").trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

use super::parse_rfc3339_seconds;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_csrf_token() {
        let cmd1 = r#""C:\bin\language_server.exe" --csrf_token be67ca93-8574-4156-a4ee-e31eb9b2caf2 --port 0"#;
        assert_eq!(extract_csrf_token(cmd1), Some("be67ca93-8574-4156-a4ee-e31eb9b2caf2".to_string()));

        let cmd2 = r#""D:\Program Files\Antigravity\language_server.exe" --csrf_token=xyz_123-abc"#;
        assert_eq!(extract_csrf_token(cmd2), Some("xyz_123-abc".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_antigravity_local_status_if_running() {
        let res = fetch_antigravity_local_status().await;
        if let Ok((plan, quotas, pace)) = res {
            println!("Detected plan: {}, quotas: {:?}, pace: {:?}", plan, quotas, pace);
            assert!(!quotas.is_empty());
        }
    }
}
