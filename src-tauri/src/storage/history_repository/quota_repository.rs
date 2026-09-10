use rusqlite::params;

use crate::storage::{
    secret_store::{expose_secret, is_protected, protect_secret},
    AppDatabase,
};

const QUOTA_ACCOUNTS_INITIALIZED_KEY: &str = "quota_accounts_initialized";

pub fn quota_accounts_initialized(database: &AppDatabase) -> Result<bool, String> {
    let connection = database.connect()?;
    let value = connection.query_row(
        "SELECT value FROM app_metadata WHERE key = ?1",
        [QUOTA_ACCOUNTS_INITIALIZED_KEY],
        |row| row.get::<_, String>(0),
    );
    match value {
        Ok(value) => Ok(value == "true"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(error) => Err(format!("读取 Quota 初始化状态失败: {error}")),
    }
}

pub fn load_quota_accounts_from_db(
    database: &AppDatabase,
) -> Result<Vec<crate::quota::models::AccountConfig>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare(
            "SELECT account_json FROM app_quota_accounts ORDER BY sort_order ASC, updated_at DESC;",
        )
        .map_err(|error| format!("准备查询 Quota 账号失败: {error}"))?;

    let mut rows = statement
        .query([])
        .map_err(|error| format!("查询 Quota 账号失败: {error}"))?;

    let mut accounts = Vec::new();
    let mut needs_protection = false;
    while let Some(row) = rows
        .next()
        .map_err(|error| format!("读取 Quota 账号行失败: {error}"))?
    {
        let json_str: String = row
            .get(0)
            .map_err(|e| format!("解析 account_json 失败: {e}"))?;
        if let Ok(mut account) =
            serde_json::from_str::<crate::quota::models::AccountConfig>(&json_str)
        {
            if cfg!(target_os = "windows")
                && account
                    .api_key
                    .as_deref()
                    .is_some_and(|key| !key.is_empty() && !is_protected(key))
            {
                needs_protection = true;
            }
            if let Some(key) = &mut account.api_key {
                *key = expose_secret(key)?;
            }
            accounts.push(account);
        }
    }
    drop(rows);
    drop(statement);
    drop(connection);
    if needs_protection {
        save_quota_accounts_to_db(database, &accounts)?;
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
        let mut stored = account.clone();
        if let Some(key) = &mut stored.api_key {
            *key = protect_secret(key)?;
        }
        let json_str = serde_json::to_string(&stored)
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
        .execute(
            "INSERT INTO app_metadata (key, value, updated_at) VALUES (?1, 'true', ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at;",
            params![QUOTA_ACCOUNTS_INITIALIZED_KEY, now],
        )
        .map_err(|error| format!("保存 Quota 初始化状态失败: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("提交 Quota 账号事务失败: {error}"))?;
    Ok(())
}
