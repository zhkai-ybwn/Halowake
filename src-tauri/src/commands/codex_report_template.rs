use tauri::{AppHandle, Manager};

use crate::storage::AppDatabase;
use crate::storage::history_repository::{
    delete_codex_report_template as db_delete_codex_report_template,
    list_codex_report_templates as db_list_codex_report_templates,
    reset_builtin_codex_report_templates as db_reset_builtin_codex_report_templates,
    save_codex_report_template as db_save_codex_report_template,
    CodexReportPromptTemplate,
};

#[tauri::command]
pub fn load_codex_report_templates(
    app: AppHandle,
) -> Result<Vec<CodexReportPromptTemplate>, String> {
    let db = app
        .try_state::<AppDatabase>()
        .ok_or_else(|| "数据库未初始化".to_string())?;
    db_list_codex_report_templates(&db)
}

#[tauri::command]
pub fn save_codex_report_template(
    app: AppHandle,
    template: CodexReportPromptTemplate,
) -> Result<(), String> {
    let db = app
        .try_state::<AppDatabase>()
        .ok_or_else(|| "数据库未初始化".to_string())?;
    db_save_codex_report_template(&db, &template)
}

#[tauri::command]
pub fn delete_codex_report_template(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let db = app
        .try_state::<AppDatabase>()
        .ok_or_else(|| "数据库未初始化".to_string())?;
    db_delete_codex_report_template(&db, &id)
}

#[tauri::command]
pub fn reset_builtin_codex_report_templates(
    app: AppHandle,
) -> Result<Vec<CodexReportPromptTemplate>, String> {
    let db = app
        .try_state::<AppDatabase>()
        .ok_or_else(|| "数据库未初始化".to_string())?;
    db_reset_builtin_codex_report_templates(&db)
}
