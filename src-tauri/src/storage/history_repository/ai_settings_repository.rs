use rusqlite::params;

use crate::storage::AppDatabase;

pub fn load_ai_settings_from_db(
    database: &AppDatabase,
) -> Result<Option<crate::commands::ai_settings::AiSettings>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare("SELECT settings_json FROM app_ai_settings WHERE key = 'main' LIMIT 1;")
        .map_err(|error| format!("准备查询 AI 设置失败: {error}"))?;

    let mut rows = statement
        .query([])
        .map_err(|error| format!("查询 AI 设置失败: {error}"))?;

    if let Some(row) = rows.next().map_err(|error| format!("读取 AI 设置行失败: {error}"))? {
        let json_str: String = row.get(0).map_err(|e| format!("解析 settings_json 失败: {e}"))?;
        let settings = serde_json::from_str(&json_str)
            .map_err(|error| format!("反序列化 AI 设置失败: {error}"))?;
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

pub fn save_ai_settings_to_db(
    database: &AppDatabase,
    settings: &crate::commands::ai_settings::AiSettings,
) -> Result<(), String> {
    let connection = database.connect()?;
    let json_str = serde_json::to_string(settings)
        .map_err(|error| format!("序列化 AI 设置失败: {error}"))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    connection
        .execute(
            "INSERT INTO app_ai_settings (key, settings_json, updated_at)
             VALUES ('main', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET
                settings_json = excluded.settings_json,
                updated_at = excluded.updated_at;",
            params![json_str, now],
        )
        .map_err(|error| format!("保存 AI 设置失败: {error}"))?;
    Ok(())
}
