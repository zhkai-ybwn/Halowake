use tauri::AppHandle;

use crate::quota::{
    discovery::discover_local_accounts,
    manager::{fetch_all_quotas, fetch_single_quota, load_accounts_config, save_accounts_config},
    models::{AccountConfig, ProviderQuota, QuotaSummary},
};

#[tauri::command]
pub async fn load_all_quotas(app: AppHandle) -> Result<(Vec<ProviderQuota>, QuotaSummary), String> {
    fetch_all_quotas(&app).await
}

#[tauri::command]
pub async fn refresh_single_quota(account: AccountConfig) -> Result<ProviderQuota, String> {
    Ok(fetch_single_quota(account).await)
}

#[tauri::command]
pub fn load_quota_accounts(app: AppHandle) -> Result<Vec<AccountConfig>, String> {
    load_accounts_config(&app)
}

#[tauri::command]
pub fn save_quota_accounts(app: AppHandle, accounts: Vec<AccountConfig>) -> Result<(), String> {
    save_accounts_config(&app, &accounts)
}

#[tauri::command]
pub fn discover_local_ai_accounts(app: AppHandle) -> Result<Vec<AccountConfig>, String> {
    Ok(discover_local_accounts(&app))
}
