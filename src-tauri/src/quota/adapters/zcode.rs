use reqwest::Client;
use serde_json::Value;
use std::{env, fs, path::PathBuf, time::Duration};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{
    AccountConfig, PaceLevel, PaceStatus, ProviderQuota, ProviderType, QuotaKind,
};

pub fn has_zcode_credentials() -> bool {
    let cred_path = get_zcode_credentials_path();
    if cred_path.exists() {
        return true;
    }
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    if !home.is_empty() {
        let zcode_dir = PathBuf::from(home).join(".zcode");
        if zcode_dir.exists() {
            return true;
        }
    }
    false
}

fn get_zcode_credentials_path() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    PathBuf::from(home)
        .join(".zcode")
        .join("v2")
        .join("credentials.json")
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

fn decrypt_zcode_value(encrypted_val: &str) -> Option<String> {
    if !encrypted_val.starts_with("enc:v1:") {
        return Some(encrypted_val.to_string());
    }

    let raw = &encrypted_val["enc:v1:".len()..];
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let iv = base64_url_decode(parts[0])?;
    let tag = base64_url_decode(parts[1])?;
    let ciphertext = base64_url_decode(parts[2])?;

    let platform = if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };

    let homedir = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    let username = env::var("USERNAME")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| "user".to_string());

    let secret = env::var("ZCODE_CREDENTIAL_SECRET").unwrap_or_else(|_| {
        format!(
            "zcode-credential-fallback:{}:{}:{}",
            platform, homedir, username
        )
    });

    let key_digest = ring::digest::digest(&ring::digest::SHA256, secret.as_bytes());

    use ring::aead::{LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    let unbound_key = UnboundKey::new(&AES_256_GCM, key_digest.as_ref()).ok()?;
    let key = LessSafeKey::new(unbound_key);
    let nonce = Nonce::try_assume_unique_for_key(&iv).ok()?;

    let mut in_out = ciphertext;
    in_out.extend_from_slice(&tag);

    let decrypted = key
        .open_in_place(nonce, ring::aead::Aad::empty(), &mut in_out)
        .ok()?;
    String::from_utf8(decrypted.to_vec()).ok()
}

fn format_timestamp_to_date(ts_sec: i64) -> String {
    let days = ts_sec / 86400;
    let mut y = 1970;
    let mut d = days;
    loop {
        let leap = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            1
        } else {
            0
        };
        let days_in_year = 365 + leap;
        if d < days_in_year {
            let month_days = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            for (idx, &md) in month_days.iter().enumerate() {
                if d < md {
                    return format!("{:04}-{:02}-{:02}", y, idx + 1, d + 1);
                }
                d -= md;
            }
            return format!("{:04}-12-31", y);
        }
        d -= days_in_year;
        y += 1;
    }
}

pub async fn fetch_zcode_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Zcode,
        name: account.name.clone(),
        plan: Some("ZCode Start Plan".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Zcode
            .default_dashboard_url()
            .map(String::from),
    };

    let creds_file = get_zcode_credentials_path();
    if !creds_file.exists() {
        quota.error_message =
            Some("未检测到 ~/.zcode 本地凭据，请启动并登录 Z-Code 客户端".to_string());
        return quota;
    }

    let content = match fs::read_to_string(&creds_file) {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("读取 Z-Code 凭据文件失败: {}", e));
            return quota;
        }
    };

    let json_val: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            quota.error_message = Some(format!("解析 Z-Code 凭据失败: {}", e));
            return quota;
        }
    };

    let raw_jwt = json_val.get("zcodejwttoken").and_then(Value::as_str);
    let jwt_token = raw_jwt.and_then(decrypt_zcode_value);

    // 解析用户信息（若存在）
    let mut display_name = None;
    if let Some(user_info_raw) = json_val
        .get("oauth:bigmodel:user_info")
        .and_then(Value::as_str)
    {
        if let Some(decrypted_user) = decrypt_zcode_value(user_info_raw) {
            if let Ok(u_json) = serde_json::from_str::<Value>(&decrypted_user) {
                display_name = u_json
                    .get("displayName")
                    .or_else(|| u_json.get("username"))
                    .and_then(Value::as_str)
                    .map(String::from);
            }
        }
    }

    let token_str = match &jwt_token {
        Some(t) if !t.is_empty() => t,
        _ => {
            // 本地 fallback：检测 coding-plan-cache.json
            let home = env::var("USERPROFILE")
                .or_else(|_| env::var("HOME"))
                .unwrap_or_default();
            let cache_path = PathBuf::from(home)
                .join(".zcode")
                .join("v2")
                .join("coding-plan-cache.json");
            if cache_path.exists() {
                quota.is_healthy = true;
                quota.plan = Some(match display_name {
                    Some(name) => format!("ZCode Start Plan ({})", name),
                    None => "ZCode Start Plan".to_string(),
                });
                quota.error_message = Some(
                    "已检测到 Z-Code 套餐缓存，但凭据不可用，无法确认实时剩余额度".to_string(),
                );
                return quota;
            }

            quota.error_message = Some("Z-Code 凭证解密失败或尚未登录".to_string());
            return quota;
        }
    };

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            quota.error_message = Some(format!("初始化 HTTP 客户端失败: {}", e));
            return quota;
        }
    };

    // The balance endpoint validates this query/header pair. Match ZCode's own
    // environment override and retain a current protocol fallback for CLI-only
    // installs where the desktop executable cannot be inspected portably.
    let app_version = env::var("ZCODE_APP_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "3.11.2".to_string());
    let platform = if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    let os_category = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        env::consts::ARCH
    };
    let request_id = format!("lumina-{}-{}", std::process::id(), chrono_now_ms());
    let mut request = client
        .get("https://zcode.z.ai/api/v1/zcode-plan/billing/balance")
        .query(&[("app_version", app_version.as_str())])
        .header("Authorization", format!("Bearer {}", token_str))
        .header("Accept", "application/json")
        .header("User-Agent", format!("ZCode/{app_version}"))
        .header("HTTP-Referer", "https://zcode.z.ai")
        .header("X-ZCode-App-Version", &app_version)
        .header("X-Title", "Z Code@electron")
        .header("X-Platform", format!("{platform}-{arch}"))
        .header("X-Release-Channel", "stable")
        .header("X-Client-Language", "zh-CN")
        .header("X-Client-Timezone", "Asia/Shanghai")
        .header("X-Os-Category", os_category)
        .header("X-Os-Version", "10.0.0")
        .header("x-request-id", request_id);
    let telemetry_path = creds_file
        .parent()
        .map(|parent| parent.join("telemetry-state.json"));
    if let Some(device_mid) = telemetry_path
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|value| {
            value
                .get("deviceMid")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .filter(|value| !value.trim().is_empty())
    {
        request = request.header("X-Device-Mid", device_mid);
    }
    let resp = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            quota.error_message = Some(format!("请求 ZCode 计费接口失败: {}", e));
            return quota;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(240)
            .collect::<String>();
        quota.error_message = Some(if detail.trim().is_empty() {
            format!("ZCode API 返回异常 ({status})")
        } else {
            format!("ZCode API 返回异常 ({status}): {detail}")
        });
        return quota;
    }

    let val: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            quota.error_message = Some(format!("解析 ZCode 计费返回失败: {}", e));
            return quota;
        }
    };

    if val
        .get("code")
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 0 && code != 200)
    {
        quota.error_message = Some("ZCode 计费接口返回业务错误".to_string());
        return quota;
    }
    let balances = val.pointer("/data/balances").and_then(Value::as_array);
    let now_sec = chrono_now_ms() / 1000;
    let Some(balances) = balances.filter(|items| !items.is_empty()) else {
        quota.error_message = Some("ZCode 计费响应中没有余额池".to_string());
        return quota;
    };
    let mut aggregate_used = 0.0;
    let mut aggregate_total = 0.0;
    let mut first_plan_id = None;
    for balance in balances {
        let Some(total) = balance
            .get("total_units")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value > 0.0)
        else {
            continue;
        };
        let Some(used) = balance
            .get("used_units")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
        else {
            continue;
        };
        let show_name = balance
            .get("show_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("ZCode 配额");
        if first_plan_id.is_none() {
            first_plan_id = balance
                .get("plan_id")
                .and_then(Value::as_str)
                .map(String::from);
        }
        let raw_period_end = balance
            .get("period_end")
            .or_else(|| balance.get("expires_at"))
            .and_then(Value::as_i64);
        let period_end = raw_period_end.map(|value| {
            if value > 100_000_000_000 {
                value / 1000
            } else {
                value
            }
        });
        let remaining_days = period_end.map(|value| (value - now_sec).max(0) / 86_400);
        quota.quotas.push(QuotaKind::Credits {
            label: Some(show_name.to_string()),
            remaining: (total - used).max(0.0),
            total: Some(total),
            expires_at: period_end.map(format_timestamp_to_date),
            expires_at_timestamp: period_end,
            remaining_days,
            cycle_type: Some("month".to_string()),
            unit: Some("Units".to_string()),
        });
        aggregate_used += used;
        aggregate_total += total;
    }
    if quota.quotas.is_empty() {
        quota.error_message = Some("ZCode 余额池缺少有效的 total_units/used_units".to_string());
        return quota;
    }
    let plan_name = first_plan_id.unwrap_or_else(|| "ZCode Start Plan".to_string());
    quota.plan = Some(match display_name {
        Some(name) => format!("{plan_name} · {name}"),
        None => plan_name,
    });
    if aggregate_total > 0.0 {
        let used_percent = (aggregate_used / aggregate_total * 100.0).clamp(0.0, 100.0);
        let used_round = used_percent.round() as i64;
        let level = if used_round >= 90 {
            PaceLevel::OverPace
        } else if used_round >= 70 {
            PaceLevel::Tight
        } else {
            PaceLevel::OnPace
        };
        quota.pace = Some(PaceStatus {
            level,
            projected_usage_percent: Some(used_round as f64),
            message: format!("已使用 {used_round}%"),
        });
    }
    quota.is_healthy = true;
    quota
}
