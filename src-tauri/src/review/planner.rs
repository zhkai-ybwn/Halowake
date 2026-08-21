use std::{collections::BTreeMap, fs, hash::{Hash, Hasher}, path::Path};

use crate::{git::{prompt::build_review_attention, runner}, storage::AppDatabase};

use super::{
    models::{ReviewBudgetMode, ReviewFileRecord, ReviewFinding, ReviewRule, ScoreBreakdownItem},
    repository,
};

#[derive(Clone, Debug)]
pub struct ReviewFileContext {
    pub path: String,
    pub diff: String,
    pub changed_lines: Vec<usize>,
    pub source_context: String,
    pub source_content: String,
    pub source_complete: bool,
    pub matched_rules: Vec<ReviewRule>,
}

#[derive(Clone, Debug)]
pub struct ReviewBatch {
    pub id: String,
    pub files: Vec<ReviewFileContext>,
}

#[derive(Clone, Debug)]
pub struct ReviewPlan {
    pub repo_root: String,
    pub fingerprint: String,
    pub files: Vec<ReviewFileRecord>,
    pub batches: Vec<ReviewBatch>,
    pub rules: Vec<ReviewRule>,
    pub deterministic_findings: Vec<ReviewFinding>,
    pub limitations: Vec<String>,
}

pub fn build_plan(
    database: &AppDatabase,
    repo_path: &str,
    selected_files: &[String],
    mode: ReviewBudgetMode,
) -> Result<ReviewPlan, String> {
    if selected_files.is_empty() {
        return Err("请至少勾选一个文件进行 Review".to_string());
    }
    let repo_root = runner::run_git(repo_path, &["rev-parse", "--show-toplevel"])?;
    let attention = build_review_attention(&repo_root, selected_files)?;
    let attention_by_path = attention.files.into_iter().map(|file| {
        (normalize_path(&file.path), file)
    }).collect::<BTreeMap<_, _>>();
    let rules = merge_rules(database, &repo_root)?;
    let per_file_limit = (mode.input_chars() / selected_files.len().max(1)).max(1_500);
    let mut contexts = Vec::new();
    let mut files = Vec::new();
    let mut limitations = Vec::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    repo_root.hash(&mut hasher);

    for path in selected_files {
        let raw_diff = runner::load_selected_file_diff(&repo_root, path)?;
        path.hash(&mut hasher);
        raw_diff.hash(&mut hasher);
        let changed_lines = parse_changed_lines(&raw_diff);
        let source_content = match read_workspace_source(&repo_root, path) {
            Ok(source) => source,
            Err(error) => {
                if !raw_diff.contains("deleted file mode") { limitations.push(error); }
                String::new()
            }
        };
        let diff_limit = if source_content.is_empty() { per_file_limit } else { per_file_limit * 2 / 5 };
        let source_limit = per_file_limit.saturating_sub(diff_limit);
        let (diff, clipped) = clip_chars(&raw_diff, diff_limit.max(1_500));
        let (source_context, source_complete) = build_source_context(&source_content, &changed_lines, source_limit);
        if clipped {
            limitations.push(format!("{} 的 diff 已按 Token 预算截断", path));
        }
        if !source_content.is_empty() && !source_complete {
            limitations.push(format!("{} 的源文件上下文已按 Token 预算保留关键窗口", path));
        }
        let matched_rules = rules
            .iter()
            .filter(|rule| rule.enabled && rule_matches_path(rule, path))
            .cloned()
            .collect::<Vec<_>>();
        let attention = attention_by_path.get(&normalize_path(path));
        let categories = attention.map(|value| value.categories.clone()).unwrap_or_default();
        let attention_score = attention.map(|value| value.score).unwrap_or_default();
        let breakdown = attention.map(|value| value.score_breakdown.iter().map(|item| ScoreBreakdownItem {
            factor: item.factor.clone(),
            delta: item.delta,
            evidence: item.evidence.clone(),
        }).collect()).unwrap_or_default();
        files.push(ReviewFileRecord {
            path: path.clone(),
            change_kind: detect_change_kind(&raw_diff),
            attention_score,
            score_categories: categories,
            score_breakdown: breakdown,
            selected: true,
            review_status: "pending".to_string(),
            batch_id: None,
            limitation: clipped.then(|| "diff clipped by review budget".to_string()),
        });
        contexts.push(ReviewFileContext {
            path: path.clone(), diff, changed_lines, source_context, source_content, source_complete, matched_rules,
        });
    }

    let batches = group_batches(contexts, mode);
    for batch in &batches {
        for context in &batch.files {
            if let Some(file) = files.iter_mut().find(|file| file.path == context.path) {
                file.batch_id = Some(batch.id.clone());
            }
        }
    }
    let deterministic_findings = run_deterministic_rules(&batches);

    Ok(ReviewPlan {
        repo_root,
        fingerprint: format!("{:016x}", hasher.finish()),
        files,
        batches,
        rules,
        deterministic_findings,
        limitations,
    })
}

fn group_batches(files: Vec<ReviewFileContext>, mode: ReviewBudgetMode) -> Vec<ReviewBatch> {
    // Compact reviews already cap the total selected diff at 24k characters.
    // Keeping that mode in one request avoids paying prompt/network latency twice.
    // Larger modes retain bounded batches so one slow response does not lose all work.
    let max_batch_chars = mode.input_chars().min(24_000).max(8_000);
    let mut scopes: BTreeMap<String, Vec<ReviewFileContext>> = BTreeMap::new();
    for file in files {
        let normalized = file.path.replace('\\', "/");
        let scope = normalized.split('/').take(2).collect::<Vec<_>>().join("/");
        scopes.entry(scope).or_default().push(file);
    }

    let mut batches = Vec::new();
    let mut current = Vec::new();
    let mut current_chars = 0;
    for (_, group) in scopes {
        let group_chars = group.iter().map(|file| file.diff.chars().count() + file.source_context.chars().count()).sum::<usize>();
        if !current.is_empty() && current_chars + group_chars > max_batch_chars {
            let id = format!("batch-{}", batches.len() + 1);
            batches.push(ReviewBatch { id, files: std::mem::take(&mut current) });
            current_chars = 0;
        }
        current_chars += group_chars;
        current.extend(group);
    }
    if !current.is_empty() {
        let id = format!("batch-{}", batches.len() + 1);
        batches.push(ReviewBatch { id, files: current });
    }
    batches
}

fn merge_rules(database: &AppDatabase, repo_root: &str) -> Result<Vec<ReviewRule>, String> {
    let mut merged = BTreeMap::new();
    for rule in builtin_rules() {
        merged.insert(rule.id.clone(), rule);
    }
    for rule in repository::list_rules(database)? {
        merged.insert(rule.id.clone(), rule);
    }
    let profile_path = Path::new(repo_root).join(".lumina").join("project-profile.json");
    if let Ok(content) = fs::read_to_string(profile_path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(rules) = value.pointer("/review/rules").and_then(|value| value.as_array()) {
                for value in rules {
                    if let Ok(mut rule) = serde_json::from_value::<ReviewRule>(value.clone()) {
                        rule.source = "project".to_string();
                        merged.insert(rule.id.clone(), rule);
                    }
                }
            }
        }
    }
    Ok(merged.into_values().collect())
}

fn builtin_rules() -> Vec<ReviewRule> {
    vec![
        deterministic_rule("builtin.debug-log", "调试日志", "minor", "maintainability", "contains", "console.log"),
        deterministic_rule("builtin.hardcoded-secret", "可能的硬编码凭据", "critical", "security", "regex", "(?i)(api[_-]?key|token|secret|password)\\s*[:=]\\s*['\\\"][^'\\\"]{8,}"),
    ]
}

fn deterministic_rule(id: &str, name: &str, severity: &str, category: &str, operator: &str, pattern: &str) -> ReviewRule {
    ReviewRule {
        id: id.to_string(), name: name.to_string(), description: None,
        kind: "deterministic".to_string(), enabled: true, severity: severity.to_string(),
        category: category.to_string(), include_globs: vec!["**".to_string()], exclude_globs: Vec::new(),
        languages: Vec::new(), definition: serde_json::json!({"target":"added-lines","operator":operator,"pattern":pattern}),
        source: "builtin".to_string(), version: 1,
    }
}

fn run_deterministic_rules(batches: &[ReviewBatch]) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    for batch in batches {
        for file in &batch.files {
            for rule in file.matched_rules.iter().filter(|rule| rule.kind == "deterministic") {
                let operator = rule.definition.get("operator").and_then(|value| value.as_str()).unwrap_or("contains");
                let pattern = rule.definition.get("pattern").and_then(|value| value.as_str()).unwrap_or("");
                for (line, text) in added_lines(&file.diff) {
                    let matched = match operator {
                        "contains" => text.contains(pattern),
                        "regex" => simple_secret_match(pattern, &text),
                        _ => false,
                    };
                    if matched {
                        let fingerprint = format!("{}:{}:{}", rule.id, file.path, line);
                        findings.push(ReviewFinding {
                            id: format!("local-{}", stable_hash(&fingerprint)), fingerprint,
                            source: "deterministic".to_string(), rule_id: Some(rule.id.clone()),
                            category: rule.category.clone(), severity: rule.severity.clone(), confidence: 1.0,
                            file_path: file.path.clone(), start_line: line, end_line: line,
                            title: rule.name.clone(), problem: format!("新增代码触发规则 {}", rule.name),
                            impact: rule.description.clone().unwrap_or_else(|| "可能违反项目审查规则".to_string()),
                            trigger_scenario: "当该新增行进入当前代码路径时".to_string(),
                            evidence: text.trim().chars().take(240).collect(), suggestion: None,
                            verified: true, status: "open".to_string(), user_note: None,
                        });
                    }
                }
            }
        }
    }
    findings
}

fn rule_matches_path(rule: &ReviewRule, path: &str) -> bool {
    let path = path.replace('\\', "/");
    let included = rule.include_globs.is_empty() || rule.include_globs.iter().any(|pattern| wildcard_match(pattern, &path));
    included && !rule.exclude_globs.iter().any(|pattern| wildcard_match(pattern, &path))
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern == "**" || pattern == "*" { return true; }
    let parts = pattern.split('*').filter(|part| !part.is_empty()).collect::<Vec<_>>();
    if parts.is_empty() { return true; }
    let mut cursor = 0;
    for part in parts {
        let Some(offset) = value[cursor..].find(part) else { return false; };
        cursor += offset + part.len();
    }
    true
}

fn parse_changed_lines(diff: &str) -> Vec<usize> { added_lines(diff).into_iter().map(|(line, _)| line).collect() }

fn added_lines(diff: &str) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let mut new_line = 0usize;
    for line in diff.lines() {
        if line.starts_with("@@") {
            if let Some(range) = line.split_whitespace().find(|part| part.starts_with('+')) {
                new_line = range.trim_start_matches('+').split(',').next().and_then(|value| value.parse().ok()).unwrap_or(0);
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            result.push((new_line, line.trim_start_matches('+').to_string()));
            new_line += 1;
        } else if !line.starts_with('-') { new_line += 1; }
    }
    result
}

fn clip_chars(value: &str, limit: usize) -> (String, bool) {
    if value.chars().count() <= limit { return (value.to_string(), false); }
    let marker = "\n...<middle of diff clipped by Lumina review budget>...\n";
    let available = limit.saturating_sub(marker.chars().count());
    let head_limit = available * 3 / 5;
    let tail_limit = available.saturating_sub(head_limit);
    let head = value.chars().take(head_limit).collect::<String>();
    let tail = value.chars().rev().take(tail_limit).collect::<Vec<_>>().into_iter().rev().collect::<String>();
    let clipped = format!("{head}{marker}{tail}");
    (clipped, true)
}

fn read_workspace_source(repo_root: &str, path: &str) -> Result<String, String> {
    let root = fs::canonicalize(repo_root).map_err(|error| format!("解析仓库目录失败: {error}"))?;
    let candidate = root.join(path);
    let resolved = fs::canonicalize(&candidate).map_err(|error| format!("读取源文件失败 {}: {error}", path))?;
    if !resolved.starts_with(&root) {
        return Err(format!("源文件路径超出仓库范围: {path}"));
    }
    fs::read_to_string(resolved).map_err(|error| format!("读取源文件失败 {}: {error}", path))
}

fn build_source_context(source: &str, changed_lines: &[usize], limit: usize) -> (String, bool) {
    if source.is_empty() || limit == 0 { return (String::new(), false); }
    let lines = source.lines().collect::<Vec<_>>();
    let render = |selected: Option<&std::collections::BTreeSet<usize>>| {
        let mut output = String::new();
        let mut previous = 0usize;
        for (index, text) in lines.iter().enumerate() {
            let line = index + 1;
            if let Some(items) = selected {
                if !items.contains(&line) { continue; }
            }
            if previous > 0 && line > previous + 1 { output.push_str("... source lines omitted ...\n"); }
            let marker = if changed_lines.contains(&line) { "+" } else { " " };
            output.push_str(&format!("{marker}{line:>6} | {text}\n"));
            previous = line;
        }
        output
    };
    let full = render(None);
    if full.chars().count() <= limit { return (full, true); }

    let mut selected = std::collections::BTreeSet::new();
    for line in 1..=lines.len().min(40) { selected.insert(line); }
    for line in lines.len().saturating_sub(80).max(1)..=lines.len() { selected.insert(line); }
    for changed in changed_lines {
        let start = changed.saturating_sub(35).max(1);
        let end = changed.saturating_add(35).min(lines.len());
        for line in start..=end { selected.insert(line); }
    }
    let contextual = render(Some(&selected));
    if contextual.chars().count() <= limit { return (contextual, false); }
    let marker = "\n...<source context clipped by Lumina review budget>...\n";
    let available = limit.saturating_sub(marker.chars().count());
    let head_limit = available * 3 / 5;
    let tail_limit = available.saturating_sub(head_limit);
    let head = contextual.chars().take(head_limit).collect::<String>();
    let tail = contextual.chars().rev().take(tail_limit).collect::<Vec<_>>().into_iter().rev().collect::<String>();
    (format!("{head}{marker}{tail}"), false)
}

fn normalize_path(path: &str) -> String { path.replace('\\', "/").to_lowercase() }

fn detect_change_kind(diff: &str) -> String {
    if diff.contains("new file mode") { "added" } else if diff.contains("deleted file mode") { "deleted" } else if diff.contains("rename from") { "renamed" } else { "modified" }.to_string()
}

fn simple_secret_match(_pattern: &str, text: &str) -> bool {
    let lower = text.to_lowercase();
    let has_name = ["api_key", "apikey", "token", "secret", "password"].iter().any(|term| lower.contains(term));
    has_name && (text.contains('=') || text.contains(':')) && (text.contains('"') || text.contains('\''))
}

fn stable_hash(value: &str) -> u64 { let mut hasher = std::collections::hash_map::DefaultHasher::new(); value.hash(&mut hasher); hasher.finish() }
