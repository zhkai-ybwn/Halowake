use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{json, Value};

use crate::git::analyzer::build_analysis_context;
use crate::git::config::AnalysisConfig;
use crate::git::models::{
    GitAiPayload, GitCommitPromptFileTrace, GitCommitPromptPreview, GitCommitPromptTrace,
    GitReviewAttention, GitReviewAttentionResult, GitReviewScoreBreakdown,
};
use crate::git::parser::parse_git_status_line;
use crate::git::runner;

const MAX_TOTAL_EVIDENCE_CHARS: usize = 12000;
const MAX_CANDIDATE_LINES_PER_FILE: usize = 80;
const MIN_EVIDENCE_SCORE: i32 = 40;

pub fn build_review_attention(
    repo_path: &str,
    selected_files: &[String],
) -> Result<GitReviewAttentionResult, String> {
    build_review_attention_with_progress(repo_path, selected_files, |_, _, _, _| {})
}

pub fn build_review_attention_with_progress<F>(
    repo_path: &str,
    selected_files: &[String],
    mut on_progress: F,
) -> Result<GitReviewAttentionResult, String>
where
    F: FnMut(usize, usize, &str, Option<&str>),
{
    let profile = load_prompt_profile(repo_path);
    let action_map = load_file_actions(repo_path);
    let mut files = Vec::new();
    let total = selected_files.len();
    on_progress(0, total, "profile", None);

    for (index, file_path) in selected_files.iter().enumerate() {
        on_progress(index, total, "cleaning", Some(file_path));
        let action = action_map
            .get(&normalize_match_path(file_path))
            .cloned()
            .unwrap_or_else(|| "modified".to_string());
        let classification = classify_file(file_path, &profile, &action);
        if let Some(category) = review_skip_category(&classification) {
            files.push(GitReviewAttention {
                path: file_path.clone(),
                score: 0,
                categories: vec![category.to_string()],
                score_breakdown: vec![GitReviewScoreBreakdown {
                    factor: "skip".to_string(),
                    delta: 0,
                    evidence: category.to_string(),
                }],
                eligible: false,
                skipped: true,
            });
            on_progress(index + 1, total, "skipped", Some(file_path));
            continue;
        }
        let raw_diff = runner::load_selected_file_diff(repo_path, file_path)?;
        let action = if action == "modified" {
            detect_action_from_diff_header(&raw_diff)
        } else {
            action
        };
        let cleaned = clean_diff_candidates(file_path, &raw_diff, &classification, &action);
        let evidence_count = cleaned.candidates.len();
        let cleaned_chars = cleaned
            .candidates
            .iter()
            .map(|line| line.text.chars().count())
            .sum::<usize>();
        let changed_lines = raw_diff
            .lines()
            .filter(|line| {
                (line.starts_with('+') && !line.starts_with("+++"))
                    || (line.starts_with('-') && !line.starts_with("---"))
            })
            .count();
        let categories = review_categories(file_path, &classification, &raw_diff);
        let score = score_review_attention(
            &profile,
            &classification,
            &action,
            evidence_count,
            cleaned_chars,
            changed_lines,
            &categories,
            cleaned.skipped,
        );
        let (_, score_breakdown) = score_review_attention_details(
            &profile,
            &classification,
            &action,
            evidence_count,
            cleaned_chars,
            changed_lines,
            &categories,
            cleaned.skipped,
        );

        files.push(GitReviewAttention {
            path: file_path.clone(),
            score,
            categories,
            score_breakdown,
            eligible: score >= 50 && !cleaned.skipped,
            skipped: false,
        });
        on_progress(index + 1, total, "scoring", Some(file_path));
    }

    files.sort_by(|left, right| right.score.cmp(&left.score).then_with(|| left.path.cmp(&right.path)));
    on_progress(total, total, "complete", None);
    Ok(GitReviewAttentionResult { files })
}

fn review_skip_category(classification: &FileClassification) -> Option<&'static str> {
    if classification.role == "generated" {
        return Some("generated");
    }

    match classification.kind.as_str() {
        "asset" => Some("resource"),
        "i18n" => Some("i18n"),
        "lockfile" => Some("dependency"),
        "internal" => Some("generated"),
        "docs" => Some("docs"),
        _ => None,
    }
}

fn score_review_attention(
    profile: &PromptProfile,
    classification: &FileClassification,
    action: &str,
    evidence_count: usize,
    cleaned_chars: usize,
    changed_lines: usize,
    categories: &[String],
    skipped: bool,
) -> i32 {
    let mut score = 8;

    match classification.role.as_str() {
        "primary" => { score += 18; }
        "tooling" => { score += 12; }
        "secondary" => { score += 4; }
        "generated" | "internal" => { score -= 18; }
        _ => {}
    }

    if let Some(weight) = profile.attention_weights.get(&classification.kind) {
        score += *weight;
    }

    for category in categories {
        score += match category.as_str() {
            "security" => 18,
            "data" => 14,
            "api" => 10,
            "logic" => 10,
            "types" => 7,
            "config" => 8,
            "markup" => 5,
            "style" => 3,
            "test" => 2,
            _ => 0,
        };
    }

    match action {
        "deleted" | "renamed" => { score += 10; }
        "added" | "untracked" => { score += 6; }
        _ => {}
    }

    if changed_lines >= 180 {
        score += 16;
    } else if changed_lines >= 60 {
        score += 9;
    } else if changed_lines >= 20 {
        score += 4;
    }

    if skipped {
        score -= 24;
    } else if evidence_count >= 24 || cleaned_chars >= 1800 {
        score += 18;
    } else if evidence_count >= 8 || cleaned_chars >= 600 {
        score += 10;
    } else if evidence_count == 0 {
        score -= 10;
    }

    score.clamp(0, 100)
}

fn score_review_attention_details(
    profile: &PromptProfile,
    classification: &FileClassification,
    action: &str,
    evidence_count: usize,
    cleaned_chars: usize,
    changed_lines: usize,
    categories: &[String],
    skipped: bool,
) -> (i32, Vec<GitReviewScoreBreakdown>) {
    let mut score = 8;
    let mut breakdown = vec![GitReviewScoreBreakdown {
        factor: "base".to_string(),
        delta: 8,
        evidence: "基础关注度".to_string(),
    }];

    let role_delta = match classification.role.as_str() {
        "primary" => 18,
        "tooling" => 12,
        "secondary" => 4,
        "generated" | "internal" => -18,
        _ => 0,
    };
    score += role_delta;
    breakdown.push(GitReviewScoreBreakdown {
        factor: "file-role".to_string(),
        delta: role_delta,
        evidence: classification.role.clone(),
    });

    if let Some(weight) = profile.attention_weights.get(&classification.kind) {
        score += *weight;
        breakdown.push(GitReviewScoreBreakdown {
            factor: "file-kind".to_string(),
            delta: *weight,
            evidence: classification.kind.clone(),
        });
    }

    for category in categories {
        let delta = match category.as_str() {
            "security" => 18,
            "data" => 14,
            "api" => 10,
            "logic" => 10,
            "types" => 7,
            "config" => 8,
            "markup" => 5,
            "style" => 3,
            "test" => 2,
            _ => 0,
        };
        score += delta;
        if delta != 0 {
            breakdown.push(GitReviewScoreBreakdown {
                factor: format!("category-{category}"),
                delta,
                evidence: category.clone(),
            });
        }
    }

    let action_delta = match action {
        "deleted" | "renamed" => 10,
        "added" | "untracked" => 6,
        _ => 0,
    };
    score += action_delta;
    if action_delta != 0 {
        breakdown.push(GitReviewScoreBreakdown {
            factor: "change-action".to_string(),
            delta: action_delta,
            evidence: action.to_string(),
        });
    }

    let size_delta = if changed_lines >= 180 {
        16
    } else if changed_lines >= 60 {
        9
    } else if changed_lines >= 20 {
        4
    } else {
        0
    };
    score += size_delta;
    if size_delta != 0 {
        breakdown.push(GitReviewScoreBreakdown {
            factor: "change-size".to_string(),
            delta: size_delta,
            evidence: format!("{changed_lines} changed lines"),
        });
    }

    let evidence_delta = if skipped {
        -24
    } else if evidence_count >= 24 || cleaned_chars >= 1800 {
        18
    } else if evidence_count >= 8 || cleaned_chars >= 600 {
        10
    } else if evidence_count == 0 {
        -10
    } else {
        0
    };
    score += evidence_delta;
    if evidence_delta != 0 {
        breakdown.push(GitReviewScoreBreakdown {
            factor: "review-evidence".to_string(),
            delta: evidence_delta,
            evidence: format!("{evidence_count} evidence lines, {cleaned_chars} chars"),
        });
    }

    let clamped = score.clamp(0, 100);
    if clamped != score {
        breakdown.push(GitReviewScoreBreakdown {
            factor: "score-limit".to_string(),
            delta: clamped - score,
            evidence: "关注度分数限制在 0-100".to_string(),
        });
    }
    (clamped, breakdown)
}

fn review_categories(path: &str, classification: &FileClassification, diff: &str) -> Vec<String> {
    let lower_path = normalize_match_path(path);
    let content = diff.to_lowercase();
    let mut categories = Vec::new();

    if ["auth", "permission", "authorization", "password", "token", "secret", "role"].iter().any(|term| content.contains(term)) {
        push_category(&mut categories, "security");
    }
    if ["migration", "database", "sql", "storage", "repository", "persist"].iter().any(|term| content.contains(term)) {
        push_category(&mut categories, "data");
    }
    if ["#[tauri::command]", "route", "endpoint", "controller", "invoke(", "fetch(", "axios"].iter().any(|term| content.contains(term)) {
        push_category(&mut categories, "api");
    }
    if ["interface ", "type ", "struct ", "enum ", "trait ", "schema"].iter().any(|term| content.contains(term)) {
        push_category(&mut categories, "types");
    }
    if classification.kind == "config" {
        push_category(&mut categories, "config");
    }
    if classification.kind == "test" {
        push_category(&mut categories, "test");
    }
    if classification.kind == "style" || content.contains("<style") {
        push_category(&mut categories, "style");
    }
    if lower_path.ends_with(".html")
        || content.contains("<template")
        || ([".vue", ".tsx", ".jsx"].iter().any(|ext| lower_path.ends_with(ext))
            && ["<div", "<section", "<button", "<input", "<form"].iter().any(|term| content.contains(term)))
    {
        push_category(&mut categories, "markup");
    }
    if classification.kind == "source"
        && (categories.is_empty()
            || [" if ", "match ", "for ", "while ", "await ", "async ", "return ", "=>", "fn ", "function "]
                .iter()
                .any(|term| content.contains(term)))
    {
        push_category(&mut categories, "logic");
    }

    categories
}

fn push_category(categories: &mut Vec<String>, category: &str) {
    if !categories.iter().any(|item| item == category) {
        categories.push(category.to_string());
    }
}

pub fn build_analysis_schema(language: &str) -> Value {
    let (title_desc, body_desc, summary_desc, risks_desc) = match language {
        "zh" => (
            "Conventional commit 风格标题，简洁的中文描述",
            "3 到 6 条要点，每行以 \"- \" 开头，使用中文",
            "简洁的中文摘要，描述主要业务变更，不要罗列文件名",
            "中文风险提示，1 到 5 条",
        ),
        _ => (
            "Conventional commit style title, concise and in English",
            "3 to 6 bullet lines separated by newline, in English",
            "A concise Chinese summary of the main business change line, not a raw file list",
            "Chinese risk notes, 1 to 5 items",
        ),
    };
    json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": title_desc
            },
            "body": {
                "type": "string",
                "description": body_desc
            },
            "summary": {
                "type": "string",
                "description": summary_desc
            },
            "risks": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "description": risks_desc
            }
        },
        "required": ["title", "body", "summary", "risks"]
    })
}

pub fn join_preview(items: &[String], limit: usize) -> String {
    if items.is_empty() {
        return "(none)".to_string();
    }

    let mut result = items
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    if items.len() > limit {
        result.push_str(&format!("\n...and {} more", items.len() - limit));
    }

    result
}

pub fn build_analysis_prompt(payload: &GitAiPayload) -> String {
    let language = payload.language.as_deref().unwrap_or("en");
    let config = AnalysisConfig::default();
    let context = build_analysis_context(&payload.status);
    let mut staged_diff_preview = payload.staged_diff.clone();
    if staged_diff_preview.len() > config.max_diff_length {
        staged_diff_preview.truncate(config.max_diff_length);
        staged_diff_preview.push_str("\n...<truncated>");
    }

    let schema_text = build_analysis_schema(language).to_string();
    let context_json = serde_json::to_string_pretty(&context).unwrap_or_else(|_| "{}".to_string());
    let scope_json =
        serde_json::to_string_pretty(&context.scope_summaries).unwrap_or_else(|_| "[]".to_string());
    let signal_json =
        serde_json::to_string_pretty(&context.diff_signals).unwrap_or_else(|_| "{}".to_string());

    let (lang_rule_6, lang_rule_7, lang_rule_16, lang_rule_17, lang_rule_18, title_examples) = match language {
        "zh" => (
            "6. summary 必须使用中文，描述主要业务变更，不要罗列文件名。",
            "7. risks 必须使用中文。",
            "16. title 必须使用中文。",
            "17. summary 和 risks 必须使用中文。",
            "18. 不要在 title 中使用英文。",
            r#"- feat(模块): 新增配置页面和相关国际化
- feat(模块): 支持日志查看和配置流程
- refactor(模块): 简化配置常量和页面接入"#,
        ),
        _ => (
            "6. summary must be in Simplified Chinese and should explain the main change line, not list filenames.",
            "7. risks must be in Simplified Chinese.",
            "16. title must be written in English only.",
            "17. summary and risks must be written in Simplified Chinese only.",
            "18. Do not use Chinese in the title.",
            r#"- feat(feature/module): add configuration pages and related i18n
- feat(app/module): support log view and configuration flow
- refactor(feature/module): simplify configuration constants and page wiring"#,
        ),
    };

    format!(
        r#"
You are a precise git commit assistant.

Return ONLY valid JSON that matches this schema:
{schema_text}

Your job is to infer the main business change line from the provided git snapshot.
Do NOT mechanically restate file paths.
Do NOT overemphasize supporting integration files.

Important rules:
1. Focus on business capability changes first.
2. Treat routes, i18n, constants, config files, generated files, and tooling files as supporting changes unless they clearly dominate the actual purpose.
3. When multiple scopes are changed, summarize the main feature-level intent first, then mention supporting integration changes if needed.
4. The title must reflect real intent instead of vague words like optimize, improve, or update, unless no more specific wording is justified.
5. The body must summarize feature-level changes, not file-by-file edits.
{lang_rule_6}
{lang_rule_7}
8. body must be plain text using 3-6 bullet lines, each line prefixed with "- ".
9. Do not include markdown fences.
10. Be grounded only in the provided data. Do not invent business details not supported by the git snapshot.
11. Avoid generic titles such as "Refactor and Add New Features in Multiple Modules", "Update several modules", or similar vague summaries.
12. The title should use conventional commit style, for example feat(scope): ..., refactor(scope): ..., fix(scope): ...
13. If multiple scopes are involved, choose the most representative one or combine at most two major scopes.
14. Prefer concrete capability words such as add pages, adjust configuration flow, update constants, support logs, integrate i18n, remove obsolete config.
15. Do not describe the change as "multiple modules" unless there is truly no stronger common intent.
{lang_rule_16}
{lang_rule_17}
{lang_rule_18}
19. The title should be a single conventional commit line and must not mention i18n unless it is a major part of the change.
20. Prefer title patterns like:
    {title_examples}
21. Supporting changes such as i18n, routes, and config wiring should usually stay in the body or summary, not in the title, unless they are the dominant change.

Heuristics:
- If there are clear primary scopes, prioritize them over secondary scopes.
- If feature code changes coexist with route/i18n/config updates, the latter are usually supporting changes.
- If new files or multiple related page/component/configuration files appear in the same scope, that often indicates a newly added feature or expanded capability.
- If constants/config changes appear together with feature code changes, mention them as supporting adjustments unless they are the dominant change.
- Prefer concise, specific intent verbs such as add, implement, support, refactor, fix, remove, or adjust.
- When page-like, detail/view-like, log-like, or configuration-like signals are present, prefer describing the change as added or adjusted pages, views, logs, or configuration flows instead of saying "updated module code".
- When constants changes appear with configuration or feature code changes, describe them as supporting constant adjustments rather than the main title focus.
- Avoid phrases like "updated related code", "updated module code", or "modified several files" unless absolutely necessary.

Good title examples:
- feat(feature/module): add configuration pages and related i18n
- feat(app/module): support log view and configuration flow
- refactor(feature/module): simplify configuration constants and page wiring

Bad title examples:
- Refactor and Add New Features in Multiple Modules
- Update several modules
- Improve business logic and configuration

Repository path:
{repo_path}

Branch:
{branch}

Structured analysis context:
{context_json}

Scope summaries:
{scope_json}

Diff signals:
{signal_json}

Primary files preview:
{primary_files}

Secondary files preview:
{secondary_files}

Generated files preview:
{generated_files}

Tooling files preview:
{tooling_files}

Untracked files preview:
{untracked_files}

Deleted files preview:
{deleted_files}

Main scopes:
{main_scopes}

Summary hint:
{summary_hint}

Staged files:
{staged_files}

Staged diff preview:
{staged_diff_preview}
"#,
        schema_text = schema_text,
        repo_path = payload.repo_path,
        branch = payload.branch,
        context_json = context_json,
        scope_json = scope_json,
        signal_json = signal_json,
        primary_files = join_preview(&context.primary_files, config.max_preview_items),
        secondary_files = join_preview(&context.secondary_files, config.max_preview_items),
        generated_files = join_preview(&context.generated_files, config.max_preview_items),
        tooling_files = join_preview(&context.tooling_files, config.max_preview_items),
        untracked_files = join_preview(&context.untracked_files, config.max_preview_items),
        deleted_files = join_preview(&context.deleted_files, config.max_preview_items),
        main_scopes = if context.main_scopes.is_empty() {
            "(none)".to_string()
        } else {
            context.main_scopes.join(", ")
        },
        summary_hint = context.summary_hint,
        staged_files = if payload.staged_files.is_empty() {
            "(none)".to_string()
        } else {
            payload.staged_files.join("\n")
        },
        staged_diff_preview = staged_diff_preview
    )
}

pub fn build_selected_commit_prompt(
    repo_path: &str,
    branch: &str,
    selected_files: &[String],
    language: &str,
) -> Result<GitCommitPromptPreview, String> {
    let profile = load_prompt_profile(repo_path);
    let action_map = load_file_actions(repo_path);
    let rules = prompt_processing_rules();
    let mut file_blocks = Vec::new();
    let mut debug_files = Vec::new();
    let mut traces = Vec::new();
    let mut raw_chars = 0;
    let mut files = Vec::new();

    for file_path in selected_files {
        let action = action_map
            .get(&normalize_match_path(file_path))
            .cloned()
            .unwrap_or_else(|| detect_action_from_diff_header(""));
        let classification = classify_file(file_path, &profile, &action);
        let raw_diff = runner::load_selected_file_diff(repo_path, file_path)?;
        let action = if action == "modified" {
            detect_action_from_diff_header(&raw_diff)
        } else {
            action
        };
        let file_raw_chars = raw_diff.chars().count();
        raw_chars += file_raw_chars;
        let clean_result = clean_diff_candidates(file_path, &raw_diff, &classification, &action);

        files.push(PreparedPromptFile {
            path: file_path.clone(),
            classification,
            action,
            raw_chars: file_raw_chars,
            candidates: clean_result.candidates,
            selected: Vec::new(),
            skipped: clean_result.skipped,
            reason: clean_result.reason,
        });
    }

    let budget_plan = build_budget_plan(&files);
    apply_group_budgets(&mut files, &budget_plan);
    preserve_small_source_changes(&mut files);
    let group_summary = build_group_summary(&files);

    let mut cleaned_chars = 0;
    let mut evidence_count = 0;

    for file in &files {
        let evidence = file
            .selected
            .iter()
            .filter(|line| line.reason != "file-level summary")
            .map(|line| line.text.clone())
            .collect::<Vec<_>>();
        let cleaned = evidence.join("\n");
        cleaned_chars += cleaned.chars().count();
        evidence_count += evidence.len();
        let reason = if file.reason.is_some() {
            file.reason.clone()
        } else if file.selected.is_empty() && !file.candidates.is_empty() {
            Some("omitted by group budget".to_string())
        } else {
            None
        };

        debug_files.push(PromptDebugFile {
            path: file.path.clone(),
            action: file.action.clone(),
            role: file.classification.role.clone(),
            scope: file.classification.scope.clone(),
            kind: file.classification.kind.clone(),
            strategy: file.classification.strategy.clone(),
            group_key: file.group_key(),
            raw_chars: file.raw_chars,
            cleaned_chars: cleaned.chars().count(),
            candidate_count: file.candidates.len(),
            evidence_count: evidence.len(),
            skipped: file.skipped,
            reason: reason.clone(),
            evidence: evidence.clone(),
            evidence_details: file.selected.clone(),
            omitted_candidate_count: file.candidates.len().saturating_sub(file.selected.len()),
        });

        traces.push(GitCommitPromptFileTrace {
            path: file.path.clone(),
            role: file.classification.role.clone(),
            scope: file.classification.scope.clone(),
            kind: file.classification.kind.clone(),
            strategy: file.classification.strategy.clone(),
            raw_chars: file.raw_chars,
            cleaned_chars: cleaned.chars().count(),
            evidence_count: evidence.len(),
            skipped: file.skipped,
            reason: reason.clone(),
        });

        if evidence.is_empty() || is_summary_only_file(file) {
            continue;
        }

        file_blocks.push(format!(
            "[{file_path} | {action} | {role}/{kind} | {scope}]\n{evidence}",
            file_path = file.path,
            action = file.action,
            role = file.classification.role.as_str(),
            scope = file.classification.scope.as_str(),
            kind = file.classification.kind.as_str(),
            evidence = cleaned
        ));
    }

    let group_overview = format_group_overview(&group_summary);
    let omitted_overview = format_omitted_files(&files);
    let schema_text = build_analysis_schema(language).to_string();
    let (lang_rule_4, lang_rule_5, lang_rule_6) = match language {
        "zh" => (
            "4. title 必须使用中文。",
            "5. body 必须使用中文要点，每行以 \"- \" 开头。",
            "6. summary 和 risks 必须使用中文。",
        ),
        _ => (
            "4. The title must be English only.",
            "5. The body must be English bullet lines, each prefixed with \"- \".",
            "6. summary and risks must be Simplified Chinese only.",
        ),
    };
    let prompt = format!(
        r#"
You are a precise git commit assistant.

Return ONLY valid JSON that matches this schema:
{schema_text}

Task:
Generate a Conventional Commit message for ONLY the selected files below.

Rules:
1. Infer the main intent from the selected files, not from the whole workspace.
2. Do not mechanically list file paths.
3. Prefer a concrete conventional commit title like feat(scope): ..., fix(scope): ..., refactor(scope): ...
{lang_rule_4}
{lang_rule_5}
{lang_rule_6}
7. Do not invent business details beyond the evidence.
8. If evidence is weak, stay moderately specific instead of becoming vague.

Repository:
{repo_path}

Branch:
{branch}

Change groups:
{group_overview}

Omitted or summarized files:
{omitted_overview}

Cleaned diff evidence:
{file_blocks}
"#,
        schema_text = schema_text,
        repo_path = repo_path,
        branch = branch,
        group_overview = group_overview,
        omitted_overview = omitted_overview,
        file_blocks = if file_blocks.is_empty() {
            "(none)".to_string()
        } else {
            file_blocks.join("\n\n")
        }
    );

    let trace = GitCommitPromptTrace {
        selected_files: traces,
        raw_chars,
        cleaned_chars,
        evidence_count,
        rules,
    };

    write_prompt_debug_file(
        repo_path,
        branch,
        selected_files,
        &trace,
        &debug_files,
        &group_summary,
        &budget_plan,
        &prompt,
    )?;

    Ok(GitCommitPromptPreview {
        prompt,
        trace,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptDebugFile {
    path: String,
    action: String,
    role: String,
    scope: String,
    kind: String,
    strategy: String,
    group_key: String,
    raw_chars: usize,
    cleaned_chars: usize,
    candidate_count: usize,
    evidence_count: usize,
    skipped: bool,
    reason: Option<String>,
    evidence: Vec<String>,
    evidence_details: Vec<EvidenceLine>,
    omitted_candidate_count: usize,
}

#[derive(Clone)]
struct PreparedPromptFile {
    path: String,
    classification: FileClassification,
    action: String,
    raw_chars: usize,
    candidates: Vec<EvidenceLine>,
    selected: Vec<EvidenceLine>,
    skipped: bool,
    reason: Option<String>,
}

impl PreparedPromptFile {
    fn group_key(&self) -> String {
        format!(
            "{}/{}/{}",
            self.classification.scope, self.classification.role, self.classification.kind
        )
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceLine {
    text: String,
    line_index: usize,
    score: i32,
    reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptGroupSummary {
    group_key: String,
    scope: String,
    role: String,
    kind: String,
    file_count: usize,
    raw_chars: usize,
    candidate_count: usize,
    evidence_count: usize,
    cleaned_chars: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptBudgetPlan {
    group_key: String,
    budget_chars: usize,
    weight: usize,
}

#[derive(Clone)]
struct PromptProfile {
    roles: Vec<(String, Vec<String>)>,
    scopes: Vec<(String, Vec<String>)>,
    attention_weights: BTreeMap<String, i32>,
    localization_patterns: Vec<String>,
}

#[derive(Clone)]
struct FileClassification {
    role: String,
    scope: String,
    kind: String,
    strategy: String,
    max_lines: usize,
    skip_verbose: bool,
}

struct CleanDiffResult {
    candidates: Vec<EvidenceLine>,
    skipped: bool,
    reason: Option<String>,
}

fn load_prompt_profile(repo_path: &str) -> PromptProfile {
    let mut profile = fallback_prompt_profile();
    let repo_root = runner::run_git(repo_path, &["rev-parse", "--show-toplevel"])
        .unwrap_or_else(|_| repo_path.to_string());
    let repo_root = Path::new(repo_root.trim());
    profile.localization_patterns = detect_localization_patterns(repo_root);
    let profile_path = repo_root.join(".lumina").join("git-profile.json");

    let Ok(content) = fs::read_to_string(profile_path) else { return profile; };
    let Ok(value) = serde_json::from_str::<Value>(&content) else { return profile; };

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

fn load_file_actions(repo_path: &str) -> BTreeMap<String, String> {
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

fn detect_action_from_diff_header(diff: &str) -> String {
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

fn fallback_prompt_profile() -> PromptProfile {
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
                vec!["*.json".to_string(), "*.toml".to_string(), "*.config.*".to_string()],
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

fn read_localization_patterns(value: &Value) -> Vec<String> {
    value
        .get("review")
        .and_then(|review| review.get("localizationPatterns"))
        .and_then(Value::as_array)
        .map(|patterns| patterns.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

fn detect_localization_patterns(repo_root: &Path) -> Vec<String> {
    let mut patterns = BTreeSet::new();
    let mut visited = 0usize;
    scan_localization_directories(repo_root, repo_root, 0, &mut visited, &mut patterns);
    patterns.into_iter().collect()
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
    let Ok(entries) = fs::read_dir(directory) else { return; };
    let mut child_directories = Vec::new();
    let mut locale_directories = Vec::new();
    let mut locale_codes = BTreeSet::new();

    for entry in entries.flatten() {
        *visited += 1;
        if *visited >= 6000 {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else { continue; };
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !matches!(name.as_str(), ".git" | ".lumina" | "node_modules" | "target" | "dist" | "build" | "vendor") {
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

    let has_parallel_resources = locale_directories.iter().enumerate().any(|(index, (_, names))| {
        locale_directories.iter().skip(index + 1).any(|(_, other_names)| {
            names.iter().any(|name| other_names.contains(name))
        })
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
    let Ok(entries) = fs::read_dir(directory) else { return BTreeSet::new(); };
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
    if !matches!(extension.as_str(), "json" | "json5" | "yaml" | "yml" | "ts" | "js" | "mjs" | "cjs" | "properties") {
        return None;
    }
    let stem = path.file_stem()?.to_string_lossy().to_lowercase();
    let candidate = stem.rsplit('.').next()?.replace('_', "-");
    normalize_locale_code(&candidate)
}

fn normalize_locale_code(candidate: &str) -> Option<String> {
    let language = candidate.split('-').next()?;
    const LANGUAGE_CODES: &[&str] = &[
        "ar", "bg", "ca", "cs", "da", "de", "el", "en", "es", "et", "fa", "fi", "fr", "he", "hi", "hr", "hu", "id", "it", "ja", "ko", "lt", "lv", "ms", "nb", "nl", "nn", "pl", "pt", "ro", "ru", "sk", "sl", "sr", "sv", "th", "tr", "uk", "vi", "zh",
        "ara", "deu", "eng", "fra", "ita", "jpn", "kor", "por", "rus", "spa", "zho",
    ];
    LANGUAGE_CODES.contains(&language).then_some(candidate.to_string())
}

fn read_attention_weights(value: &Value) -> BTreeMap<String, i32> {
    value
        .get("review")
        .and_then(|review| review.get("attentionWeights"))
        .and_then(Value::as_object)
        .map(|weights| {
            weights
                .iter()
                .filter_map(|(kind, weight)| weight.as_i64().map(|weight| (kind.clone(), weight as i32)))
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

fn classify_file(path: &str, profile: &PromptProfile, action: &str) -> FileClassification {
    let kind = if profile.localization_patterns.iter().any(|pattern| matches_pattern(path, pattern)) {
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

    let parts = pattern.split('*').filter(|part| !part.is_empty()).collect::<Vec<_>>();
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

fn normalize_match_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_lowercase()
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
        "png" | "jpg" | "jpeg" | "gif" | "ico" | "svg" | "webp" | "avif"
            | "woff" | "woff2" | "ttf" | "otf" | "mp3" | "mp4" | "webm"
    ) {
        "asset".to_string()
    } else if lower.ends_with(".gitignore") {
        "ignore".to_string()
    } else if ext == "ps1" {
        "script".to_string()
    } else if ext == "html" {
        "entry".to_string()
    } else if matches!(ext, "json" | "toml" | "yaml" | "yml" | "cjs" | "mjs") || lower.contains("config") {
        "config".to_string()
    } else if matches!(ext, "md" | "txt") {
        "docs".to_string()
    } else {
        "other".to_string()
    }
}

fn strategy_for(role: &str, kind: &str, action: &str) -> (String, usize, bool) {
    if role == "internal" || role == "generated" || matches!(kind, "internal" | "lockfile" | "asset") {
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
        (_, "style" | "i18n" | "docs" | "test") => ("reduced supporting evidence".to_string(), 14, false),
        _ => ("limited evidence".to_string(), 18, false),
    }
}

fn prompt_processing_rules() -> Vec<String> {
    vec![
        "Read only selected files and generate diff evidence locally before any model call.".to_string(),
        "Use .lumina/git-profile.json scopes and roles first; fall back to built-in rules when missing.".to_string(),
        "Evidence is selected by group budget, not by file order.".to_string(),
        "Comment, declaration, command, error, prompt, and user-facing text lines receive higher scores.".to_string(),
        "Generated, lock, and asset files are summarized instead of sending verbose content.".to_string(),
        "Style, i18n, docs, config, and tooling files keep reduced evidence as supporting context.".to_string(),
        "The final prompt is capped globally to avoid sending oversized workspace content.".to_string(),
    ]
}

fn build_budget_plan(files: &[PreparedPromptFile]) -> Vec<PromptBudgetPlan> {
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

fn apply_group_budgets(files: &mut [PreparedPromptFile], budget_plan: &[PromptBudgetPlan]) {
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
        let per_file_budget = (budget / files_in_group(files, &group_key).max(1)).clamp(120, 900);
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

fn files_in_group(files: &[PreparedPromptFile], group_key: &str) -> usize {
    files.iter().filter(|file| file.group_key() == group_key).count()
}

fn build_group_summary(files: &[PreparedPromptFile]) -> Vec<PromptGroupSummary> {
    let mut groups = BTreeMap::<String, PromptGroupSummary>::new();

    for file in files {
        let group_key = file.group_key();
        let entry = groups.entry(group_key.clone()).or_insert_with(|| PromptGroupSummary {
            group_key,
            scope: file.classification.scope.clone(),
            role: file.classification.role.clone(),
            kind: file.classification.kind.clone(),
            file_count: 0,
            raw_chars: 0,
            candidate_count: 0,
            evidence_count: 0,
            cleaned_chars: 0,
        });

        entry.file_count += 1;
        entry.raw_chars += file.raw_chars;
        entry.candidate_count += file.candidates.len();
        entry.evidence_count += file.selected.len();
        entry.cleaned_chars += file
            .selected
            .iter()
            .map(|line| line.text.chars().count() + 1)
            .sum::<usize>();
    }

    groups.into_values().collect()
}

fn format_group_overview(groups: &[PromptGroupSummary]) -> String {
    let lines = groups
        .iter()
        .filter(|group| group.evidence_count > 0)
        .map(|group| {
            format!(
                "- {group}: files={files}, evidenceLines={lines}, cleanedChars={chars}",
                group = group.group_key,
                files = group.file_count,
                lines = group.evidence_count,
                chars = group.cleaned_chars
            )
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        "(none)".to_string()
    } else {
        lines.join("\n")
    }
}

fn format_omitted_files(files: &[PreparedPromptFile]) -> String {
    let mut groups = BTreeMap::<String, usize>::new();
    for file in files
        .iter()
        .filter(|file| file.selected.is_empty() && file.classification.kind != "internal")
    {
        *groups.entry(file.group_key()).or_insert(0) += 1;
    }

    if groups.is_empty() {
        "(none)".to_string()
    } else {
        groups
            .into_iter()
            .map(|(group, count)| format!("- {group}: {count} files omitted or summarized"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn group_weight(classification: &FileClassification) -> usize {
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

fn min_evidence_score(classification: &FileClassification) -> i32 {
    if matches!(classification.kind.as_str(), "lockfile" | "asset" | "internal") {
        return i32::MAX;
    }

    match (classification.role.as_str(), classification.kind.as_str()) {
        ("primary", "source") => 70,
        (_, "source") => 40,
        (_, "config" | "script") => 36,
        (_, "docs") => 28,
        (_, "i18n" | "style" | "test") => 40,
        _ => MIN_EVIDENCE_SCORE,
    }
}

fn is_summary_only_file(file: &PreparedPromptFile) -> bool {
    !file.selected.is_empty()
        && file
            .selected
            .iter()
            .all(|line| line.reason == "file-level summary")
}

fn write_prompt_debug_file(
    repo_path: &str,
    branch: &str,
    selected_files: &[String],
    trace: &GitCommitPromptTrace,
    files: &[PromptDebugFile],
    group_summary: &[PromptGroupSummary],
    budget_plan: &[PromptBudgetPlan],
    prompt: &str,
) -> Result<(), String> {
    let repo_root = runner::run_git(repo_path, &["rev-parse", "--show-toplevel"])
        .unwrap_or_else(|_| repo_path.to_string());
    let debug_dir = Path::new(repo_root.trim()).join(".lumina");
    fs::create_dir_all(&debug_dir)
        .map_err(|e| format!("创建 Prompt 调试目录失败 {}: {}", debug_dir.display(), e))?;

    let debug_path = debug_dir.join("commit-prompt-debug.json");
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let content = json!({
        "version": 1,
        "generatedAtUnix": generated_at,
        "repoPath": repo_path,
        "branch": branch,
        "selectedFiles": selected_files,
        "summary": {
            "selectedFiles": trace.selected_files.len(),
            "rawChars": trace.raw_chars,
            "cleanedChars": trace.cleaned_chars,
            "evidenceCount": trace.evidence_count
        },
        "rules": trace.rules,
        "groupSummary": group_summary,
        "budgetPlan": budget_plan,
        "files": files,
        "promptLength": prompt.chars().count(),
        "prompt": prompt
    });
    let content = serde_json::to_string_pretty(&content)
        .map_err(|e| format!("序列化 Prompt 调试文件失败: {}", e))?;

    fs::write(&debug_path, content)
        .map_err(|e| format!("写入 Prompt 调试文件失败 {}: {}", debug_path.display(), e))
}

fn clean_diff_candidates(
    file_path: &str,
    diff: &str,
    classification: &FileClassification,
    action: &str,
) -> CleanDiffResult {
    if matches!(classification.kind.as_str(), "internal" | "lockfile" | "asset") {
        return CleanDiffResult {
            candidates: Vec::new(),
            skipped: true,
            reason: Some(format!("{} ignored for commit prompt evidence", classification.kind)),
        };
    }

    if classification.skip_verbose {
        return CleanDiffResult {
            candidates: vec![EvidenceLine {
                text: format!("+<{} changed; verbose content omitted>", file_path),
                line_index: 0,
                score: 1,
                reason: "summary-only file".to_string(),
            }],
            skipped: true,
            reason: Some(format!("{}; verbose content omitted", classification.strategy)),
        };
    }

    if action == "deleted" {
        return CleanDiffResult {
            candidates: vec![EvidenceLine {
                text: format!("-<{} deleted>", file_path),
                line_index: 0,
                score: 90,
                reason: "file-level deletion".to_string(),
            }],
            skipped: false,
            reason: Some("deleted file summarized at file level".to_string()),
        };
    }

    if action == "renamed" {
        return CleanDiffResult {
            candidates: vec![EvidenceLine {
                text: format!("~<{} renamed or moved>", file_path),
                line_index: 0,
                score: 80,
                reason: "file-level rename".to_string(),
            }],
            skipped: false,
            reason: Some("renamed file summarized at file level".to_string()),
        };
    }

    if matches!(action, "added" | "untracked") || is_large_diff(diff) {
        return summarize_file_from_diff(file_path, diff, classification, action);
    }

    let mut candidates = Vec::new();

    for (line_index, line) in diff.lines().enumerate() {
        if candidates.len() >= MAX_CANDIDATE_LINES_PER_FILE {
            break;
        }

        if is_diff_noise(line) {
            continue;
        }

        if line.starts_with('+') || line.starts_with('-') {
            let trimmed = line.trim();
            if trimmed.len() <= 1 {
                continue;
            }
            if is_low_value_line(trimmed) {
                continue;
            }
            let (score, reason) = score_evidence_line(trimmed, classification);
            if score <= 0 {
                continue;
            }
            candidates.push(EvidenceLine {
                text: redact_sensitive(&truncate_chars(trimmed, 220)),
                line_index,
                score,
                reason,
            });
        }
    }

    CleanDiffResult {
        skipped: false,
        reason: if candidates.len() >= MAX_CANDIDATE_LINES_PER_FILE {
            Some(format!("candidate lines capped at {}", MAX_CANDIDATE_LINES_PER_FILE))
        } else {
            None
        },
        candidates,
    }
}

fn score_evidence_line(line: &str, classification: &FileClassification) -> (i32, String) {
    let content = line.trim_start_matches(['+', '-']).trim();
    if is_sensitive_only_line(content) {
        return (0, "sensitive-only".to_string());
    }
    let lower = content.to_lowercase();
    let mut score = group_weight(classification) as i32;
    let mut reasons = Vec::new();

    if is_comment_line(content) {
        score += 24;
        reasons.push("comment");
    }
    if looks_like_declaration(content) {
        score += 20;
        reasons.push("declaration");
    }
    if looks_like_error_message(content) {
        score += 18;
        reasons.push("error-message");
    }
    if lower.contains("prompt") || lower.contains("rule") || lower.contains("schema") {
        score += 16;
        reasons.push("prompt-or-rule");
    }
    if lower.contains("command") || lower.contains("invoke") || lower.contains("tauri") {
        score += 12;
        reasons.push("command-or-integration");
    }
    if contains_cjk(content) {
        score += 10;
        reasons.push("user-facing-or-cn");
    }
    if matches!(classification.kind.as_str(), "style" | "i18n" | "test") {
        score -= 8;
        reasons.push("supporting-kind");
    }
    if is_boilerplate_line(content) {
        score -= 18;
        reasons.push("boilerplate");
    }
    if is_attribute_or_decorator(content) {
        score -= 32;
        reasons.push("attribute-or-decorator");
    }

    if reasons.is_empty() {
        reasons.push("changed-line");
    }

    (score, reasons.join(", "))
}

fn summarize_file_from_diff(
    file_path: &str,
    diff: &str,
    classification: &FileClassification,
    action: &str,
) -> CleanDiffResult {
    let mut candidates = vec![EvidenceLine {
        text: format!("{}<{} {} {}>", action_prefix(action), file_path, action, classification.kind),
        line_index: 0,
        score: 90,
        reason: "file-level summary".to_string(),
    }];

    for (line_index, line) in diff.lines().enumerate() {
        if candidates.len() >= classification.max_lines {
            break;
        }
        if is_diff_noise(line) || !line.starts_with('+') {
            continue;
        }

        let trimmed = line.trim();
        let content = trimmed.trim_start_matches('+').trim();
        if is_low_value_line(trimmed) || is_boilerplate_line(content) {
            continue;
        }

        let (score, reason) = score_structural_summary_line(content, classification);
        if score <= 0 {
            continue;
        }
        candidates.push(EvidenceLine {
            text: redact_sensitive(&truncate_chars(trimmed, 220)),
            line_index,
            score,
            reason,
        });
    }

    CleanDiffResult {
        candidates,
        skipped: false,
        reason: Some(if matches!(action, "added" | "untracked") {
            "new file summarized structurally".to_string()
        } else {
            "large diff summarized structurally".to_string()
        }),
    }
}

fn score_structural_summary_line(content: &str, classification: &FileClassification) -> (i32, String) {
    if is_sensitive_only_line(content) {
        return (0, "sensitive-only".to_string());
    }
    let lower = content.to_lowercase();

    if looks_like_declaration(content) {
        return (95, "structural declaration".to_string());
    }
    if is_comment_line(content) && meaningful_comment(content) {
        return (82, "meaningful comment".to_string());
    }
    if classification.kind == "docs" && content.starts_with('#') {
        return (76, "markdown heading".to_string());
    }
    if classification.kind == "style" && looks_like_selector_or_variable(content) {
        return (58, "style selector or variable".to_string());
    }
    if classification.kind == "config" && looks_like_config_key(content) {
        return (54, "config key".to_string());
    }
    if contains_cjk(content) && !is_boilerplate_line(content) {
        return (52, "user-facing text".to_string());
    }
    if lower.contains("prompt") || lower.contains("rule") || lower.contains("schema") {
        return (70, "prompt-or-rule".to_string());
    }
    if lower.contains("command") || lower.contains("invoke") || lower.contains("tauri") {
        return (64, "command-or-integration".to_string());
    }

    (0, "not structural".to_string())
}

fn action_prefix(action: &str) -> &'static str {
    match action {
        "deleted" => "-",
        "renamed" => "~",
        _ => "+",
    }
}

fn is_large_diff(diff: &str) -> bool {
    diff.lines().filter(|line| line.starts_with('+') || line.starts_with('-')).count() > 200
}

fn is_diff_noise(line: &str) -> bool {
    line.starts_with("diff --git ")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
        || line.starts_with("@@")
        || line.starts_with("\\ No newline")
}

fn is_comment_line(content: &str) -> bool {
    let trimmed = content.trim();
    !is_attribute_or_decorator(trimmed)
        && (trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('#')
        || trimmed.starts_with("<!--"))
}

fn looks_like_declaration(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.starts_with("fn ")
        || lower.starts_with("pub fn ")
        || lower.starts_with("struct ")
        || lower.starts_with("pub struct ")
        || lower.starts_with("enum ")
        || lower.starts_with("pub enum ")
        || lower.starts_with("interface ")
        || lower.starts_with("export interface ")
        || lower.starts_with("type ")
        || lower.starts_with("export type ")
        || lower.starts_with("class ")
        || lower.starts_with("export class ")
        || lower.starts_with("function ")
        || lower.starts_with("export function ")
        || lower.starts_with("const ")
        || lower.starts_with("export const ")
        || lower.contains("=>")
}

fn contains_cjk(content: &str) -> bool {
    content.chars().any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
}

fn looks_like_error_message(content: &str) -> bool {
    let lower = content.to_lowercase();
    let has_error_word = lower.contains("error")
        || lower.contains("warn")
        || lower.contains("failed")
        || lower.contains("失败")
        || lower.contains("错误");
    if !has_error_word {
        return false;
    }

    lower.contains("format!")
        || lower.contains("map_err")
        || lower.contains("throw")
        || lower.contains("message")
        || lower.contains("toast")
        || lower.contains("console.")
        || lower.contains("return err")
        || content.contains('"')
        || content.contains('\'')
        || contains_cjk(content)
}

fn is_attribute_or_decorator(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with("#[") || trimmed.starts_with("@")
}

fn meaningful_comment(content: &str) -> bool {
    let stripped = content
        .trim()
        .trim_start_matches("//")
        .trim_start_matches("/*")
        .trim_start_matches('*')
        .trim_start_matches('#')
        .trim();

    stripped.chars().count() >= 8
        && !stripped.eq_ignore_ascii_case("todo")
        && !stripped.eq_ignore_ascii_case("fixme")
}

fn looks_like_config_key(content: &str) -> bool {
    let trimmed = content.trim().trim_end_matches(',');
    trimmed.contains(':')
        && !trimmed.starts_with('{')
        && !trimmed.starts_with('}')
        && !trimmed.starts_with('[')
        && !trimmed.starts_with(']')
}

fn looks_like_selector_or_variable(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with('.')
        || trimmed.starts_with('#')
        || trimmed.starts_with("--")
        || trimmed.ends_with('{')
}

fn is_sensitive_only_line(content: &str) -> bool {
    let lower = content.to_lowercase();
    let sensitive = lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("api-key")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("authorization")
        || lower.contains("bearer ")
        || lower.contains("password")
        || lower.contains("sk-");

    sensitive
}

fn redact_sensitive(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let lower = part.to_lowercase();
            if lower.contains("sk-")
                || lower.contains("bearer")
                || lower.contains("authorization")
                || lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("secret")
                || lower.contains("token")
                || lower.contains("password")
            {
                "[REDACTED]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_boilerplate_line(content: &str) -> bool {
    let lower = content.trim().to_lowercase();
    lower.starts_with("import ")
        || lower.starts_with("export {")
        || lower.starts_with("@import ")
        || lower.starts_with("#[derive")
        || lower.starts_with("#[serde")
        || lower.starts_with("#[tauri::command")
        || lower.starts_with("\"")
        || lower.starts_with("'")
        || lower.contains("eslint")
        || lower.contains("prettier")
        || lower.contains("node_modules")
        || lower.contains("package-lock")
        || lower.contains("integrity")
        || lower.contains("resolved")
}

fn is_low_value_line(line: &str) -> bool {
    let content = line.trim_start_matches(['+', '-']).trim();
    let compact = content.replace(' ', "");
    content.is_empty()
        || content == "{"
        || content == "}"
        || content == "["
        || content == "]"
        || content == ","
        || matches!(compact.as_str(), "}," | "]," | "});" | "};" | ")," | ");")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            result.push_str("...");
            break;
        }
        result.push(ch);
    }
    result
}

fn preserve_small_source_changes(files: &mut [PreparedPromptFile]) {
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

#[cfg(test)]
mod tests {
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    use super::{
        apply_group_budgets, build_budget_plan, clean_diff_candidates, detect_localization_patterns,
        fallback_prompt_profile, preserve_small_source_changes, review_skip_category,
        score_review_attention, score_review_attention_details, FileClassification, PreparedPromptFile,
    };

    #[test]
    fn prioritizes_primary_source_with_substantial_evidence() {
        let classification = FileClassification {
            role: "primary".to_string(),
            scope: "frontend".to_string(),
            kind: "source".to_string(),
            strategy: "primary source evidence".to_string(),
            max_lines: 40,
            skip_verbose: false,
        };

        let profile = fallback_prompt_profile();
        let score = score_review_attention(
            &profile,
            &classification,
            "modified",
            12,
            900,
            80,
            &["logic".to_string()],
            false,
        );

        assert!(score >= 60);
        let (explained_score, breakdown) = score_review_attention_details(
            &profile,
            &classification,
            "modified",
            12,
            900,
            80,
            &["logic".to_string()],
            false,
        );
        assert_eq!(score, explained_score);
        assert_eq!(score, breakdown.iter().map(|item| item.delta).sum::<i32>());
    }

    #[test]
    fn deprioritizes_generated_or_summary_only_changes() {
        let classification = FileClassification {
            role: "generated".to_string(),
            scope: "root".to_string(),
            kind: "lockfile".to_string(),
            strategy: "summarize only".to_string(),
            max_lines: 1,
            skip_verbose: true,
        };

        let profile = fallback_prompt_profile();
        let score = score_review_attention(
            &profile,
            &classification,
            "modified",
            0,
            0,
            0,
            &["dependency".to_string()],
            true,
        );

        assert!(score < 50);
    }

    #[test]
    fn skips_assets_and_i18n_before_diff_scoring() {
        for kind in ["asset", "i18n"] {
            let classification = FileClassification {
                role: "secondary".to_string(),
                scope: "frontend".to_string(),
                kind: kind.to_string(),
                strategy: "summarize only".to_string(),
                max_lines: 1,
                skip_verbose: true,
            };

            assert!(review_skip_category(&classification).is_some());
        }
    }

    #[test]
    fn preserves_a_single_field_definition_as_commit_evidence() {
        let classification = FileClassification {
            role: "primary".to_string(),
            scope: "frontend".to_string(),
            kind: "source".to_string(),
            strategy: "primary source evidence".to_string(),
            max_lines: 40,
            skip_verbose: false,
        };
        let diff = "diff --git a/form.ts b/form.ts\n@@ -1,1 +1,2 @@\n const form = {\n+  securitySuiteLabel: [{ value: '', disabled: true }],";
        let cleaned = clean_diff_candidates("form.ts", diff, &classification, "modified");
        let mut files = vec![PreparedPromptFile {
            path: "form.ts".to_string(),
            classification,
            action: "modified".to_string(),
            raw_chars: diff.chars().count(),
            candidates: cleaned.candidates,
            selected: Vec::new(),
            skipped: false,
            reason: None,
        }];

        let budget = build_budget_plan(&files);
        apply_group_budgets(&mut files, &budget);
        assert!(files[0].selected.is_empty(), "the regression requires the normal threshold to omit this line");

        preserve_small_source_changes(&mut files);

        assert_eq!(files[0].selected.len(), 1);
        assert!(files[0].selected[0].text.contains("securitySuiteLabel"));
    }

    #[test]
    fn detects_localization_by_language_file_cluster_without_directory_keywords() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("lumina-locale-test-{unique}"));
        let messages = root.join("client").join("messages");
        fs::create_dir_all(&messages).unwrap();
        fs::write(messages.join("en-US.ts"), "export default {}").unwrap();
        fs::write(messages.join("zh-CN.ts"), "export default {}").unwrap();
        let english = root.join("shared").join("en");
        let chinese = root.join("shared").join("zh-CN");
        fs::create_dir_all(&english).unwrap();
        fs::create_dir_all(&chinese).unwrap();
        fs::write(english.join("common.json"), "{}").unwrap();
        fs::write(chinese.join("common.json"), "{}").unwrap();

        let patterns = detect_localization_patterns(&root);

        assert!(patterns.iter().any(|pattern| pattern == "client/messages/**"));
        assert!(patterns.iter().any(|pattern| pattern == "shared/en/**"));
        assert!(patterns.iter().any(|pattern| pattern == "shared/zh-CN/**"));
        fs::remove_dir_all(root).unwrap();
    }
}
