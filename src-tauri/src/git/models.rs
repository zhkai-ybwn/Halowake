use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    pub repo_path: String,
    pub repo_root: String,
    pub branch: String,
    pub repository_state: GitRepositoryState,
    pub status: Vec<String>,
    pub staged_files: Vec<String>,
    pub staged_diff: String,
    pub file_stats: Vec<GitFileStat>,
    pub branches: Vec<GitBranch>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryState {
    pub has_commits: bool,
    pub remote_name: Option<String>,
    pub remote_url: Option<String>,
    pub upstream: Option<String>,
    pub upstream_gone: bool,
    pub ahead: usize,
    pub behind: usize,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileStat {
    pub path: String,
    pub added: Option<usize>,
    pub removed: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiffPayload {
    pub repo_path: String,
    pub file_path: String,
    pub staged: bool,
    pub full_context: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiffResponse {
    pub file_path: String,
    pub staged: bool,
    pub diff: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPayload {
    pub repo_path: String,
    pub title: String,
    pub body: String,
    pub selected_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPushPayload {
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPullPayload {
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRebasePayload {
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitConfigureRemotePayload {
    pub repo_path: String,
    pub remote_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepairUpstreamPayload {
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileActionPayload {
    pub repo_path: String,
    pub file_path: String,
    pub full_context: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFilesActionPayload {
    pub repo_path: String,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchPayload {
    pub repo_path: String,
    pub branch: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitMergePayload {
    pub repo_path: String,
    pub source_branch: String,
    pub no_fast_forward: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteBranchCheckoutPayload {
    pub repo_path: String,
    pub remote_branch: String,
    pub local_branch: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchUpstreamPayload {
    pub repo_path: String,
    pub branch: String,
    pub upstream: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitClonePayload {
    pub remote_url: String,
    pub destination_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranch {
    pub name: String,
    pub kind: GitBranchKind,
    pub current: bool,
    pub upstream: Option<String>,
    pub upstream_status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitBranchKind {
    Local,
    Remote,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepoPayload {
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLogPayload {
    pub repo_path: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitDetailPayload {
    pub repo_path: String,
    pub hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitFileDiffPayload {
    pub repo_path: String,
    pub hash: String,
    pub file_path: String,
    pub full_context: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandResult {
    pub command: String,
    pub message: String,
    pub stdout: String,
    pub stderr: String,
    pub suggestion: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandProgressEvent {
    pub repo_path: String,
    pub command: String,
    pub stream: String,
    pub text: String,
    pub phase: Option<String>,
    pub percent: Option<u8>,
    pub transfer: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncStatus {
    pub command: String,
    pub message: String,
    pub stdout: String,
    pub stderr: String,
    pub suggestion: Option<String>,
    pub state: GitRepositoryState,
    pub recommended_action: GitSyncRecommendedAction,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GitSyncRecommendedAction {
    Push,
    Pull,
    ResolveDivergence,
    ConfigureRemote,
    PublishBranch,
    None,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLogEntry {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitChangedFile {
    pub status: String,
    pub path: String,
    pub original_path: Option<String>,
    pub added: Option<usize>,
    pub removed: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitDetail {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub subject: String,
    pub body: String,
    pub short_stat: String,
    pub changed_files: Vec<GitCommitChangedFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitFileDiffResponse {
    pub hash: String,
    pub file_path: String,
    pub diff: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAiPayload {
    pub repo_path: String,
    pub branch: String,
    pub status: Vec<String>,
    pub staged_files: Vec<String>,
    pub staged_diff: String,
    pub model: AiModelConfig,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPromptPayload {
    pub repo_path: String,
    pub branch: String,
    pub selected_files: Vec<String>,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitReviewAttentionPayload {
    pub repo_path: String,
    pub selected_files: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitReviewAttention {
    pub path: String,
    pub score: i32,
    pub categories: Vec<String>,
    pub score_breakdown: Vec<GitReviewScoreBreakdown>,
    pub eligible: bool,
    pub skipped: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitReviewScoreBreakdown {
    pub factor: String,
    pub delta: i32,
    pub evidence: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitReviewAttentionResult {
    pub files: Vec<GitReviewAttention>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPromptAiPayload {
    pub request_id: String,
    pub prompt: String,
    pub model: AiModelConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPromptPreview {
    pub prompt: String,
    pub trace: GitCommitPromptTrace,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPromptTrace {
    pub selected_files: Vec<GitCommitPromptFileTrace>,
    pub raw_chars: usize,
    pub cleaned_chars: usize,
    pub evidence_count: usize,
    pub rules: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPromptFileTrace {
    pub path: String,
    pub role: String,
    pub scope: String,
    pub kind: String,
    pub strategy: String,
    pub raw_chars: usize,
    pub cleaned_chars: usize,
    pub evidence_count: usize,
    pub skipped: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAiAnalysis {
    pub title: String,
    pub body: String,
    pub summary: String,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiProviderType {
    OpenaiCompatible,
    Ollama,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelConfig {
    pub id: String,
    pub name: String,
    pub provider: AiProviderType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaGenerateResponse {
    pub response: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedGitFile {
    pub raw: String,
    pub x: String,
    pub y: String,
    pub path: String,
    pub original_path: Option<String>,
    pub change_type: String,
    pub staged: bool,
    pub unstaged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum FileRole {
    Primary,
    Secondary,
    Generated,
    Tooling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeSummary {
    pub scope: String,
    pub role: FileRole,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSignals {
    pub has_route_changes: bool,
    pub has_i18n_changes: bool,
    pub has_constants_changes: bool,
    pub has_config_changes: bool,
    pub has_new_files: bool,
    pub has_deleted_files: bool,
    pub has_feature_code_changes: bool,
    pub has_renamed_files: bool,
    pub has_untracked_files: bool,
    pub has_page_like_changes: bool,
    pub has_component_like_changes: bool,
    pub has_service_like_changes: bool,
    pub has_model_like_changes: bool,
    pub has_log_like_changes: bool,
    pub has_form_like_changes: bool,
    pub has_list_like_changes: bool,
    pub has_detail_like_changes: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAnalysisContext {
    pub primary_files: Vec<String>,
    pub secondary_files: Vec<String>,
    pub generated_files: Vec<String>,
    pub tooling_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub main_scopes: Vec<String>,
    pub scope_summaries: Vec<ScopeSummary>,
    pub diff_signals: DiffSignals,
    pub counts: GitAnalysisCounts,
    pub summary_hint: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAnalysisCounts {
    pub total: usize,
    pub primary: usize,
    pub secondary: usize,
    pub generated: usize,
    pub tooling: usize,
    pub untracked: usize,
    pub deleted: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Page,
    Component,
    Service,
    Model,
    Config,
    Constants,
    Route,
    I18n,
    Style,
    Test,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileAction {
    Add,
    Update,
    Remove,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSemanticInfo {
    pub path: String,
    pub scope: String,
    pub role: FileRole,
    pub kind: FileKind,
    pub action: FileAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeActionSummary {
    pub scope: String,
    pub score: i32,
    pub actions: Vec<String>,
    pub key_kinds: Vec<String>,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffEvidence {
    pub scope: String,
    pub file_path: String,
    pub kind: FileKind,
    pub action: FileAction,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitIntentSummary {
    pub main_scopes: Vec<String>,
    pub primary_actions: Vec<ScopeActionSummary>,
    pub supporting_changes: Vec<String>,
    pub risk_signals: Vec<String>,
    pub diff_evidences: Vec<DiffEvidence>,
}
