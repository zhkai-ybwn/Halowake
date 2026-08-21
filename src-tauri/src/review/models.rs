use serde::{Deserialize, Serialize};

use crate::git::models::AiModelConfig;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartReviewPayload {
    pub repo_path: String,
    pub selected_files: Vec<String>,
    pub model: AiModelConfig,
    pub budget_mode: ReviewBudgetMode,
    pub language: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewBudgetMode {
    Compact,
    Standard,
    Deep,
}

impl ReviewBudgetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }

    pub fn input_chars(self) -> usize {
        match self {
            Self::Compact => 24_000,
            Self::Standard => 48_000,
            Self::Deep => 96_000,
        }
    }

    pub fn max_findings(self) -> usize {
        match self {
            Self::Compact => 6,
            Self::Standard => 12,
            Self::Deep => 20,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBreakdownItem {
    pub factor: String,
    pub delta: i32,
    pub evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFileRecord {
    pub path: String,
    pub change_kind: String,
    pub attention_score: i32,
    pub score_categories: Vec<String>,
    pub score_breakdown: Vec<ScoreBreakdownItem>,
    pub selected: bool,
    pub review_status: String,
    pub batch_id: Option<String>,
    pub limitation: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFinding {
    pub id: String,
    pub fingerprint: String,
    pub source: String,
    pub rule_id: Option<String>,
    pub category: String,
    pub severity: String,
    pub confidence: f64,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub title: String,
    pub problem: String,
    pub impact: String,
    pub trigger_scenario: String,
    pub evidence: String,
    pub suggestion: Option<String>,
    pub verified: bool,
    pub status: String,
    pub user_note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub enabled: bool,
    pub severity: String,
    pub category: String,
    #[serde(default)]
    pub include_globs: Vec<String>,
    #[serde(default)]
    pub exclude_globs: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    pub definition: serde_json::Value,
    #[serde(default = "default_rule_source")]
    pub source: String,
    #[serde(default = "default_rule_version")]
    pub version: i64,
}

fn default_rule_source() -> String { "project".to_string() }
fn default_rule_version() -> i64 { 1 }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCallUsage {
    pub batch_id: String,
    pub files: Vec<String>,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub estimated: bool,
    pub duration_ms: u64,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReviewOverview {
    pub critical: usize,
    pub major: usize,
    pub minor: usize,
    pub suggestion: usize,
    pub applied_rules: usize,
    pub triggered_rules: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSession {
    pub id: String,
    pub repo_root: String,
    pub diff_fingerprint: String,
    pub status: String,
    pub phase: String,
    pub progress_done: usize,
    pub progress_total: usize,
    pub current_file: Option<String>,
    pub budget_mode: String,
    pub model_id: String,
    pub selected_files: Vec<String>,
    pub overview: ReviewOverview,
    pub limitations: Vec<String>,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub usage_estimated: bool,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub is_pinned: bool,
    pub files: Vec<ReviewFileRecord>,
    pub findings: Vec<ReviewFinding>,
    pub ai_calls: Vec<AiCallUsage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSessionSummary {
    pub id: String,
    pub repo_root: String,
    pub status: String,
    pub phase: String,
    pub selected_file_count: usize,
    pub overview: ReviewOverview,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub usage_estimated: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_pinned: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFindingPayload {
    pub session_id: String,
    pub finding_id: String,
    pub status: String,
    pub user_note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReviewedFile {
    pub path: String,
    pub status: String,
    pub limitation: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReviewFinding {
    pub client_id: String,
    pub rule_id: Option<String>,
    pub category: String,
    pub severity: String,
    pub confidence: f64,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default)]
    pub existing_code: String,
    pub title: String,
    pub problem: String,
    pub impact: String,
    pub trigger_scenario: String,
    pub evidence: String,
    pub suggestion: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReviewBatchResult {
    pub schema_version: u8,
    pub batch_id: String,
    pub reviewed_files: Vec<AiReviewedFile>,
    pub findings: Vec<AiReviewFinding>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProgressEvent {
    pub session_id: String,
    pub revision: u64,
    pub status: String,
    pub phase: String,
    pub completed: usize,
    pub total: usize,
    pub current_file: Option<String>,
}
