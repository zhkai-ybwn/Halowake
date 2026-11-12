use std::{
    collections::BTreeMap,
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
};
use crate::git::runner;

mod classifier;
mod diff_cleaner;
mod profile;

use classifier::{
    apply_group_budgets, build_budget_plan, classify_file, detect_action_from_diff_header,
    group_weight, load_file_actions, preserve_small_source_changes,
};
use diff_cleaner::clean_diff_candidates;
use profile::{
    load_prompt_profile, prompt_processing_rules, FileClassification, PromptProfile,
};

#[cfg(test)]
use profile::detect_localization_patterns;

const MAX_TOTAL_EVIDENCE_CHARS: usize = 12000;
const MAX_CANDIDATE_LINES_PER_FILE: usize = 80;
const MIN_EVIDENCE_SCORE: i32 = 40;

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

    let tracked_paths: Vec<_> = selected_files.iter().map(String::as_str)
        .filter(|path| action_map.get(*path).is_none_or(|action| action != "untracked"))
        .collect();
    let selected_diffs = runner::load_selected_file_diffs(repo_path, &tracked_paths)?;

    for file_path in selected_files {
        let known_action = action_map.get(file_path);
        let action = known_action
            .cloned()
            .unwrap_or_else(|| detect_action_from_diff_header(""));
        let classification = classify_file(file_path, &profile, &action);
        let fallback;
        let raw_diff = match selected_diffs.get(file_path.as_str()) {
            Some(diff) => diff.as_str(),
            None if action == "untracked" => {
                fallback = runner::load_untracked_file_diff(repo_path, file_path, "--unified=3")?;
                &fallback
            }
            None if known_action.is_none() => {
                fallback = runner::load_selected_file_diff(repo_path, file_path)?;
                &fallback
            }
            None => "",
        };
        let action = if action == "modified" {
            detect_action_from_diff_header(raw_diff)
        } else {
            action
        };
        let file_raw_chars = raw_diff.chars().count();
        raw_chars += file_raw_chars;
        let clean_result = clean_diff_candidates(file_path, raw_diff, &classification, &action);

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

struct CleanDiffResult {
    candidates: Vec<EvidenceLine>,
    skipped: bool,
    reason: Option<String>,
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

#[cfg(test)]
mod tests {
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    use super::{
        apply_group_budgets, build_budget_plan, clean_diff_candidates, detect_localization_patterns,
        preserve_small_source_changes, FileClassification, PreparedPromptFile,
    };

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
        let root = std::env::temp_dir().join(format!("halowake-locale-test-{unique}"));
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
