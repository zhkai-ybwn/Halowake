use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

use super::AppDatabase;

pub fn load_skill_registry(database: &AppDatabase) -> Result<Option<Value>, String> {
    let connection = database.connect()?;
    let snapshot: Option<(String, i64, Option<String>, String)> = connection
        .query_row(
            "SELECT version, updated_at, description, pipelines_json FROM skill_registry_snapshot WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| format!("读取技能目录快照失败: {error}"))?;
    let Some((version, updated_at, description, pipelines_json)) = snapshot else {
        return Ok(None);
    };
    let mut statement = connection
        .prepare("SELECT payload_json FROM skill_registry_entries ORDER BY rowid")
        .map_err(|error| format!("读取技能目录失败: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("查询技能目录失败: {error}"))?;
    let mut skills = Vec::new();
    for row in rows {
        let text = row.map_err(|error| format!("读取技能条目失败: {error}"))?;
        skills.push(serde_json::from_str::<Value>(&text).map_err(|error| format!("解析技能条目失败: {error}"))?);
    }
    let pipelines: Value = serde_json::from_str(&pipelines_json)
        .map_err(|error| format!("解析技能流水线失败: {error}"))?;
    Ok(Some(json!({
        "version": version,
        "updatedAt": updated_at,
        "description": description,
        "skills": skills,
        "pipelines": pipelines,
    })))
}

pub fn save_skill_registry(database: &AppDatabase, registry: &Value) -> Result<(), String> {
    let skills = registry.get("skills").and_then(Value::as_array)
        .ok_or_else(|| "技能目录缺少 skills 数组".to_string())?;
    if skills.is_empty() || skills.len() > 100_000 {
        return Err("技能目录数量无效".to_string());
    }
    let version = registry.get("version").and_then(Value::as_str).unwrap_or("1.0.0");
    let updated_at = registry.get("updatedAt").and_then(Value::as_i64).unwrap_or(0);
    let description = registry.get("description").and_then(Value::as_str);
    let pipelines = registry.get("pipelines").cloned().unwrap_or_else(|| json!([]));
    let mut connection = database.connect()?;
    let transaction = connection.transaction()
        .map_err(|error| format!("开启技能目录事务失败: {error}"))?;
    transaction.execute("DELETE FROM skill_registry_entries", [])
        .map_err(|error| format!("清理旧技能目录失败: {error}"))?;
    {
        let mut insert = transaction.prepare(
            "INSERT INTO skill_registry_entries (id, name, description, stage, payload_json) VALUES (?1, ?2, ?3, ?4, ?5)"
        ).map_err(|error| format!("准备技能目录写入失败: {error}"))?;
        for skill in skills {
            let id = skill.get("id").and_then(Value::as_str).unwrap_or("");
            let name = skill.get("name").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() || name.is_empty() { return Err("技能条目缺少 id 或 name".to_string()); }
            let text = serde_json::to_string(skill).map_err(|error| format!("序列化技能条目失败: {error}"))?;
            insert.execute(params![id, name, skill.get("description").and_then(Value::as_str).unwrap_or(""),
                skill.get("stage").and_then(Value::as_str).unwrap_or("coding"), text])
                .map_err(|error| format!("写入技能 {} 失败: {error}", id))?;
        }
    }
    transaction.execute(
        "INSERT INTO skill_registry_snapshot (id, version, updated_at, description, pipelines_json)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET version=excluded.version, updated_at=excluded.updated_at,
           description=excluded.description, pipelines_json=excluded.pipelines_json",
        params![version, updated_at, description, pipelines.to_string()],
    ).map_err(|error| format!("写入技能目录快照失败: {error}"))?;
    transaction.commit().map_err(|error| format!("提交技能目录失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;

    #[test]
    fn failed_refresh_keeps_last_complete_catalog() {
        let path = std::env::temp_dir().join(format!(
            "lumina-skill-registry-test-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let database = AppDatabase::from_path(path.clone());
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        let skill = json!({"id":"one","name":"Example","description":"Example skill","stage":"coding"});
        let good = json!({"version":"1","updatedAt":123,"skills":[skill],"pipelines":[]});
        save_skill_registry(&database, &good).unwrap();
        let bad = json!({"version":"2","updatedAt":456,"skills":[skill, skill],"pipelines":[]});
        assert!(save_skill_registry(&database, &bad).is_err());
        let loaded = load_skill_registry(&database).unwrap().unwrap();
        assert_eq!(loaded["updatedAt"], 123);
        assert_eq!(loaded["skills"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
