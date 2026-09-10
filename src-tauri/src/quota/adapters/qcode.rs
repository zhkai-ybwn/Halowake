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

pub fn has_qcode_credentials() -> bool {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    if !home.is_empty()
        && (Path::new(&home).join(".qoder-cn").exists()
            || Path::new(&home).join(".tongyi").exists()
            || Path::new(&home).join(".lingma").exists())
    {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = env::var("APPDATA") {
            if PathBuf::from(app_data)
                .join("com.qodercn.app.stable")
                .exists()
            {
                return true;
            }
        }
    }

    false
}

#[derive(Debug, Clone)]
struct QcodeAuth {
    token: String,
    user_name: Option<String>,
}

#[cfg(target_os = "windows")]
mod dpapi {
    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptUnprotectData(
            p_data_in: *const DataBlob,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *mut std::ffi::c_void,
            p_prompt_struct: *mut std::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }

    pub fn decrypt(data: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let data_in = DataBlob {
                cb_data: data.len() as u32,
                pb_data: data.as_ptr() as *mut u8,
            };
            let mut data_out = DataBlob {
                cb_data: 0,
                pb_data: std::ptr::null_mut(),
            };
            if CryptUnprotectData(
                &data_in,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                &mut data_out,
            ) != 0
            {
                let slice = std::slice::from_raw_parts(data_out.pb_data, data_out.cb_data as usize);
                let vec = slice.to_vec();
                LocalFree(data_out.pb_data as *mut std::ffi::c_void);
                Some(vec)
            } else {
                None
            }
        }
    }
}

fn decrypt_aes_gcm(master_key: &[u8], nonce_bytes: &[u8], ciphertext: &[u8]) -> Option<Vec<u8>> {
    use ring::aead::{LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

    let unbound_key = UnboundKey::new(&AES_256_GCM, master_key).ok()?;
    let key = LessSafeKey::new(unbound_key);
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes).ok()?;

    let mut in_out = ciphertext.to_vec();
    let decrypted = key
        .open_in_place(nonce, ring::aead::Aad::empty(), &mut in_out)
        .ok()?;
    Some(decrypted.to_vec())
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
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

fn resolve_qcode_auth_from_client() -> Option<QcodeAuth> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = env::var("APPDATA") {
            let base_dir = PathBuf::from(app_data).join("com.qodercn.app.stable");
            let local_state_path = base_dir.join("Local State");
            let auth_dat_path = base_dir.join("auth.v1.dat");

            if local_state_path.exists() && auth_dat_path.exists() {
                if let (Ok(local_state_str), Ok(auth_dat_bytes)) = (
                    fs::read_to_string(&local_state_path),
                    fs::read(&auth_dat_path),
                ) {
                    if let Ok(local_state_json) = serde_json::from_str::<Value>(&local_state_str) {
                        let enc_key_b64 = local_state_json
                            .pointer("/os_crypt/encrypted_key")
                            .and_then(Value::as_str);

                        if let Some(enc_key_str) = enc_key_b64 {
                            if let Some(enc_key_bytes) = base64_decode(enc_key_str) {
                                // 前5字节为 b"DPAPI"
                                if enc_key_bytes.len() > 5 {
                                    if let Some(master_key) = dpapi::decrypt(&enc_key_bytes[5..]) {
                                        // auth.v1.dat: 前3字节为 b"v10"，接下来12字节为 Nonce，剩余为 ciphertext + 16字节 tag
                                        if auth_dat_bytes.len() > 15
                                            && auth_dat_bytes.starts_with(b"v10")
                                        {
                                            let nonce = &auth_dat_bytes[3..15];
                                            let ciphertext = &auth_dat_bytes[15..];
                                            if let Some(decrypted) =
                                                decrypt_aes_gcm(&master_key, nonce, ciphertext)
                                            {
                                                if let Ok(auth_json) =
                                                    serde_json::from_slice::<Value>(&decrypted)
                                                {
                                                    let token = auth_json
                                                        .get("token")
                                                        .or_else(|| auth_json.get("accessToken"))
                                                        .or_else(|| auth_json.get("deviceToken"))
                                                        .or_else(|| auth_json.get("device_token"))
                                                        .and_then(Value::as_str);
                                                    let user_name = auth_json
                                                        .pointer("/user/name")
                                                        .and_then(Value::as_str)
                                                        .map(String::from);

                                                    if let Some(t) = token {
                                                        return Some(QcodeAuth {
                                                            token: t.to_string(),
                                                            user_name,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn format_epoch_seconds_to_date(epoch_sec: i64) -> String {
    let days = epoch_sec / 86400;
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
            let mut m = 1;
            for md in month_days {
                if d < md {
                    return format!("{:04}-{:02}-{:02}", y, m, d + 1);
                }
                d -= md;
                m += 1;
            }
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    format!("{:04}", y)
}

#[derive(Debug, Clone, PartialEq)]
struct QcodeQuotaPool {
    label: &'static str,
    total: f64,
    used: f64,
    remaining: f64,
    unit: String,
}

#[derive(Debug, Clone, PartialEq)]
struct QcodeUsage {
    user_type: Option<String>,
    expires_at: Option<i64>,
    is_quota_exceeded: bool,
    pools: Vec<QcodeQuotaPool>,
}

fn value_as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
    })
}

fn value_as_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_f64().map(|number| number as i64))
            .or_else(|| value.as_str().and_then(|text| text.parse::<i64>().ok()))
    })
}

fn parse_qcode_pool(
    value: Option<&Value>,
    label: &'static str,
) -> Result<Option<QcodeQuotaPool>, String> {
    let Some(pool) = value.and_then(Value::as_object) else {
        return Ok(None);
    };
    let total = value_as_f64(pool.get("total")).unwrap_or(0.0);
    let used = value_as_f64(pool.get("used")).unwrap_or(0.0);
    let remaining = value_as_f64(pool.get("remaining")).unwrap_or_else(|| (total - used).max(0.0));
    if !total.is_finite()
        || !used.is_finite()
        || !remaining.is_finite()
        || total < 0.0
        || used < 0.0
        || remaining < 0.0
    {
        return Err(format!("{label}额度包含无效数值"));
    }
    if total == 0.0 && used == 0.0 && remaining == 0.0 {
        return Ok(None);
    }
    let unit = pool
        .get("unit")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|unit| !unit.is_empty())
        .unwrap_or("Credits")
        .to_string();
    Ok(Some(QcodeQuotaPool {
        label,
        total,
        used,
        remaining,
        unit,
    }))
}

fn parse_qcode_usage(value: &Value) -> Result<QcodeUsage, String> {
    let payload = value.get("data").unwrap_or(value);
    let mut pools = Vec::new();
    if let Some(pool) = parse_qcode_pool(
        payload
            .get("userQuota")
            .or_else(|| payload.get("user_quota")),
        "基础额度",
    )? {
        pools.push(pool);
    }
    if let Some(pool) = parse_qcode_pool(
        payload
            .get("addOnQuota")
            .or_else(|| payload.get("add_on_quota")),
        "赠送/签到额度",
    )? {
        pools.push(pool);
    }
    if pools.is_empty() {
        return Err("Qoder 额度响应中没有可用的 userQuota/addOnQuota".to_string());
    }
    let raw_expiry = value_as_i64(
        payload
            .get("expiresAt")
            .or_else(|| payload.get("expires_at")),
    );
    let expires_at = raw_expiry
        .map(|timestamp| {
            if timestamp > 10_000_000_000 {
                timestamp / 1000
            } else {
                timestamp
            }
        })
        .filter(|timestamp| *timestamp > 0 && *timestamp < 4_102_444_800);
    Ok(QcodeUsage {
        user_type: payload
            .get("userType")
            .or_else(|| payload.get("user_type"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(String::from),
        expires_at,
        is_quota_exceeded: payload
            .get("isQuotaExceeded")
            .or_else(|| payload.get("is_quota_exceeded"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        pools,
    })
}

async fn exchange_personal_token(client: &Client, token: &str) -> Result<String, String> {
    if !token.starts_with("pt-") {
        return Ok(token.to_string());
    }
    let response = client
        .post("https://openapi.qoder.com.cn/api/v1/jobToken/exchange")
        .header("Accept", "application/json")
        .json(&serde_json::json!({ "personal_token": token }))
        .send()
        .await
        .map_err(|error| format!("Qoder PAT 交换失败: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Qoder PAT 被拒绝 ({status})"));
    }
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| format!("解析 Qoder PAT 交换响应失败: {error}"))?;
    value
        .get("token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
        .ok_or_else(|| "Qoder PAT 交换响应缺少 job token".to_string())
}

pub async fn fetch_qcode_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Qcode,
        name: account.name.clone(),
        plan: Some("阿里灵码 (Qoder CN)".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Qcode
            .default_dashboard_url()
            .map(String::from),
    };

    // 1. 尝试获取 Token（优先用户手动填写，其次从本地客户端安全存储解密）
    let auth = if let Some(key) = &account.api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            Some(QcodeAuth {
                token: trimmed.to_string(),
                user_name: None,
            })
        } else {
            resolve_qcode_auth_from_client()
        }
    } else {
        resolve_qcode_auth_from_client()
    };

    let auth = match auth {
        Some(a) => a,
        None => {
            // 本地 fallback: 检查 ~/.qoder-cn/.qoder-app-status.json
            let home = env::var("USERPROFILE")
                .or_else(|_| env::var("HOME"))
                .unwrap_or_default();
            if !home.is_empty() {
                let status_path = Path::new(&home)
                    .join(".qoder-cn")
                    .join(".qoder-app-status.json");
                if status_path.exists() {
                    if let Ok(content) = fs::read_to_string(&status_path) {
                        if let Ok(status_json) = serde_json::from_str::<Value>(&content) {
                            if status_json
                                .get("logged_in")
                                .and_then(Value::as_bool)
                                .unwrap_or(false)
                            {
                                let name = status_json
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or("已登录用户");
                                quota.plan = Some(format!("阿里灵码 ({})", name));
                                quota.error_message = Some(
                                    "已检测到 Qoder 登录状态，但未找到可用于额度接口的本地凭据"
                                        .to_string(),
                                );
                                return quota;
                            }
                        }
                    }
                }
            }

            quota.error_message = Some(
                "未检测到阿里灵码 (Qoder CN) 登录凭据，请启动并登录客户端，或在设置中配置 Token"
                    .to_string(),
            );
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

    let token = match exchange_personal_token(&client, auth.token.trim()).await {
        Ok(token) => token,
        Err(error) => {
            quota.error_message = Some(error);
            return quota;
        }
    };
    let authorization = format!("Bearer {token}");
    let (usage_result, plan_result) = tokio::join!(
        client
            .get("https://openapi.qoder.com.cn/api/v2/quota/usage")
            .header("Authorization", authorization.clone())
            .header("User-Agent", "QoderCN/0.2.2")
            .header("Accept", "application/json")
            .send(),
        client
            .get("https://openapi.qoder.com.cn/api/v2/user/plan")
            .header("Authorization", authorization)
            .header("User-Agent", "QoderCN/0.2.2")
            .header("Accept", "application/json")
            .send()
    );
    let response = match usage_result {
        Ok(response) => response,
        Err(error) => {
            quota.error_message = Some(format!("请求 Qoder 额度接口失败: {error}"));
            return quota;
        }
    };
    let status = response.status();
    if !status.is_success() {
        quota.error_message = Some(if matches!(status.as_u16(), 401 | 403) {
            "Qoder 登录凭据已失效，请重新打开并登录 Qoder".to_string()
        } else {
            format!("Qoder 额度接口返回异常 ({status})")
        });
        return quota;
    }
    let value = match response.json::<Value>().await {
        Ok(value) => value,
        Err(error) => {
            quota.error_message = Some(format!("解析 Qoder 额度响应失败: {error}"));
            return quota;
        }
    };
    let usage = match parse_qcode_usage(&value) {
        Ok(usage) => usage,
        Err(error) => {
            quota.error_message = Some(error);
            return quota;
        }
    };
    let plan_value = match plan_result {
        Ok(response) if response.status().is_success() => response.json::<Value>().await.ok(),
        _ => None,
    };
    let plan_name = plan_value
        .as_ref()
        .and_then(|value| value.get("plan_tier_name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
        .or(usage.user_type.clone())
        .unwrap_or_else(|| "Qoder CN".to_string());
    quota.plan = Some(match auth.user_name {
        Some(name) if !name.trim().is_empty() => format!("{plan_name} · {name}"),
        _ => plan_name,
    });
    let now_sec = chrono_now_ms() / 1000;
    let expires_at = usage.expires_at.map(format_epoch_seconds_to_date);
    let remaining_days = usage
        .expires_at
        .map(|timestamp| ((timestamp - now_sec) / 86_400).max(0));
    for pool in &usage.pools {
        quota.quotas.push(QuotaKind::Credits {
            label: Some(pool.label.to_string()),
            remaining: pool.remaining,
            total: Some(pool.total),
            expires_at: expires_at.clone(),
            expires_at_timestamp: usage.expires_at,
            remaining_days,
            cycle_type: Some("subscription".to_string()),
            unit: Some(pool.unit.clone()),
        });
    }
    let total = usage.pools.iter().map(|pool| pool.total).sum::<f64>();
    let used = usage.pools.iter().map(|pool| pool.used).sum::<f64>();
    if total > 0.0 {
        let used_percent = if usage.is_quota_exceeded {
            100.0
        } else {
            (used / total * 100.0).clamp(0.0, 100.0)
        };
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

#[cfg(test)]
mod tests {
    use super::parse_qcode_usage;
    use serde_json::json;

    #[test]
    fn parses_base_and_add_on_credits() {
        let usage = parse_qcode_usage(&json!({
            "userType": "personal_professional",
            "expiresAt": 1_800_000_000_000_i64,
            "isQuotaExceeded": false,
            "userQuota": { "total": 2000, "used": 750, "remaining": 1250, "unit": "credits" },
            "addOnQuota": { "total": 300, "used": 20, "remaining": 280 }
        }))
        .expect("parse quota");

        assert_eq!(usage.pools.len(), 2);
        assert_eq!(usage.pools[0].remaining, 1250.0);
        assert_eq!(usage.pools[1].remaining, 280.0);
        assert_eq!(usage.expires_at, Some(1_800_000_000));
    }

    #[test]
    fn rejects_empty_quota_payload() {
        assert!(parse_qcode_usage(&json!({ "userQuota": {} })).is_err());
    }
}
