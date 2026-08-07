use crate::ai::ollama;
use crate::ai::qwencloud;
use crate::git::models::{
    AiModelConfig, AiProviderType, GitAiAnalysis, GitAiPayload, GitCommitPayload,
    GitCommitPromptPayload, GitCommitPromptPreview, GitFileDiffPayload, GitFileDiffResponse,
    GitCommandResult, GitCommitDetail, GitCommitDetailPayload, GitCommitFileDiffPayload,
    GitBranch, GitBranchPayload, GitBranchUpstreamPayload, GitClonePayload, GitCommitFileDiffResponse,
    GitConfigureRemotePayload, GitRemoteBranchCheckoutPayload,
    GitFileActionPayload, GitFilesActionPayload,
    GitLogEntry, GitLogPayload, GitMergePayload, GitPromptAiPayload, GitPullPayload, GitPushPayload, GitRebasePayload, GitRepoPayload,
    GitRepairUpstreamPayload, GitReviewAttentionPayload, GitReviewAttentionResult, GitSnapshot, GitSyncStatus,
};
use crate::git::prompt::{build_analysis_prompt, build_review_attention_with_progress, build_selected_commit_prompt};
use crate::git::profile::{self, GitProjectProfileFile};
use crate::git::runner;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

static AI_GENERATION_TASKS: OnceLock<Mutex<HashMap<String, tokio::task::AbortHandle>>> = OnceLock::new();

fn ai_generation_tasks() -> &'static Mutex<HashMap<String, tokio::task::AbortHandle>> {
    AI_GENERATION_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGitProjectProfilePayload {
    pub repo_path: String,
    pub content: String,
}

#[tauri::command]
pub async fn load_git_snapshot(repo_path: String) -> Result<GitSnapshot, String> {
    run_git_task("加载仓库状态", move || runner::load_git_snapshot(&repo_path)).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelGitAiPayload {
    pub request_id: String,
}

#[tauri::command]
pub async fn load_git_file_diff(payload: GitFileDiffPayload) -> Result<GitFileDiffResponse, String> {
    run_git_task("加载文件 Diff", move || {
        runner::load_file_diff(
            &payload.repo_path,
            &payload.file_path,
            payload.staged,
            payload.full_context.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn load_git_file_head_diff(payload: GitFileActionPayload) -> Result<GitFileDiffResponse, String> {
    run_git_task("加载文件上一版本 Diff", move || runner::load_file_head_diff(&payload)).await
}

#[tauri::command]
pub async fn commit_git_changes(app: AppHandle, payload: GitCommitPayload) -> Result<GitCommandResult, String> {
    run_git_task("提交变更", move || {
        runner::commit_changes_with_progress(&payload, |event| {
            let _ = app.emit("git-command-progress", event);
        })
    })
    .await
}

#[tauri::command]
pub async fn push_git_changes(app: AppHandle, payload: GitPushPayload) -> Result<GitCommandResult, String> {
    run_git_task("推送变更", move || {
        runner::push_changes_with_progress(&payload, |event| {
            let _ = app.emit("git-command-progress", event);
        })
    })
    .await
}

#[tauri::command]
pub async fn pull_git_changes(app: AppHandle, payload: GitPullPayload) -> Result<GitCommandResult, String> {
    run_git_task("拉取变更", move || {
        runner::pull_changes_with_progress(&payload, |event| {
            let _ = app.emit("git-command-progress", event);
        })
    })
    .await
}

#[tauri::command]
pub async fn rebase_git_changes(app: AppHandle, payload: GitRebasePayload) -> Result<GitCommandResult, String> {
    run_git_task("Rebase 变更", move || {
        runner::rebase_changes_with_progress(&payload, |event| {
            let _ = app.emit("git-command-progress", event);
        })
    })
    .await
}

#[tauri::command]
pub async fn fetch_git_changes(app: AppHandle, payload: GitRepoPayload) -> Result<GitCommandResult, String> {
    run_git_task("Fetch 远端变更", move || {
        runner::fetch_changes_with_progress(&payload, |event| {
            let _ = app.emit("git-command-progress", event);
        })
    })
    .await
}

#[tauri::command]
pub async fn sync_git_status(payload: GitRepoPayload) -> Result<GitSyncStatus, String> {
    run_git_task("检查远端同步状态", move || runner::sync_status(&payload)).await
}

#[tauri::command]
pub async fn configure_git_origin(payload: GitConfigureRemotePayload) -> Result<GitCommandResult, String> {
    run_git_task("配置远端", move || runner::configure_origin_remote(&payload)).await
}

#[tauri::command]
pub async fn repair_git_upstream(payload: GitRepairUpstreamPayload) -> Result<GitCommandResult, String> {
    run_git_task("修复 upstream", move || runner::repair_upstream(&payload)).await
}

#[tauri::command]
pub async fn open_git_file_external(payload: GitFileActionPayload) -> Result<GitCommandResult, String> {
    run_git_task("打开外部编辑器", move || runner::open_file_external(&payload)).await
}

#[tauri::command]
pub async fn mark_git_files_resolved(payload: GitFilesActionPayload) -> Result<GitCommandResult, String> {
    run_git_task("标记冲突已解决", move || runner::mark_files_resolved(&payload)).await
}

#[tauri::command]
pub async fn revert_git_file(payload: GitFileActionPayload) -> Result<GitCommandResult, String> {
    run_git_task("回退文件变更", move || runner::revert_file(&payload)).await
}

#[tauri::command]
pub async fn stage_git_files(payload: GitFilesActionPayload) -> Result<GitCommandResult, String> {
    run_git_task("暂存文件", move || runner::stage_files(&payload)).await
}

#[tauri::command]
pub async fn unstage_git_files(payload: GitFilesActionPayload) -> Result<GitCommandResult, String> {
    run_git_task("取消暂存文件", move || runner::unstage_files(&payload)).await
}

#[tauri::command]
pub async fn load_git_branches(repo_path: String) -> Result<Vec<GitBranch>, String> {
    run_git_task("加载分支", move || runner::load_branches(&repo_path)).await
}

#[tauri::command]
pub async fn create_git_branch(payload: GitBranchPayload) -> Result<GitCommandResult, String> {
    run_git_task("创建并切换分支", move || runner::create_branch(&payload)).await
}

#[tauri::command]
pub async fn switch_git_branch(payload: GitBranchPayload) -> Result<GitCommandResult, String> {
    run_git_task("切换分支", move || runner::switch_branch(&payload)).await
}

#[tauri::command]
pub async fn checkout_git_remote_branch(payload: GitRemoteBranchCheckoutPayload) -> Result<GitCommandResult, String> {
    run_git_task("检出远端分支", move || runner::checkout_remote_branch(&payload)).await
}

#[tauri::command]
pub async fn merge_git_branch(payload: GitMergePayload) -> Result<GitCommandResult, String> {
    run_git_task("合并分支", move || runner::merge_branch(&payload)).await
}

#[tauri::command]
pub async fn delete_git_branch(payload: GitBranchPayload) -> Result<GitCommandResult, String> {
    run_git_task("删除分支", move || runner::delete_branch(&payload)).await
}

#[tauri::command]
pub async fn set_git_branch_upstream(payload: GitBranchUpstreamPayload) -> Result<GitCommandResult, String> {
    run_git_task("设置分支上游", move || runner::set_branch_upstream(&payload)).await
}

#[tauri::command]
pub async fn init_git_repository(repo_path: String) -> Result<GitCommandResult, String> {
    run_git_task("初始化仓库", move || runner::init_repository(&repo_path)).await
}

#[tauri::command]
pub async fn clone_git_repository(payload: GitClonePayload) -> Result<GitCommandResult, String> {
    run_git_task("克隆仓库", move || runner::clone_repository(&payload)).await
}

#[tauri::command]
pub async fn abort_git_merge(payload: GitRepoPayload) -> Result<GitCommandResult, String> {
    run_git_task("中止 merge", move || runner::abort_merge(&payload)).await
}

#[tauri::command]
pub async fn continue_git_merge(payload: GitRepoPayload) -> Result<GitCommandResult, String> {
    run_git_task("继续 merge", move || runner::continue_merge(&payload)).await
}

#[tauri::command]
pub async fn continue_git_rebase(payload: GitRepoPayload) -> Result<GitCommandResult, String> {
    run_git_task("继续 rebase", move || runner::continue_rebase(&payload)).await
}

#[tauri::command]
pub async fn abort_git_rebase(payload: GitRepoPayload) -> Result<GitCommandResult, String> {
    run_git_task("中止 rebase", move || runner::abort_rebase(&payload)).await
}

#[tauri::command]
pub async fn load_git_log(payload: GitLogPayload) -> Result<Vec<GitLogEntry>, String> {
    run_git_task("加载 Git Log", move || runner::load_git_log(&payload)).await
}

#[tauri::command]
pub async fn load_git_commit_detail(payload: GitCommitDetailPayload) -> Result<GitCommitDetail, String> {
    run_git_task("加载提交明细", move || runner::load_git_commit_detail(&payload)).await
}

#[tauri::command]
pub async fn load_git_commit_file_diff(payload: GitCommitFileDiffPayload) -> Result<GitCommitFileDiffResponse, String> {
    run_git_task("加载提交文件 Diff", move || runner::load_git_commit_file_diff(&payload)).await
}

#[tauri::command]
pub fn ensure_git_project_profile(repo_path: String) -> Result<GitProjectProfileFile, String> {
    profile::ensure_project_profile(&repo_path)
}

#[tauri::command]
pub fn load_git_project_profile(repo_path: String) -> Result<GitProjectProfileFile, String> {
    profile::load_project_profile(&repo_path)
}

#[tauri::command]
pub fn save_git_project_profile(payload: SaveGitProjectProfilePayload) -> Result<GitProjectProfileFile, String> {
    profile::save_project_profile(&payload.repo_path, &payload.content)
}

#[tauri::command]
pub async fn build_git_commit_prompt(payload: GitCommitPromptPayload) -> Result<GitCommitPromptPreview, String> {
    let language = payload.language.unwrap_or_else(|| "en".to_string());
    tokio::task::spawn_blocking(move || {
        build_selected_commit_prompt(&payload.repo_path, &payload.branch, &payload.selected_files, &language)
    })
    .await
    .map_err(|e| format!("构建 Commit Prompt 任务失败: {}", e))?
}

#[tauri::command]
pub async fn generate_git_ai_analysis(payload: GitAiPayload) -> Result<GitAiAnalysis, String> {
    let prompt = build_analysis_prompt(&payload);
    call_ai_with_prompt(&payload.model, &prompt).await
}

#[tauri::command]
pub async fn generate_git_ai_analysis_from_prompt(payload: GitPromptAiPayload) -> Result<GitAiAnalysis, String> {
    let request_id = payload.request_id;
    let task = tokio::spawn(async move { call_ai_with_prompt(&payload.model, &payload.prompt).await });
    ai_generation_tasks()
        .lock()
        .map_err(|_| "AI 生成任务状态不可用".to_string())?
        .insert(request_id.clone(), task.abort_handle());

    let result = match task.await {
        Ok(result) => result,
        Err(error) if error.is_cancelled() => Err("AI_GENERATION_CANCELLED".to_string()),
        Err(error) => Err(format!("AI 生成任务异常: {}", error)),
    };
    if let Ok(mut tasks) = ai_generation_tasks().lock() {
        tasks.remove(&request_id);
    }
    result
}

#[tauri::command]
pub fn cancel_git_ai_analysis(payload: CancelGitAiPayload) -> Result<bool, String> {
    let task = ai_generation_tasks()
        .lock()
        .map_err(|_| "AI 生成任务状态不可用".to_string())?
        .remove(&payload.request_id);
    if let Some(task) = task {
        task.abort();
        return Ok(true);
    }
    Ok(false)
}

async fn call_ai_with_prompt(model: &AiModelConfig, prompt: &str) -> Result<GitAiAnalysis, String> {
    match model.provider {
        AiProviderType::Ollama => ollama::call_ollama_generate(model, prompt).await,
        AiProviderType::OpenaiCompatible => {
            qwencloud::call_openai_compatible_generate(model, prompt).await
        }
    }
}

#[tauri::command]
pub async fn score_git_review_files(app: AppHandle, payload: GitReviewAttentionPayload) -> Result<GitReviewAttentionResult, String> {
    let repo_path = payload.repo_path.clone();
    run_git_task("计算代码审查优先级", move || {
        build_review_attention_with_progress(
            &payload.repo_path,
            &payload.selected_files,
            |completed, total, phase, file_path| {
                let _ = app.emit("git-review-score-progress", serde_json::json!({
                    "repoPath": repo_path.as_str(),
                    "completed": completed,
                    "total": total,
                    "phase": phase,
                    "filePath": file_path,
                }));
            },
        )
    }).await
}

async fn run_git_task<T, F>(task_name: &str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("{}任务异常: {}", task_name, e))?
}

#[tauri::command]
pub async fn test_ai_model_connection(model: AiModelConfig) -> Result<String, String> {
    if !model.enabled {
        return Err("Model is disabled".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = match model.provider {
        AiProviderType::Ollama => {
            client
                .get(format!("{}/api/tags", model.base_url.trim_end_matches('/')))
                .send()
                .await
        }
        AiProviderType::OpenaiCompatible => {
            let mut request = client.get(format!("{}/models", model.base_url.trim_end_matches('/')));
            if let Some(api_key) = model.api_key.as_ref().map(|key| key.trim()).filter(|key| !key.is_empty()) {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }
            request.send().await
        }
    }
    .map_err(|e| format!("Connection failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Connection failed with status {}: {}", status, text));
    }

    Ok(format!("{} connected", model.name))
}
