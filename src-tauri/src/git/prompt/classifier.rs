use std::collections::BTreeMap;

use crate::git::{parser::parse_git_status_line, runner};

use super::{
    EvidenceLine, FileClassification, PreparedPromptFile, PromptBudgetPlan, PromptProfile,
    MAX_TOTAL_EVIDENCE_CHARS, MIN_EVIDENCE_SCORE,
};

pub(super) fn load_file_actions(repo_path: &str) -> BTreeMap<String, String> {
    let Ok(status_raw) = runner::run_git_raw(
        repo_path,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    ) else {
        return BTreeMap::new();
    };

    status_raw
        .lines()
        .map(parse_git_status_line)
        .map(|file| (normalize_match_path(&file.path), file.change_type))
        .collect()
}

pub(super) fn detect_action_from_diff_header(diff: &str) -> String {
    if diff.contains("new file mode") {
        "added".to_string()
    } else if diff.contains("deleted file mode") {
        "deleted".to_string()
    } else if diff.contains("rename from ") || diff.contains("rename to ") {
        "renamed".to_string()
    } else {
        "modified".to_string()
    }
}

pub(super) fn normalize_match_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .to_lowercase()
}

pub(super) fn classify_file(
    path: &str,
    profile: &PromptProfile,
    action: &str,
) -> FileClassification {
    let kind = if profile
        .localization_patterns
        .iter()
        .any(|pattern| matches_pattern(path, pattern))
    {
        "i18n".to_string()
    } else {
        classify_kind(path)
    };
    let role = if kind == "internal" {
        "internal".to_string()
    } else {
        first_matching_group(path, &profile.roles).unwrap_or_else(|| "secondary".to_string())
    };
    let scope = if kind == "internal" {
        "lumina".to_string()
    } else {
        first_matching_group(path, &profile.scopes).unwrap_or_else(|| "root".to_string())
    };
    let (strategy, max_lines, skip_verbose) = strategy_for(&role, &kind, action);

    FileClassification {
        role,
        scope,
        kind,
        strategy,
        max_lines,
        skip_verbose,
    }
}

pub(super) fn build_budget_plan(files: &[PreparedPromptFile]) -> Vec<PromptBudgetPlan> {
    let mut weights = BTreeMap::<String, usize>::new();
    for file in files {
        let weight = group_weight(&file.classification);
        if weight == 0 {
            continue;
        }
        *weights.entry(file.group_key()).or_insert(0) += weight;
    }

    let total_weight = weights.values().sum::<usize>().max(1);
    weights
        .into_iter()
        .map(|(group_key, weight)| {
            let budget_chars = ((MAX_TOTAL_EVIDENCE_CHARS * weight) / total_weight).max(160);
            PromptBudgetPlan {
                group_key,
                budget_chars,
                weight,
            }
        })
        .collect()
}

pub(super) fn apply_group_budgets(
    files: &mut [PreparedPromptFile],
    budget_plan: &[PromptBudgetPlan],
) {
    let budgets = budget_plan
        .iter()
        .map(|item| (item.group_key.clone(), item.budget_chars))
        .collect::<BTreeMap<_, _>>();
    let mut group_candidates = BTreeMap::<String, Vec<(usize, EvidenceLine)>>::new();

    for (file_index, file) in files.iter().enumerate() {
        for candidate in &file.candidates {
            group_candidates
                .entry(file.group_key())
                .or_default()
                .push((file_index, candidate.clone()));
        }
    }

    for (group_key, mut candidates) in group_candidates {
        candidates.sort_by(|left, right| {
            right
                .1
                .score
                .cmp(&left.1.score)
                .then_with(|| left.0.cmp(&right.0))
        });

        let mut used_chars = 0;
        let budget = budgets.get(&group_key).copied().unwrap_or(160);
        let per_file_budget =
            (budget / files_in_group(files, &group_key).max(1)).clamp(120, 900);
        let mut file_chars = BTreeMap::<usize, usize>::new();
        for (file_index, candidate) in candidates {
            if candidate.score < min_evidence_score(&files[file_index].classification) {
                continue;
            }
            let line_chars = candidate.text.chars().count() + 1;
            if used_chars + line_chars > budget {
                continue;
            }
            let used_by_file = file_chars.get(&file_index).copied().unwrap_or_default();
            if used_by_file + line_chars > per_file_budget {
                continue;
            }
            if files[file_index].selected.len() >= files[file_index].classification.max_lines {
                continue;
            }
            used_chars += line_chars;
            file_chars.insert(file_index, used_by_file + line_chars);
            files[file_index].selected.push(candidate);
        }
    }

    for file in files {
        file.selected.sort_by_key(|line| line.line_index);
    }
}

pub(super) fn preserve_small_source_changes(files: &mut [PreparedPromptFile]) {
    for file in files {
        if file.classification.kind != "source"
            || !file.selected.is_empty()
            || file.candidates.is_empty()
            || file.candidates.len() > 6
        {
            continue;
        }

        if let Some(candidate) = file.candidates.iter().max_by_key(|line| line.score) {
            file.selected.push(candidate.clone());
        }
    }
}

pub(super) fn group_weight(classification: &FileClassification) -> usize {
    let role_weight = match classification.role.as_str() {
        "primary" => 8,
        "tooling" => 4,
        "secondary" => 3,
        "generated" => 1,
        "internal" => 0,
        _ => 2,
    };
    let kind_weight = match classification.kind.as_str() {
        "source" => 8,
        "config" => 4,
        "script" => 4,
        "test" => 3,
        "docs" => 3,
        "i18n" | "style" => 2,
        "entry" | "ignore" => 2,
        "lockfile" | "asset" => 1,
        "internal" => 0,
        _ => 2,
    };

    role_weight * kind_weight
}

fn first_matching_group(path: &str, groups: &[(String, Vec<String>)]) -> Option<String> {
    groups.iter().find_map(|(name, patterns)| {
        patterns
            .iter()
            .any(|pattern| matches_pattern(path, pattern))
            .then(|| name.clone())
    })
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    let path = normalize_match_path(path);
    let pattern = normalize_match_path(pattern);

    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{}/", prefix));
    }

    if !pattern.contains('*') {
        return path == pattern || path.ends_with(&format!("/{}", pattern));
    }

    let parts = pattern
        .split('*')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return true;
    }

    let mut cursor = 0;
    for part in parts {
        let Some(index) = path[cursor..].find(part) else {
            return false;
        };
        cursor += index + part.len();
    }

    true
}

fn classify_kind(path: &str) -> String {
    let lower = normalize_match_path(path);
    let ext = lower.rsplit('.').next().unwrap_or("");

    if lower.starts_with(".lumina/") {
        "internal".to_string()
    } else if lower.ends_with("package-lock.json")
        || lower.ends_with("pnpm-lock.yaml")
        || lower.ends_with("yarn.lock")
        || lower.ends_with("cargo.lock")
    {
        "lockfile".to_string()
    } else if lower.contains(".test.") || lower.contains(".spec.") || lower.contains("/tests/") {
        "test".to_string()
    } else if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "vue" | "rs") {
        "source".to_string()
    } else if matches!(ext, "css" | "scss" | "less" | "sass") {
        "style".to_string()
    } else if matches!(
        ext,
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "ico"
            | "svg"
            | "webp"
            | "avif"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "mp3"
            | "mp4"
            | "webm"
    ) {
        "asset".to_string()
    } else if lower.ends_with(".gitignore") {
        "ignore".to_string()
    } else if ext == "ps1" {
        "script".to_string()
    } else if ext == "html" {
        "entry".to_string()
    } else if matches!(
        ext,
        "json" | "toml" | "yaml" | "yml" | "cjs" | "mjs"
    ) || lower.contains("config")
    {
        "config".to_string()
    } else if matches!(ext, "md" | "txt") {
        "docs".to_string()
    } else {
        "other".to_string()
    }
}

fn strategy_for(role: &str, kind: &str, action: &str) -> (String, usize, bool) {
    if role == "internal"
        || role == "generated"
        || matches!(kind, "internal" | "lockfile" | "asset")
    {
        return ("summarize only".to_string(), 1, true);
    }

    if matches!(action, "added" | "untracked") {
        return ("new file structural summary".to_string(), 12, false);
    }
    if matches!(action, "deleted" | "renamed") {
        return ("file-level change summary".to_string(), 2, false);
    }

    match (role, kind) {
        ("primary", "source") => ("primary source evidence".to_string(), 40, false),
        ("tooling", _) | (_, "config") => ("config/tooling evidence".to_string(), 18, false),
        (_, "style" | "i18n" | "docs" | "test") => {
            ("reduced supporting evidence".to_string(), 14, false)
        }
        _ => ("limited evidence".to_string(), 18, false),
    }
}

fn files_in_group(files: &[PreparedPromptFile], group_key: &str) -> usize {
    files
        .iter()
        .filter(|file| file.group_key() == group_key)
        .count()
}

fn min_evidence_score(classification: &FileClassification) -> i32 {
    if matches!(
        classification.kind.as_str(),
        "lockfile" | "asset" | "internal"
    ) {
        return i32::MAX;
    }

    match (
        classification.role.as_str(),
        classification.kind.as_str(),
    ) {
        ("primary", "source") => 70,
        (_, "source") => 40,
        (_, "config" | "script") => 36,
        (_, "docs") => 28,
        (_, "i18n" | "style" | "test") => 40,
        _ => MIN_EVIDENCE_SCORE,
    }
}
