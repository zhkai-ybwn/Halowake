use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

use crate::quota::adapters::{
    codex::fetch_codex_quota,
    deepseek::fetch_deepseek_quota,
    gemini::fetch_gemini_quota,
    openrouter::fetch_openrouter_quota,
};
use crate::quota::discovery::discover_local_accounts;
use crate::quota::models::{
    AccountConfig, ProviderQuota, ProviderType, QuotaKind, QuotaSummary,
};

const QUOTA_ACCOUNTS_FILE: &str = "ai-quota-accounts.json";

fn accounts_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| format!("解析应用配置目录失败: {}", e))?
        .join(QUOTA_ACCOUNTS_FILE))
}

use crate::storage::{
    history_repository::{load_quota_accounts_from_db, save_quota_accounts_to_db},
    AppDatabase,
};

pub fn load_accounts_config(app: &AppHandle) -> Result<Vec<AccountConfig>, String> {
    if let Some(db) = app.try_state::<AppDatabase>() {
        if let Ok(accounts) = load_quota_accounts_from_db(&db) {
            if !accounts.is_empty() {
                return Ok(accounts);
            }
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
        let _ = save_quota_accounts_to_db(&db, &accounts);
        let _ = fs::remove_file(&path);
    }

    Ok(accounts)
}

pub fn save_accounts_config(app: &AppHandle, accounts: &[AccountConfig]) -> Result<(), String> {
    if let Some(db) = app.try_state::<AppDatabase>() {
        save_quota_accounts_to_db(&db, accounts)?;
    }

    // 移除旧明文 JSON 文件，统一收口至 SQLite 数据库
    if let Ok(path) = accounts_file_path(app) {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}

pub async fn fetch_all_quotas(app: &AppHandle) -> Result<(Vec<ProviderQuota>, QuotaSummary), String> {
    let accounts = load_accounts_config(app)?;
    let mut tasks = Vec::new();

    for account in accounts {
        if !account.enabled {
            continue;
        }
        tasks.push(tokio::spawn(async move {
            match account.provider_type {
                ProviderType::Codex => fetch_codex_quota(&account).await,
                ProviderType::Deepseek => fetch_deepseek_quota(&account).await,
                ProviderType::Openrouter => fetch_openrouter_quota(&account).await,
                ProviderType::Gemini => fetch_gemini_quota(&account).await,
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

    // 计算汇总信息
    let mut total_cny = 0.0;
    let mut total_usd = 0.0;
    let mut warnings = 0;

    for q in &quotas {
        if !q.is_healthy || q.error_message.is_some() {
            warnings += 1;
        }

        for item in &q.quotas {
            if let QuotaKind::Balance { currency, total_remaining, .. } = item {
                if currency.eq_ignore_ascii_case("CNY") {
                    total_cny += total_remaining;
                } else if currency.eq_ignore_ascii_case("USD") {
                    total_usd += total_remaining;
                }
            }
        }
    }

    let summary = QuotaSummary {
        total_cny_balance: total_cny,
        total_usd_balance: total_usd,
        active_accounts_count: quotas.len(),
        warning_accounts_count: warnings,
    };

    Ok((quotas, summary))
}

pub async fn fetch_single_quota(account: AccountConfig) -> ProviderQuota {
    match account.provider_type {
        ProviderType::Codex => fetch_codex_quota(&account).await,
        ProviderType::Deepseek => fetch_deepseek_quota(&account).await,
        ProviderType::Openrouter => fetch_openrouter_quota(&account).await,
        ProviderType::Gemini => fetch_gemini_quota(&account).await,
        ProviderType::Custom => fetch_deepseek_quota(&account).await,
    }
}
