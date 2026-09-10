use reqwest::Client;
use serde_json::Value;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{
    AccountConfig, PaceLevel, PaceStatus, ProviderQuota, ProviderType, QuotaKind,
};

#[derive(Debug, Clone)]
struct WorkBuddyAuth {
    access_token: String,
    uid: String,
    domain: String,
    nickname: Option<String>,
    account_type: Option<String>,
}

/// 获取各操作系统下 WorkBuddy / CodeBuddy 客户端可能存储鉴权信息的文件路径列表
pub fn get_candidate_auth_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            let base = PathBuf::from(local_app_data);
            paths.push(
                base.join("CodeBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
            paths.push(
                base.join("WorkBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
        }
        if let Ok(app_data) = env::var("APPDATA") {
            let base = PathBuf::from(app_data);
            paths.push(
                base.join("CodeBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
            paths.push(
                base.join("WorkBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            let app_sup = PathBuf::from(&home)
                .join("Library")
                .join("Application Support");
            paths.push(
                app_sup
                    .join("CodeBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
            paths.push(
                app_sup
                    .join("WorkBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = env::var("HOME") {
            let home_p = PathBuf::from(&home);
            paths.push(
                home_p
                    .join(".local")
                    .join("share")
                    .join("CodeBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
            paths.push(
                home_p
                    .join(".local")
                    .join("share")
                    .join("WorkBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
            paths.push(
                home_p
                    .join(".config")
                    .join("CodeBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
            paths.push(
                home_p
                    .join(".config")
                    .join("WorkBuddyExtension")
                    .join("Data")
                    .join("Public")
                    .join("auth")
                    .join("workbuddy-desktop.info"),
            );
        }
    }

    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    if !home.is_empty() {
        let wb = PathBuf::from(&home).join(".workbuddy");
        paths.push(wb.join("auth").join("workbuddy-desktop.info"));
        paths.push(wb.join("workbuddy-desktop.info"));
    }

    paths
}

/// 检查本地是否安装并登录了 WorkBuddy，或存在 WorkBuddy 配置
pub fn has_workbuddy_credentials() -> bool {
    for path in get_candidate_auth_paths() {
        if path.exists() {
            return true;
        }
    }

    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    if !home.is_empty() {
        if Path::new(&home).join(".workbuddy").exists() {
            return true;
        }
    }

    false
}

fn base64_url_decode(input: &str) -> Option<Vec<u8>> {
    let mut clean = input.replace('-', "+").replace('_', "/");
    while clean.len() % 4 != 0 {
        clean.push('=');
    }
    let bytes = clean.as_bytes();
    let decode_char = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => Some(0),
            _ => None,
        }
    };

    let mut output = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return None;
        }
        let c0 = decode_char(chunk[0])?;
        let c1 = decode_char(chunk[1])?;
        let c2 = decode_char(chunk[2])?;
        let c3 = decode_char(chunk[3])?;

        output.push((c0 << 2) | (c1 >> 4));
        if chunk[2] != b'=' {
            output.push((c1 << 4) | (c2 >> 2));
        }
        if chunk[3] != b'=' {
            output.push((c2 << 6) | c3);
        }
    }
    Some(output)
}

fn decode_jwt_claims(token: &str) -> Option<(Option<String>, Option<String>)> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload_bytes = base64_url_decode(parts[1])?;
    let payload_str = String::from_utf8(payload_bytes).ok()?;
    let val: Value = serde_json::from_str(&payload_str).ok()?;

    let sub = val.get("sub").and_then(Value::as_str).map(String::from);
    let nickname = val
        .get("nickname")
        .or_else(|| val.get("name"))
        .and_then(Value::as_str)
        .map(String::from);

    Some((sub, nickname))
}

fn parse_auth_file(content: &str) -> Option<WorkBuddyAuth> {
    let val = serde_json::from_str::<Value>(content).ok()?;

    let token = val
        .pointer("/auth/accessToken")
        .or_else(|| val.get("accessToken"))
        .or_else(|| val.get("token"))
        .and_then(Value::as_str)?
        .trim();

    if token.is_empty() {
        return None;
    }

    let domain = val
        .pointer("/auth/domain")
        .or_else(|| val.get("domain"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("copilot.tencent.com")
        .to_string();

    let mut uid = val
        .pointer("/account/uid")
        .or_else(|| val.pointer("/accounts/0/uid"))
        .or_else(|| val.get("uid"))
        .and_then(Value::as_str)
        .map(String::from);

    let mut nickname = val
        .pointer("/account/nickname")
        .or_else(|| val.pointer("/accounts/0/nickname"))
        .and_then(Value::as_str)
        .map(String::from);

    let account_type = val
        .pointer("/account/type")
        .or_else(|| val.pointer("/accounts/0/type"))
        .and_then(Value::as_str)
        .map(String::from);

    if uid.is_none() || nickname.is_none() {
        if let Some((jwt_sub, jwt_nick)) = decode_jwt_claims(token) {
            if uid.is_none() {
                uid = jwt_sub;
            }
            if nickname.is_none() {
                nickname = jwt_nick;
            }
        }
    }

    let uid = uid?;

    Some(WorkBuddyAuth {
        access_token: token.to_string(),
        uid,
        domain,
        nickname,
        account_type,
    })
}

fn resolve_workbuddy_auth(account: &AccountConfig) -> Option<WorkBuddyAuth> {
    // 1. 如果用户手动配置了 api_key，优先尝试使用
    if let Some(key) = &account.api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            // 尝试直接作为 JSON 解析
            if let Some(auth) = parse_auth_file(trimmed) {
                return Some(auth);
            }

            // 尝试作为 JWT Token 解析
            if let Some((Some(sub), nickname)) = decode_jwt_claims(trimmed) {
                return Some(WorkBuddyAuth {
                    access_token: trimmed.to_string(),
                    uid: sub,
                    domain: "copilot.tencent.com".to_string(),
                    nickname,
                    account_type: Some("personal".to_string()),
                });
            }
        }
    }

    // 2. 从本地客户端安装或缓存路径自动探测
    for path in get_candidate_auth_paths() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(auth) = parse_auth_file(&content) {
                    return Some(auth);
                }
            }
        }
    }

    None
}

fn parse_number_or_str(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn parse_workbuddy_datetime(dt_str: &str) -> Option<i64> {
    let parts: Vec<&str> = dt_str.trim().split(' ').collect();
    if parts.len() != 2 {
        return None;
    }
    let d_parts: Vec<i64> = parts[0].split('-').filter_map(|s| s.parse().ok()).collect();
    let t_parts: Vec<i64> = parts[1].split(':').filter_map(|s| s.parse().ok()).collect();
    if d_parts.len() != 3 || t_parts.len() != 3 {
        return None;
    }
    let y = d_parts[0];
    let m = d_parts[1];
    let d = d_parts[2];
    let h = t_parts[0];
    let min = t_parts[1];
    let s = t_parts[2];

    let mut days = (y - 1970) * 365 + (y - 1969) / 4;
    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for i in 1..m {
        days += month_days[i as usize];
    }
    if m > 2 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)) {
        days += 1;
    }
    days += d - 1;

    Some(days * 86400 + h * 3600 + min * 60 + s)
}

#[derive(Debug, Default, Clone)]
struct PackageDetailInfo {
    package_name: Option<String>,
    sub_product_name: Option<String>,
    cycle_end_time: Option<String>,
    end_time_ts: Option<i64>,
    capacity_type: Option<i64>,
}

async fn fetch_package_details_map(
    client: &Client,
    auth: &WorkBuddyAuth,
    package_codes: &[String],
) -> std::collections::HashMap<String, PackageDetailInfo> {
    use std::collections::HashMap;
    let mut map: HashMap<String, PackageDetailInfo> = HashMap::new();
    if package_codes.is_empty() {
        return map;
    }

    let endpoints = [
        "https://copilot.tencent.com/billing/meter/get-user-resource-free-packages",
        "https://copilot.tencent.com/billing/meter/get-user-resource-paid-packages",
    ];

    let body = serde_json::json!({
        "PageNumber": 1,
        "PageSize": 50,
        "PackageCodes": package_codes,
        "Status": [0, 1, 2],
        "NeedRenewInfo": true
    });

    for ep in &endpoints {
        if let Ok(resp) = client
            .post(*ep)
            .header("Authorization", format!("Bearer {}", auth.access_token))
            .header("X-User-Id", &auth.uid)
            .header("X-Domain", &auth.domain)
            .header("User-Agent", "workbuddy-daily-credit/1.1")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Accept-Language", "zh")
            .json(&body)
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(val) = resp.json::<Value>().await {
                    if let Some(accounts) = val.pointer("/data/Accounts").and_then(Value::as_array)
                    {
                        for acc in accounts {
                            let code = acc.get("PackageCode").and_then(Value::as_str).unwrap_or("");
                            if code.is_empty() {
                                continue;
                            }
                            let pkg_name = acc
                                .get("PackageName")
                                .and_then(Value::as_str)
                                .map(String::from);
                            let sub_name = acc
                                .get("SubProductName")
                                .and_then(Value::as_str)
                                .map(String::from);
                            let end_time = acc
                                .get("CycleEndTime")
                                .and_then(Value::as_str)
                                .map(String::from);
                            let cap_type = acc.get("CapacityType").and_then(Value::as_i64);

                            let end_ts = end_time.as_deref().and_then(parse_workbuddy_datetime);

                            let entry = map.entry(code.to_string()).or_default();
                            if entry.package_name.is_none() && pkg_name.is_some() {
                                entry.package_name = pkg_name;
                            }
                            if entry.sub_product_name.is_none() && sub_name.is_some() {
                                entry.sub_product_name = sub_name;
                            }
                            if entry.capacity_type.is_none() && cap_type.is_some() {
                                entry.capacity_type = cap_type;
                            }
                            // 优先记录最早或最有效的有效截止时间
                            if let Some(ts) = end_ts {
                                if entry.end_time_ts.is_none() || ts < entry.end_time_ts.unwrap() {
                                    entry.end_time_ts = Some(ts);
                                    entry.cycle_end_time = end_time;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    map
}

fn resolve_package_presentation(
    code: &str,
    detail: Option<&PackageDetailInfo>,
    is_single: bool,
) -> (
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
) {
    let raw_name = detail
        .and_then(|d| d.package_name.as_deref())
        .or_else(|| detail.and_then(|d| d.sub_product_name.as_deref()))
        .unwrap_or("");

    let end_time = detail.and_then(|d| d.cycle_end_time.clone());
    let end_ts = detail.and_then(|d| d.end_time_ts);

    let now_sec = chrono_now_ms() / 1000;
    let remaining_days = end_ts.map(|ts| (ts - now_sec).max(0) / 86400);

    // 根据 PackageCode 前缀与包名智能分类
    if code.starts_with("TCACA_code_008")
        || raw_name.contains("个人体验版")
        || raw_name.contains("月度")
    {
        let label = "个人版月度额度 (按月重置)".to_string();
        let cycle_type = Some("月度周期 (月末自动重置)".to_string());
        return (label, cycle_type, end_time, end_ts, remaining_days);
    }

    if code.starts_with("TCACA_code_007")
        || raw_name.contains("裂变")
        || raw_name.contains("加赠")
        || raw_name.contains("赠送")
    {
        let label = if raw_name.contains("裂变") {
            "运营裂变加赠包 (赠送额度)".to_string()
        } else {
            "营销活动加赠包 (赠送额度)".to_string()
        };
        let cycle_type = Some("一次性加赠 (到期清空)".to_string());
        return (label, cycle_type, end_time, end_ts, remaining_days);
    }

    let clean_label = if !raw_name.is_empty() {
        raw_name
            .replace("CodeBuddy", "")
            .replace("WorkBuddy", "")
            .trim()
            .to_string()
    } else if is_single {
        "可用积分 (Credits)".to_string()
    } else {
        format!("积分包 ({})", &code[..code.len().min(14)])
    };

    let cycle_type = Some("额度包".to_string());
    (clean_label, cycle_type, end_time, end_ts, remaining_days)
}

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

    let auth = match resolve_workbuddy_auth(account) {
        Some(a) => a,
        None => {
            // 本地 fallback 探测（兼容用户可能在 ~/.workbuddy/models.json 放置的手动配置）
            if try_load_local_fallback(&mut quota) {
                return quota;
            }

            quota.error_message = Some(
                "未检测到有效的 WorkBuddy 登录凭据，请启动并登录 WorkBuddy 客户端".to_string(),
            );
            return quota;
        }
    };

    // 如果获取到了昵称或账号类型，丰富 plan 名称
    let plan_title = match (&auth.nickname, &auth.account_type) {
        (Some(nick), Some(typ)) if typ == "enterprise" => format!("WorkBuddy 企业版 ({})", nick),
        (Some(nick), _) => format!("WorkBuddy 个人版 ({})", nick),
        (None, Some(typ)) if typ == "enterprise" => "WorkBuddy 企业版".to_string(),
        _ => "WorkBuddy 个人版".to_string(),
    };
    quota.plan = Some(plan_title);

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    let api_url = "https://copilot.tencent.com/billing/meter/get-user-resource-summary";
    let resp = match client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", auth.access_token))
        .header("X-User-Id", &auth.uid)
        .header("X-Domain", &auth.domain)
        .header("User-Agent", "workbuddy-daily-credit/1.1")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&serde_json::json!({}))
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            if try_load_local_fallback(&mut quota) {
                return quota;
            }
            quota.error_message = Some(format!("请求腾讯 WorkBuddy 接口失败: {}", e));
            return quota;
        }
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        quota.error_message =
            Some("WorkBuddy 登录凭据已失效或已过期，请重新登录 WorkBuddy 客户端".to_string());
        return quota;
    }

    if !status.is_success() {
        quota.error_message = Some(format!("WorkBuddy 接口返回异常状态码: HTTP {}", status));
        return quota;
    }

    let val: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            quota.error_message = Some(format!("解析 WorkBuddy 返回数据失败: {}", e));
            return quota;
        }
    };

    let code = val.get("code").and_then(Value::as_i64).unwrap_or(-1);
    if code != 0 {
        let msg = val
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("未知业务错误");
        quota.error_message = Some(format!("WorkBuddy 接口返回业务错误: {}", msg));
        return quota;
    }

    let packages = val.pointer("/data/Packages").and_then(Value::as_array);

    match packages {
        Some(pkgs) if !pkgs.is_empty() => {
            let is_single = pkgs.len() == 1;
            let mut total_remain = 0.0;
            let mut total_cap = 0.0;

            let pkg_codes: Vec<String> = pkgs
                .iter()
                .filter_map(|p| p.get("PackageCode").and_then(Value::as_str))
                .map(String::from)
                .collect();

            let details_map = fetch_package_details_map(&client, &auth, &pkg_codes).await;

            for pkg in pkgs {
                let status = pkg.get("Status").and_then(Value::as_str).unwrap_or("");
                if !status.is_empty() && status != "active" {
                    continue;
                }

                let remain = parse_number_or_str(pkg.get("CycleRemainCapacity")).unwrap_or(0.0);
                let total = parse_number_or_str(pkg.get("CycleTotalCapacity")).unwrap_or(0.0);
                let code = pkg.get("PackageCode").and_then(Value::as_str).unwrap_or("");

                let (label, cycle_type, expires_at, expires_at_timestamp, remaining_days) =
                    resolve_package_presentation(code, details_map.get(code), is_single);

                total_remain += remain;
                total_cap += total;

                quota.quotas.push(QuotaKind::Credits {
                    label: Some(label),
                    remaining: (remain * 100.0).round() / 100.0,
                    total: if total > 0.0 {
                        Some((total * 100.0).round() / 100.0)
                    } else {
                        None
                    },
                    expires_at,
                    expires_at_timestamp,
                    remaining_days,
                    cycle_type,
                    unit: Some("点".to_string()),
                });
            }

            quota.is_healthy = true;

            // 计算配速状态
            if total_cap > 0.0 {
                let used_percent =
                    ((total_cap - total_remain) / total_cap * 100.0).clamp(0.0, 100.0);
                let (level, msg) = if total_remain <= 0.0 {
                    (
                        PaceLevel::OverPace,
                        "积分已耗尽，请及时充值或领取额度".to_string(),
                    )
                } else if total_remain / total_cap < 0.15 {
                    (
                        PaceLevel::Tight,
                        format!("当前剩余积分低于 15%（共剩余 {:.1} 点）", total_remain),
                    )
                } else {
                    (
                        PaceLevel::OnPace,
                        format!(
                            "积分充足，当前剩余 {:.2} / {:.0} 点",
                            total_remain, total_cap
                        ),
                    )
                };

                quota.pace = Some(PaceStatus {
                    level,
                    projected_usage_percent: Some(used_percent.round()),
                    message: msg,
                });
            }
        }
        _ => {
            // 没有 packages
            quota.is_healthy = true;
            quota.quotas.push(QuotaKind::Credits {
                label: Some("可用积分 (Credits)".to_string()),
                remaining: 0.0,
                total: None,
                expires_at: None,
                expires_at_timestamp: None,
                remaining_days: None,
                cycle_type: None,
                unit: Some("点".to_string()),
            });
        }
    }

    quota
}

fn try_load_local_fallback(quota: &mut ProviderQuota) -> bool {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    if home.is_empty() {
        return false;
    }

    let wb_dir = Path::new(&home).join(".workbuddy");
    let fallback_files = ["models.json", "credits.json"];
    for file in &fallback_files {
        let p = wb_dir.join(file);
        if p.exists() {
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                    let credits_remaining = json_val
                        .get("credits")
                        .or_else(|| json_val.get("remaining"))
                        .and_then(Value::as_f64);
                    let credits_total = json_val
                        .get("total")
                        .or_else(|| json_val.get("totalCredits"))
                        .and_then(Value::as_f64);
                    if let Some(plan) = json_val.get("plan").and_then(Value::as_str) {
                        quota.plan = Some(plan.to_string());
                    }

                    if let Some(remaining) = credits_remaining {
                        quota.is_healthy = true;
                        quota.quotas.push(QuotaKind::Credits {
                            label: Some("可用积分 (Credits)".to_string()),
                            remaining,
                            total: credits_total,
                            expires_at: None,
                            expires_at_timestamp: None,
                            remaining_days: None,
                            cycle_type: None,
                            unit: Some("点".to_string()),
                        });
                        return true;
                    }
                }
            }
        }
    }

    false
}
