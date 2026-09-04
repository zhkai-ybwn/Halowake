use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
    Codex,
    Deepseek,
    Openrouter,
    Gemini,
    Custom,
}

impl ProviderType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Deepseek => "DeepSeek",
            Self::Openrouter => "OpenRouter",
            Self::Gemini => "Gemini",
            Self::Custom => "Custom Provider",
        }
    }

    pub fn default_dashboard_url(&self) -> Option<&'static str> {
        match self {
            Self::Codex => Some("https://chatgpt.com/"),
            Self::Deepseek => Some("https://platform.deepseek.com/usage"),
            Self::Openrouter => Some("https://openrouter.ai/credits"),
            Self::Gemini => Some("https://aistudio.google.com/"),
            Self::Custom => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfig {
    pub id: String,
    pub provider_type: ProviderType,
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub enabled: bool,
    pub auto_discovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum QuotaKind {
    #[serde(rename_all = "camelCase")]
    Balance {
        currency: String, // "CNY", "USD"
        topped_up: f64,
        granted: f64,
        total_remaining: f64,
    },
    #[serde(rename_all = "camelCase")]
    RateLimit {
        period_label: String, // "5h", "Weekly", "RPM"
        used_percent: f64,    // 0.0 - 100.0
        resets_at: Option<i64>, // Unix timestamp in seconds
        resets_in_seconds: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    Credits {
        label: Option<String>,
        remaining: f64,
        total: Option<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PaceLevel {
    OnPace,
    Tight,
    OverPace,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaceStatus {
    pub level: PaceLevel,
    pub projected_usage_percent: Option<f64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCreditItem {
    pub id: String,
    pub status: String,
    pub title: Option<String>,
    pub granted_at: Option<String>,
    pub expires_at: Option<String>,
    pub expires_at_timestamp: Option<i64>,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCredits {
    pub available_count: i64,
    pub applicable_available_count: Option<i64>,
    pub nearest_expires_at: Option<i64>,
    pub nearest_expires_in_seconds: Option<i64>,
    pub items: Vec<ResetCreditItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuota {
    pub id: String,
    pub account_id: String,
    pub provider_type: ProviderType,
    pub name: String,
    pub plan: Option<String>,
    pub quotas: Vec<QuotaKind>,
    pub pace: Option<PaceStatus>,
    pub reset_credits: Option<ResetCredits>,
    pub last_updated: i64,
    pub is_healthy: bool,
    pub error_message: Option<String>,
    pub official_dashboard_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSummary {
    pub total_cny_balance: f64,
    pub total_usd_balance: f64,
    pub active_accounts_count: usize,
    pub warning_accounts_count: usize,
}
