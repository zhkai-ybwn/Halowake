use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectCommandKind {
    Service,
    Task,
}

impl Default for ProjectCommandKind {
    fn default() -> Self { Self::Task }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectRunPolicy {
    Singleton,
}

impl Default for ProjectRunPolicy {
    fn default() -> Self { Self::Singleton }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRuntimes {
    pub python: Option<PythonRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonRuntime {
    pub interpreter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommandCommon {
    pub id: String,
    pub name: String,
    /// Kept only so schema v1 files can still be read. Schema v2 never writes
    /// or uses service/task to decide runtime behavior.
    #[serde(default, rename = "kind", skip_serializing)]
    pub legacy_kind: Option<ProjectCommandKind>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub run_policy: ProjectRunPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "executor", rename_all = "kebab-case")]
pub enum ProjectCommandConfig {
    PackageScript { #[serde(flatten)] common: ProjectCommandCommon, script: String },
    Python { #[serde(flatten)] common: ProjectCommandCommon, script: String },
    PythonModule { #[serde(flatten)] common: ProjectCommandCommon, module: String },
    Cmd { #[serde(flatten)] common: ProjectCommandCommon, script: String },
    Powershell { #[serde(flatten)] common: ProjectCommandCommon, script: String },
}

impl ProjectCommandConfig {
    pub fn common(&self) -> &ProjectCommandCommon {
        match self {
            Self::PackageScript { common, .. }
            | Self::Python { common, .. }
            | Self::PythonModule { common, .. }
            | Self::Cmd { common, .. }
            | Self::Powershell { common, .. } => common,
        }
    }

    pub fn id(&self) -> &str { &self.common().id }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommandOverride {
    pub name: Option<String>,
    #[serde(default, rename = "kind", skip_serializing)]
    pub legacy_kind: Option<ProjectCommandKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommandDefaults {
    pub command_id: Option<String>,
    #[serde(default, rename = "serviceCommandId", skip_serializing)]
    pub legacy_service_command_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuminaProjectConfig {
    pub schema_version: u32,
    pub name: Option<String>,
    #[serde(default)]
    pub types: Vec<String>,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub runtimes: ProjectRuntimes,
    #[serde(default)]
    pub commands: Vec<ProjectCommandConfig>,
    #[serde(default)]
    pub command_overrides: HashMap<String, ProjectCommandOverride>,
    #[serde(default)]
    pub defaults: ProjectCommandDefaults,
}

impl Default for LuminaProjectConfig {
    fn default() -> Self {
        Self {
            schema_version: 2,
            name: None,
            types: Vec::new(),
            working_directory: None,
            environment: HashMap::new(),
            runtimes: ProjectRuntimes::default(),
            commands: Vec::new(),
            command_overrides: HashMap::new(),
            defaults: ProjectCommandDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommand {
    pub id: String,
    pub name: String,
    pub executor: String,
    pub source: String,
    pub source_label: String,
    pub command_preview: String,
    pub working_directory: String,
    pub run_policy: ProjectRunPolicy,
    pub config_revision: String,
    pub environment_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommandCandidate {
    pub suggested_id: String,
    pub name: String,
    pub executor: String,
    pub confidence: String,
    pub reason: String,
    pub source: String,
    pub draft: serde_json::Value,
}
