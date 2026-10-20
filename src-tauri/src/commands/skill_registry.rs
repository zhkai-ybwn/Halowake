use serde_json::Value;
use tauri::State;

use crate::storage::{
    skill_registry_repository::{self, SkillSearchRequest, SkillSearchResponse},
    AppDatabase,
};

#[tauri::command]
pub async fn load_skill_registry(database: State<'_, AppDatabase>) -> Result<Option<Value>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || skill_registry_repository::load_skill_registry(&database))
        .await
        .map_err(|error| format!("读取技能目录任务失败: {error}"))?
}

#[tauri::command]
pub async fn save_skill_registry(database: State<'_, AppDatabase>, registry: Value) -> Result<(), String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || skill_registry_repository::save_skill_registry(&database, &registry))
        .await
        .map_err(|error| format!("写入技能目录任务失败: {error}"))?
}

#[tauri::command]
pub async fn search_skill_registry(
    database: State<'_, AppDatabase>,
    request: SkillSearchRequest,
) -> Result<SkillSearchResponse, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        skill_registry_repository::search_skill_registry(&database, &request)
    })
    .await
    .map_err(|error| format!("搜索技能目录任务失败: {error}"))?
}

#[tauri::command]
pub async fn rebuild_skill_registry_fts(
    database: State<'_, AppDatabase>,
    force: Option<bool>,
) -> Result<bool, String> {
    let database = database.inner().clone();
    let force_rebuild = force.unwrap_or(true);
    tauri::async_runtime::spawn_blocking(move || {
        skill_registry_repository::rebuild_skill_registry_fts(&database, force_rebuild)
    })
    .await
    .map_err(|error| format!("重建技能 FTS 索引任务失败: {error}"))?
}
