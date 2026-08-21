use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Manager, State};

use crate::{
    review::{
        models::{ReviewRule, ReviewSession, ReviewSessionSummary, StartReviewPayload, UpdateFindingPayload},
        planner, repository, runner, ReviewTaskRegistry,
    },
    storage::AppDatabase,
};

static REVIEW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[tauri::command]
pub async fn start_local_code_review(
    app: AppHandle,
    database: State<'_, AppDatabase>,
    registry: State<'_, ReviewTaskRegistry>,
    payload: StartReviewPayload,
) -> Result<String, String> {
    let db = database.inner().clone();
    let plan_payload = payload.clone();
    let plan_db = db.clone();
    let plan = tokio::task::spawn_blocking(move || planner::build_plan(&plan_db, &plan_payload.repo_path, &plan_payload.selected_files, plan_payload.budget_mode))
        .await.map_err(|error| format!("创建 Review 计划任务失败: {error}"))??;
    let session_id = format!("review-{}-{}", repository::now_millis(), REVIEW_ID_COUNTER.fetch_add(1, Ordering::Relaxed));
    repository::create_session(
        &db, &session_id, &plan.repo_root, &plan.fingerprint, payload.budget_mode.as_str(),
        &payload.model.id, &payload.selected_files, &plan.rules, crate::commands::storage::current_retention_days(&app),
    )?;
    let task_app = app.clone();
    let task_db = db.clone();
    let task_id = session_id.clone();
    let task = tokio::spawn(async move {
        runner::execute_review(task_app.clone(), task_db, task_id.clone(), payload, plan).await;
        task_app.state::<ReviewTaskRegistry>().remove(&task_id);
    });
    registry.insert(session_id.clone(), task.abort_handle())?;
    Ok(session_id)
}

#[tauri::command]
pub fn get_local_code_review(database: State<'_, AppDatabase>, session_id: String) -> Result<ReviewSession, String> {
    repository::get_session(&database, &session_id)
}

#[tauri::command]
pub fn list_local_code_reviews(database: State<'_, AppDatabase>, repo_root: String, limit: Option<usize>) -> Result<Vec<ReviewSessionSummary>, String> {
    repository::list_sessions(&database, &repo_root, limit.unwrap_or(20).clamp(1, 100))
}

#[tauri::command]
pub fn cancel_local_code_review(
    database: State<'_, AppDatabase>, registry: State<'_, ReviewTaskRegistry>, session_id: String,
) -> Result<bool, String> {
    let cancelled = registry.cancel(&session_id)?;
    if cancelled {
        repository::finish_session(&database, &session_id, "cancelled", &Default::default(), &[], 0, 0, true, None)?;
    }
    Ok(cancelled)
}

#[tauri::command]
pub fn update_review_finding(database: State<'_, AppDatabase>, payload: UpdateFindingPayload) -> Result<ReviewSession, String> {
    repository::update_finding(&database, &payload.finding_id, &payload.status, payload.user_note.as_deref())?;
    repository::get_session(&database, &payload.session_id)
}

#[tauri::command]
pub fn list_review_rules(database: State<'_, AppDatabase>) -> Result<Vec<ReviewRule>, String> { repository::list_rules(&database) }

#[tauri::command]
pub fn save_review_rule(database: State<'_, AppDatabase>, rule: ReviewRule) -> Result<Vec<ReviewRule>, String> {
    repository::upsert_rule(&database, &rule)?;
    repository::list_rules(&database)
}

#[tauri::command]
pub fn delete_review_rule(database: State<'_, AppDatabase>, id: String) -> Result<Vec<ReviewRule>, String> {
    repository::delete_rule(&database, &id)?;
    repository::list_rules(&database)
}
