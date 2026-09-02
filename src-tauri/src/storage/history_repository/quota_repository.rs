use rusqlite::params;

use crate::storage::AppDatabase;

pub fn load_quota_accounts_from_db(
    database: &AppDatabase,
) -> Result<Vec<crate::quota::models::AccountConfig>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare("SELECT account_json FROM app_quota_accounts ORDER BY sort_order ASC, updated_at DESC;")
        .map_err(|error| format!("准备查询 Quota 账号失败: {error}"))?;

    let mut rows = statement
        .query([])
        .map_err(|error| format!("查询 Quota 账号失败: {error}"))?;

    let mut accounts = Vec::new();
    while let Some(row) = rows.next().map_err(|error| format!("读取 Quota 账号行失败: {error}"))? {
        let json_str: String = row.get(0).map_err(|e| format!("解析 account_json 失败: {e}"))?;
        if let Ok(account) = serde_json::from_str(&json_str) {
            accounts.push(account);
        }
    }
    Ok(accounts)
}

pub fn save_quota_accounts_to_db(
    database: &AppDatabase,
    accounts: &[crate::quota::models::AccountConfig],
) -> Result<(), String> {
    let mut connection = database.connect()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("开启 Quota 账号事务失败: {error}"))?;

    transaction
        .execute("DELETE FROM app_quota_accounts;", [])
        .map_err(|error| format!("清理旧 Quota 账号失败: {error}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    for (index, account) in accounts.iter().enumerate() {
        let json_str = serde_json::to_string(account)
            .map_err(|error| format!("序列化 Quota 账号失败: {error}"))?;
        transaction
            .execute(
                "INSERT INTO app_quota_accounts (id, account_json, enabled, sort_order, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5);",
                params![
                    account.id,
                    json_str,
                    if account.enabled { 1 } else { 0 },
                    index as i64,
                    now,
                ],
            )
            .map_err(|error| format!("插入 Quota 账号失败: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("提交 Quota 账号事务失败: {error}"))?;
    Ok(())
}
