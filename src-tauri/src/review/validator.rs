use std::{collections::{HashMap, HashSet}, hash::{Hash, Hasher}};

use super::{models::{AiReviewBatchResult, ReviewFinding}, planner::ReviewBatch};

pub fn validate_batch_result(batch: &ReviewBatch, result: AiReviewBatchResult) -> (Vec<ReviewFinding>, Vec<String>) {
    let files = batch.files.iter().map(|file| (file.path.clone(), file)).collect::<HashMap<_,_>>();
    let mut findings = Vec::new();
    let mut limitations = result.limitations;
    let mut fingerprints = HashSet::new();
    if result.schema_version != 1 { limitations.push(format!("{} returned unsupported schema version", batch.id)); }
    for candidate in result.findings {
        let Some(file) = files.get(&candidate.file_path) else { limitations.push(format!("discarded finding for unselected file {}", candidate.file_path)); continue; };
        let Some((start_line, end_line)) = resolve_existing_code(&candidate.existing_code, &file.source_content, &file.changed_lines) else {
            limitations.push(format!("discarded finding without a unique changed-source anchor in {}", candidate.file_path));
            continue;
        };
        if candidate.problem.trim().is_empty() || candidate.evidence.trim().is_empty() || candidate.trigger_scenario.trim().is_empty() { continue; }
        let severity = normalize_severity(&candidate.severity, candidate.confidence, &candidate.impact);
        let fingerprint_source = format!("{}|{}|{}|{}", candidate.rule_id.clone().unwrap_or_default(), candidate.file_path, start_line, candidate.title.to_lowercase());
        let fingerprint = stable_hash(&fingerprint_source);
        if !fingerprints.insert(fingerprint.clone()) { continue; }
        findings.push(ReviewFinding {
            id: format!("ai-{fingerprint}"), fingerprint, source: "ai".to_string(), rule_id: candidate.rule_id,
            category: candidate.category, severity, confidence: candidate.confidence.clamp(0.0, 1.0),
            file_path: candidate.file_path, start_line, end_line,
            title: candidate.title, problem: candidate.problem, impact: candidate.impact,
            trigger_scenario: candidate.trigger_scenario, evidence: candidate.existing_code,
            suggestion: candidate.suggestion, verified: false, status: "open".to_string(), user_note: None,
        });
    }
    (findings, limitations)
}

fn resolve_existing_code(existing_code: &str, source: &str, changed_lines: &[usize]) -> Option<(usize, usize)> {
    let target = normalized_nonempty_lines(existing_code);
    if target.is_empty() || source.is_empty() { return None; }
    let source_lines = source.lines().enumerate().filter_map(|(index, line)| {
        let normalized = normalize_code_line(line);
        (!normalized.is_empty()).then_some((index + 1, normalized))
    }).collect::<Vec<_>>();
    if source_lines.len() < target.len() { return None; }
    let changed = changed_lines.iter().copied().collect::<HashSet<_>>();
    let mut matches = Vec::new();
    for index in 0..=source_lines.len() - target.len() {
        if target.iter().enumerate().all(|(offset, target_line)| source_lines[index + offset].1 == *target_line) {
            let start = source_lines[index].0;
            let end = source_lines[index + target.len() - 1].0;
            if (start..=end).any(|line| changed.contains(&line)) { matches.push((start, end)); }
        }
    }
    (matches.len() == 1).then(|| matches[0])
}

fn normalized_nonempty_lines(value: &str) -> Vec<String> {
    value.lines().map(normalize_code_line).filter(|line| !line.is_empty()).collect()
}

fn normalize_code_line(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_severity(value: &str, confidence: f64, impact: &str) -> String {
    let value = match value { "critical" | "major" | "minor" | "suggestion" => value, _ => "suggestion" };
    if matches!(value, "critical" | "major") && (confidence < 0.65 || impact.trim().chars().count() < 12) { "minor".to_string() } else { value.to_string() }
}

fn stable_hash(value: &str) -> String { let mut hasher = std::collections::hash_map::DefaultHasher::new(); value.hash(&mut hasher); format!("{:016x}", hasher.finish()) }

#[cfg(test)]
mod tests {
    use super::resolve_existing_code;

    #[test]
    fn relocates_exact_code_to_the_changed_source_line() {
        let source = "class Example {\n  loading = true;\n  reset() {\n    this.loading = false;\n  }\n}";
        assert_eq!(resolve_existing_code("this.loading = false;", source, &[4]), Some((4, 4)));
    }

    #[test]
    fn rejects_an_anchor_that_is_not_on_changed_code() {
        let source = "const stable = true;\nconst changed = true;";
        assert_eq!(resolve_existing_code("const stable = true;", source, &[2]), None);
    }

    #[test]
    fn rejects_ambiguous_changed_anchors() {
        let source = "reset();\nwork();\nreset();";
        assert_eq!(resolve_existing_code("reset();", source, &[1, 3]), None);
    }
}
