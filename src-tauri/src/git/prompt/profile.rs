use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde_json::Value;

use crate::git::runner;

#[derive(Clone)]
pub(super) struct PromptProfile {
    pub(super) roles: Vec<(String, Vec<String>)>,
    pub(super) scopes: Vec<(String, Vec<String>)>,
    pub(super) attention_weights: BTreeMap<String, i32>,
    pub(super) localization_patterns: Vec<String>,
}

#[derive(Clone)]
pub(super) struct FileClassification {
    pub(super) role: String,
    pub(super) scope: String,
    pub(super) kind: String,
    pub(super) strategy: String,
    pub(super) max_lines: usize,
    pub(super) skip_verbose: bool,
}

pub(super) fn load_prompt_profile(repo_path: &str) -> PromptProfile {
    let mut profile = fallback_prompt_profile();
    let repo_root = runner::run_git(repo_path, &["rev-parse", "--show-toplevel"])
        .unwrap_or_else(|_| repo_path.to_string());
    let repo_root = Path::new(repo_root.trim());
    profile.localization_patterns = detect_localization_patterns(repo_root);
    let profile_path = repo_root.join(".lumina").join("git-profile.json");

    let Ok(content) = fs::read_to_string(profile_path) else {
        return profile;
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return profile;
    };

    let roles = read_role_patterns(&value);
    if !roles.is_empty() {
        profile.roles = roles;
    }

    let scopes = read_scope_patterns(&value);
    if !scopes.is_empty() {
        profile.scopes = scopes;
    }

    let attention_weights = read_attention_weights(&value);
    if !attention_weights.is_empty() {
        profile.attention_weights = attention_weights;
    }

    for pattern in read_localization_patterns(&value) {
        if !profile.localization_patterns.contains(&pattern) {
            profile.localization_patterns.push(pattern);
        }
    }

    profile
}

pub(super) fn fallback_prompt_profile() -> PromptProfile {
    PromptProfile {
        roles: vec![
            (
                "generated".to_string(),
                vec![
                    "dist/**".to_string(),
                    "target/**".to_string(),
                    "node_modules/**".to_string(),
                    "package-lock.json".to_string(),
                    "pnpm-lock.yaml".to_string(),
                    "yarn.lock".to_string(),
                    "Cargo.lock".to_string(),
                    "src-tauri/Cargo.lock".to_string(),
                    ".lumina/commit-prompt-debug.json".to_string(),
                ],
            ),
            (
                "tooling".to_string(),
                vec![
                    "vite.config.*".to_string(),
                    "tsconfig*.json".to_string(),
                    "src-tauri/Cargo.toml".to_string(),
                    "src-tauri/tauri.conf.json".to_string(),
                ],
            ),
            (
                "primary".to_string(),
                vec![
                    "src/views/**".to_string(),
                    "src/components/**".to_string(),
                    "src/services/**".to_string(),
                    "src/stores/**".to_string(),
                    "src-tauri/src/commands/**".to_string(),
                    "src-tauri/src/git/**".to_string(),
                ],
            ),
            (
                "secondary".to_string(),
                vec![
                    "src/i18n/**".to_string(),
                    "src/styles/**".to_string(),
                    "src/router/**".to_string(),
                    "src/types/**".to_string(),
                ],
            ),
        ],
        scopes: vec![
            ("frontend".to_string(), vec!["src/**".to_string()]),
            ("tauri".to_string(), vec!["src-tauri/**".to_string()]),
            (
                "config".to_string(),
                vec![
                    "*.json".to_string(),
                    "*.toml".to_string(),
                    "*.config.*".to_string(),
                ],
            ),
        ],
        attention_weights: BTreeMap::from([
            ("source".to_string(), 10),
            ("config".to_string(), 8),
            ("style".to_string(), 3),
            ("lockfile".to_string(), -5),
        ]),
        localization_patterns: Vec::new(),
    }
}

pub(super) fn detect_localization_patterns(repo_root: &Path) -> Vec<String> {
    let mut patterns = BTreeSet::new();
    let mut visited = 0usize;
    scan_localization_directories(repo_root, repo_root, 0, &mut visited, &mut patterns);
    patterns.into_iter().collect()
}

pub(super) fn prompt_processing_rules() -> Vec<String> {
    vec![
        "Read only selected files and generate diff evidence locally before any model call."
            .to_string(),
        "Use .lumina/git-profile.json scopes and roles first; fall back to built-in rules when missing."
            .to_string(),
        "Evidence is selected by group budget, not by file order.".to_string(),
        "Comment, declaration, command, error, prompt, and user-facing text lines receive higher scores."
            .to_string(),
        "Generated, lock, and asset files are summarized instead of sending verbose content."
            .to_string(),
        "Style, i18n, docs, config, and tooling files keep reduced evidence as supporting context."
            .to_string(),
        "The final prompt is capped globally to avoid sending oversized workspace content."
            .to_string(),
    ]
}

fn read_localization_patterns(value: &Value) -> Vec<String> {
    value
        .get("review")
        .and_then(|review| review.get("localizationPatterns"))
        .and_then(Value::as_array)
        .map(|patterns| {
            patterns
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn scan_localization_directories(
    repo_root: &Path,
    directory: &Path,
    depth: usize,
    visited: &mut usize,
    patterns: &mut BTreeSet<String>,
) {
    if depth > 8 || *visited >= 6000 {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut child_directories = Vec::new();
    let mut locale_directories = Vec::new();
    let mut locale_codes = BTreeSet::new();

    for entry in entries.flatten() {
        *visited += 1;
        if *visited >= 6000 {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !matches!(
                name.as_str(),
                ".git" | ".lumina" | "node_modules" | "target" | "dist" | "build" | "vendor"
            ) {
                if normalize_locale_code(&name).is_some() {
                    let resource_names = localization_resource_names(&path);
                    if !resource_names.is_empty() {
                        locale_directories.push((path.clone(), resource_names));
                    }
                }
                child_directories.push(path);
            }
            continue;
        }
        if file_type.is_file() {
            if let Some(code) = locale_code_from_file(&path) {
                locale_codes.insert(code);
            }
        }
    }

    if locale_codes.len() >= 2 {
        if let Ok(relative) = directory.strip_prefix(repo_root) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !relative.is_empty() {
                patterns.insert(format!("{}/**", relative));
            }
        }
    }

    let has_parallel_resources =
        locale_directories
            .iter()
            .enumerate()
            .any(|(index, (_, names))| {
                locale_directories
                    .iter()
                    .skip(index + 1)
                    .any(|(_, other_names)| names.iter().any(|name| other_names.contains(name)))
            });
    if has_parallel_resources {
        for (locale_directory, _) in &locale_directories {
            if let Ok(relative) = locale_directory.strip_prefix(repo_root) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                patterns.insert(format!("{}/**", relative));
            }
        }
    }

    for child in child_directories {
        scan_localization_directories(repo_root, &child, depth + 1, visited, patterns);
    }
}

fn localization_resource_names(directory: &Path) -> BTreeSet<String> {
    let Ok(entries) = fs::read_dir(directory) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            Some(path.file_name()?.to_string_lossy().to_lowercase())
        })
        .collect()
}

fn locale_code_from_file(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    if !matches!(
        extension.as_str(),
        "json" | "json5" | "yaml" | "yml" | "ts" | "js" | "mjs" | "cjs" | "properties"
    ) {
        return None;
    }
    let stem = path.file_stem()?.to_string_lossy().to_lowercase();
    let candidate = stem.rsplit('.').next()?.replace('_', "-");
    normalize_locale_code(&candidate)
}

fn normalize_locale_code(candidate: &str) -> Option<String> {
    let language = candidate.split('-').next()?;
    const LANGUAGE_CODES: &[&str] = &[
        "ar", "bg", "ca", "cs", "da", "de", "el", "en", "es", "et", "fa", "fi", "fr", "he", "hi", "hr", "hu", "id", "it", "ja", "ko", "lt", "lv", "ms", "nb", "nl", "nn", "pl", "pt", "ro", "ru", "sk", "sl", "sr", "sv", "th", "tr", "uk", "vi", "zh", "ara", "deu", "eng", "fra", "ita", "jpn", "kor", "por", "rus", "spa", "zho",
    ];
    LANGUAGE_CODES
        .contains(&language)
        .then_some(candidate.to_string())
}

fn read_attention_weights(value: &Value) -> BTreeMap<String, i32> {
    value
        .get("review")
        .and_then(|review| review.get("attentionWeights"))
        .and_then(Value::as_object)
        .map(|weights| {
            weights
                .iter()
                .filter_map(|(kind, weight)| {
                    weight
                        .as_i64()
                        .map(|weight| (kind.clone(), weight as i32))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_role_patterns(value: &Value) -> Vec<(String, Vec<String>)> {
    let mut roles = Vec::new();
    for role in ["generated", "tooling", "primary", "secondary"] {
        if let Some(patterns) = value
            .get("roles")
            .and_then(|roles| roles.get(role))
            .and_then(Value::as_array)
        {
            let patterns = patterns
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            if !patterns.is_empty() {
                roles.push((role.to_string(), patterns));
            }
        }
    }
    roles
}

fn read_scope_patterns(value: &Value) -> Vec<(String, Vec<String>)> {
    value
        .get("scopes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let patterns = item
                        .get("patterns")?
                        .as_array()?
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    (!patterns.is_empty()).then_some((name, patterns))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
