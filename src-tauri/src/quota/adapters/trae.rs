use rusqlite::Connection;
use serde_json::Value;
use std::{env, fs, path::PathBuf};

use crate::quota::adapters::deepseek::chrono_now_ms;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType};

#[derive(Debug, Clone)]
struct TraeAccountInfo {
    user_id: Option<String>,
    username: Option<String>,
    mobile: Option<String>,
    _region: Option<String>,
    models: Vec<String>,
    is_cn: bool,
}

pub fn has_trae_credentials() -> bool {
    let candidate_dirs = get_trae_dirs();
    for dir in candidate_dirs {
        let storage = dir.join("User").join("globalStorage").join("storage.json");
        let vscdb = dir.join("User").join("globalStorage").join("state.vscdb");
        if storage.exists() || vscdb.exists() {
            return true;
        }
    }
    false
}

fn get_trae_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(app_data) = env::var("APPDATA") {
        let p = PathBuf::from(app_data);
        dirs.push(p.join("Trae CN"));
        dirs.push(p.join("Trae"));
    }
    if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
        let p = PathBuf::from(home);
        dirs.push(
            p.join("Library")
                .join("Application Support")
                .join("Trae CN"),
        );
        dirs.push(p.join("Library").join("Application Support").join("Trae"));
        dirs.push(p.join(".config").join("Trae CN"));
        dirs.push(p.join(".config").join("Trae"));
    }
    dirs
}

fn extract_trae_info() -> Option<TraeAccountInfo> {
    let candidate_dirs = get_trae_dirs();
    for dir in candidate_dirs {
        let storage_path = dir.join("User").join("globalStorage").join("storage.json");
        let vscdb_path = dir.join("User").join("globalStorage").join("state.vscdb");

        let mut user_id = None;
        let mut username = None;
        let mut mobile = None;
        let mut region = None;
        let is_cn = dir.to_string_lossy().contains("Trae CN");

        if storage_path.exists() {
            if let Ok(content) = fs::read_to_string(&storage_path) {
                if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                    if let Some(icube_val) = json_val.get("iCubeAuthInfo://icube.cloudide") {
                        let parsed_icube: Option<Value> = if icube_val.is_string() {
                            serde_json::from_str(icube_val.as_str().unwrap_or("")).ok()
                        } else {
                            Some(icube_val.clone())
                        };

                        if let Some(icube) = parsed_icube {
                            user_id = icube
                                .get("userId")
                                .and_then(Value::as_str)
                                .map(String::from);
                            if let Some(acc) = icube.get("account") {
                                username = acc
                                    .get("username")
                                    .and_then(Value::as_str)
                                    .filter(|s| !s.trim().is_empty())
                                    .map(String::from);
                                mobile = acc
                                    .get("nonPlainTextMobile")
                                    .and_then(Value::as_str)
                                    .filter(|s| !s.trim().is_empty())
                                    .map(String::from);
                                region = acc
                                    .get("storeRegion")
                                    .and_then(Value::as_str)
                                    .map(String::from);
                            }
                        }
                    }
                }
            }
        }

        // 读取 state.vscdb 中的模型信息
        let mut models = Vec::new();
        if vscdb_path.exists() {
            if let Ok(conn) = Connection::open_with_flags(
                &vscdb_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            ) {
                let mut stmt = conn
                    .prepare("SELECT key, value FROM ItemTable WHERE key LIKE '%model_list%'")
                    .ok();
                if let Some(ref mut statement) = stmt {
                    let rows = statement
                        .query_map([], |row| {
                            let _k: String = row.get(0)?;
                            let v: String = row.get(1)?;
                            Ok(v)
                        })
                        .ok();

                    if let Some(r_iter) = rows {
                        for row_val in r_iter.flatten() {
                            if let Ok(m_arr) = serde_json::from_str::<Value>(&row_val) {
                                if let Some(items) = m_arr.as_array() {
                                    for item in items {
                                        if let Some(name) = item
                                            .get("display_name")
                                            .or_else(|| item.get("name"))
                                            .and_then(Value::as_str)
                                        {
                                            let cleaned = name.trim().to_string();
                                            if !cleaned.is_empty() && !models.contains(&cleaned) {
                                                models.push(cleaned);
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

        if user_id.is_some() || username.is_some() || !models.is_empty() {
            return Some(TraeAccountInfo {
                user_id,
                username,
                mobile,
                _region: region,
                models,
                is_cn,
            });
        }
    }

    None
}

pub async fn fetch_trae_quota(account: &AccountConfig) -> ProviderQuota {
    let mut quota = ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: ProviderType::Trae,
        name: account.name.clone(),
        plan: Some("Trae AI 个人版".to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: None,
        official_dashboard_url: ProviderType::Trae.default_dashboard_url().map(String::from),
    };

    let info = match extract_trae_info() {
        Some(inf) => inf,
        None => {
            quota.error_message =
                Some("未在本地检测到 Trae 客户端或有效登录会话，请启动并登录 Trae".to_string());
            return quota;
        }
    };

    let display_user = info
        .username
        .as_deref()
        .or(info.mobile.as_deref())
        .or(info.user_id.as_deref())
        .unwrap_or("已授权");

    let edition = if info.is_cn { "Trae CN" } else { "Trae" };
    quota.plan = Some(format!("{} · {}", edition, display_user));
    quota.is_healthy = true;
    if !info.models.is_empty() {
        quota.plan = Some(format!(
            "{} · {} · {} 个本地模型配置",
            edition,
            display_user,
            info.models.len()
        ));
    }
    quota.error_message =
        Some("已检测到 Trae 登录状态，但客户端未提供可可靠读取的实时额度".to_string());

    quota
}
