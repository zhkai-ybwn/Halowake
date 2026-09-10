use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

use crate::quota::adapters::{
    claude::fetch_claude_quota,
    codex::fetch_codex_quota,
    cursor::fetch_cursor_quota,
    deepseek::{chrono_now_ms, fetch_deepseek_quota},
    gemini::fetch_gemini_quota,
    moonshot::fetch_moonshot_quota,
    opencode::fetch_opencode_quota,
    openrouter::fetch_openrouter_quota,
    qcode::fetch_qcode_quota,
    siliconflow::fetch_siliconflow_quota,
    trae::fetch_trae_quota,
    workbuddy::fetch_workbuddy_quota,
    zcode::fetch_zcode_quota,
    zhipu::fetch_zhipu_quota,
};
use crate::quota::discovery::discover_local_accounts;
use crate::quota::models::{AccountConfig, ProviderQuota, ProviderType, QuotaKind, QuotaSummary};

const QUOTA_ACCOUNTS_FILE: &str = "ai-quota-accounts.json";

fn is_aggregate_credit_unit(unit: Option<&str>) -> bool {
    let Some(unit) = unit.map(str::trim).filter(|unit| !unit.is_empty()) else {
        return true;
    };
    matches!(
        unit.to_ascii_lowercase().as_str(),
        "点" | "积分" | "credit" | "credits" | "point" | "points"
    )
}

fn unsupported_provider_quota(account: &AccountConfig, message: &str) -> ProviderQuota {
    ProviderQuota {
        id: account.id.clone(),
        account_id: account.id.clone(),
        provider_type: account.provider_type.clone(),
        name: account.name.clone(),
        plan: Some(account.provider_type.display_name().to_string()),
        quotas: Vec::new(),
        pace: None,
        reset_credits: None,
        last_updated: chrono_now_ms(),
        is_healthy: false,
        error_message: Some(message.to_string()),
        official_dashboard_url: account
            .provider_type
            .default_dashboard_url()
            .map(String::from),
    }
}

fn accounts_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| format!("解析应用配置目录失败: {}", e))?
        .join(QUOTA_ACCOUNTS_FILE))
}

use crate::storage::{
    history_repository::{
        load_quota_accounts_from_db, quota_accounts_initialized, save_quota_accounts_to_db,
    },
    AppDatabase,
};

pub fn load_accounts_config(app: &AppHandle) -> Result<Vec<AccountConfig>, String> {
    if let Some(db) = app.try_state::<AppDatabase>() {
        let accounts = load_quota_accounts_from_db(&db)?;
        if quota_accounts_initialized(&db)? || !accounts.is_empty() {
            return Ok(accounts);
        }
    }

    let path = accounts_file_path(app)?;
    if !path.exists() {
        // 如果初次使用且未保存配置，自动探测本地默认
        let discovered = discover_local_accounts(app);
        if !discovered.is_empty() {
            let _ = save_accounts_config(app, &discovered);
            return Ok(discovered);
        }
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取账号配置文件失败 {}: {}", path.display(), e))?;
    let accounts = serde_json::from_str::<Vec<AccountConfig>>(&content)
        .map_err(|e| format!("账号配置文件解析失败: {}", e))?;

    if let Some(db) = app.try_state::<AppDatabase>() {
        save_quota_accounts_to_db(&db, &accounts)?;
        fs::remove_file(&path)
            .map_err(|error| format!("清理旧账号配置文件失败 {}: {error}", path.display()))?;
    }

    Ok(accounts)
}

pub fn save_accounts_config(app: &AppHandle, accounts: &[AccountConfig]) -> Result<(), String> {
    let db = app
        .try_state::<AppDatabase>()
        .ok_or_else(|| "应用数据库未初始化，账号配置未保存".to_string())?;
    save_quota_accounts_to_db(&db, accounts)?;

    // 移除旧明文 JSON 文件，统一收口至 SQLite 数据库
    if let Ok(path) = accounts_file_path(app) {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}

pub async fn fetch_all_quotas(
    app: &AppHandle,
) -> Result<(Vec<ProviderQuota>, QuotaSummary), String> {
    let accounts = load_accounts_config(app)?;
    let mut tasks = Vec::new();

    for account in accounts {
        if !account.enabled {
            continue;
        }
        tasks.push(tokio::spawn(async move {
            match account.provider_type {
                ProviderType::Codex => fetch_codex_quota(&account).await,
                ProviderType::Claude => fetch_claude_quota(&account).await,
                ProviderType::Deepseek => fetch_deepseek_quota(&account).await,
                ProviderType::Openrouter => fetch_openrouter_quota(&account).await,
                ProviderType::Gemini => fetch_gemini_quota(&account).await,
                ProviderType::Opencode => fetch_opencode_quota(&account).await,
                ProviderType::Workbuddy => fetch_workbuddy_quota(&account).await,
                ProviderType::Siliconflow => fetch_siliconflow_quota(&account).await,
                ProviderType::Moonshot => fetch_moonshot_quota(&account).await,
                ProviderType::Zhipu => fetch_zhipu_quota(&account).await,
                ProviderType::Qwen => unsupported_provider_quota(
                    &account,
                    "通义千问暂不支持自动查询额度，请前往官方控制台查看",
                ),
                ProviderType::Minimax => unsupported_provider_quota(
                    &account,
                    "MiniMax 暂不支持自动查询额度，请前往官方控制台查看",
                ),
                ProviderType::Cursor => fetch_cursor_quota(&account).await,
                ProviderType::Qcode => fetch_qcode_quota(&account).await,
                ProviderType::Trae => fetch_trae_quota(&account).await,
                ProviderType::Zcode => fetch_zcode_quota(&account).await,
                ProviderType::Custom => fetch_deepseek_quota(&account).await,
            }
        }));
    }

    let mut quotas = Vec::new();
    for task in tasks {
        if let Ok(quota) = task.await {
            quotas.push(quota);
        }
    }

    // 计算汇总信息 (法定货币 CNY/USD + 算力积分 Credits)
    let mut total_cny = 0.0;
    let mut total_usd = 0.0;
    let mut total_credits = 0.0;
    let mut warnings = 0;

    for q in &quotas {
        if !q.is_healthy || q.error_message.is_some() {
            warnings += 1;
        }

        for item in &q.quotas {
            match item {
                QuotaKind::Balance {
                    currency,
                    total_remaining,
                    ..
                } => {
                    if currency.eq_ignore_ascii_case("CNY") {
                        total_cny += total_remaining;
                    } else if currency.eq_ignore_ascii_case("USD") {
                        total_usd += total_remaining;
                    }
                }
                QuotaKind::Credits {
                    remaining, unit, ..
                } if is_aggregate_credit_unit(unit.as_deref()) => {
                    total_credits += remaining;
                }
                _ => {}
            }
        }
    }

    let summary = QuotaSummary {
        total_cny_balance: (total_cny * 100.0).round() / 100.0,
        total_usd_balance: (total_usd * 100.0).round() / 100.0,
        total_credits: (total_credits * 100.0).round() / 100.0,
        active_accounts_count: quotas.iter().filter(|q| q.is_healthy).count(),
        warning_accounts_count: warnings,
    };

    Ok((quotas, summary))
}

pub async fn fetch_single_quota(account: AccountConfig) -> ProviderQuota {
    match account.provider_type {
        ProviderType::Codex => fetch_codex_quota(&account).await,
        ProviderType::Claude => fetch_claude_quota(&account).await,
        ProviderType::Deepseek => fetch_deepseek_quota(&account).await,
        ProviderType::Openrouter => fetch_openrouter_quota(&account).await,
        ProviderType::Gemini => fetch_gemini_quota(&account).await,
        ProviderType::Opencode => fetch_opencode_quota(&account).await,
        ProviderType::Workbuddy => fetch_workbuddy_quota(&account).await,
        ProviderType::Siliconflow => fetch_siliconflow_quota(&account).await,
        ProviderType::Moonshot => fetch_moonshot_quota(&account).await,
        ProviderType::Zhipu => fetch_zhipu_quota(&account).await,
        ProviderType::Qwen => unsupported_provider_quota(
            &account,
            "通义千问暂不支持自动查询额度，请前往官方控制台查看",
        ),
        ProviderType::Minimax => unsupported_provider_quota(
            &account,
            "MiniMax 暂不支持自动查询额度，请前往官方控制台查看",
        ),
        ProviderType::Cursor => fetch_cursor_quota(&account).await,
        ProviderType::Qcode => fetch_qcode_quota(&account).await,
        ProviderType::Trae => fetch_trae_quota(&account).await,
        ProviderType::Zcode => fetch_zcode_quota(&account).await,
        ProviderType::Custom => fetch_deepseek_quota(&account).await,
    }
}

#[cfg(test)]
mod tests {
    use super::is_aggregate_credit_unit;

    #[test]
    fn only_point_like_units_are_aggregate_credits() {
        assert!(is_aggregate_credit_unit(None));
        assert!(is_aggregate_credit_unit(Some("点")));
        assert!(is_aggregate_credit_unit(Some("Credits")));
        assert!(!is_aggregate_credit_unit(Some("Tokens / 日")));
        assert!(!is_aggregate_credit_unit(Some("次")));
        assert!(!is_aggregate_credit_unit(Some("个模型")));
    }
}
