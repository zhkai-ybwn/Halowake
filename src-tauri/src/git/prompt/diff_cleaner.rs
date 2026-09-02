use super::{
    group_weight, CleanDiffResult, EvidenceLine, FileClassification, MAX_CANDIDATE_LINES_PER_FILE,
};

pub(super) fn clean_diff_candidates(
    file_path: &str,
    diff: &str,
    classification: &FileClassification,
    action: &str,
) -> CleanDiffResult {
    if matches!(
        classification.kind.as_str(),
        "internal" | "lockfile" | "asset"
    ) {
        return CleanDiffResult {
            candidates: Vec::new(),
            skipped: true,
            reason: Some(format!(
                "{} ignored for commit prompt evidence",
                classification.kind
            )),
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
            reason: Some(format!(
                "{}; verbose content omitted",
                classification.strategy
            )),
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
            if trimmed.len() <= 1 || is_low_value_line(trimmed) {
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
            Some(format!(
                "candidate lines capped at {}",
                MAX_CANDIDATE_LINES_PER_FILE
            ))
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
        text: format!(
            "{}<{} {} {}>",
            action_prefix(action),
            file_path,
            action,
            classification.kind
        ),
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

fn score_structural_summary_line(
    content: &str,
    classification: &FileClassification,
) -> (i32, String) {
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
    diff.lines()
        .filter(|line| line.starts_with('+') || line.starts_with('-'))
        .count()
        > 200
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
    content
        .chars()
        .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
}

fn looks_like_error_message(content: &str) -> bool {
    let lower = content.to_lowercase();
    let has_error_word = lower.contains("error")
        || lower.contains("warn")
        || lower.contains("failed")
        || lower.contains("失败")
        || lower.contains("错误");
    has_error_word
        && (lower.contains("format!")
            || lower.contains("map_err")
            || lower.contains("throw")
            || lower.contains("message")
            || lower.contains("toast")
            || lower.contains("console.")
            || lower.contains("return err")
            || content.contains('"')
            || content.contains('\'')
            || contains_cjk(content))
}

fn is_attribute_or_decorator(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with("#[") || trimmed.starts_with('@')
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
    lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("api-key")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("authorization")
        || lower.contains("bearer ")
        || lower.contains("password")
        || lower.contains("sk-")
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
