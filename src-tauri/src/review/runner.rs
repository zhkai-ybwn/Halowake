use std::time::Instant;

use reqwest::{header, Client};
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::{git::models::{AiModelConfig, AiProviderType}, storage::AppDatabase};

use super::{
    models::{AiCallUsage, AiReviewBatchResult, ReviewOverview, ReviewProgressEvent, StartReviewPayload},
    planner::ReviewPlan,
    prompt::build_review_prompt,
    repository,
    validator::validate_batch_result,
};

pub async fn execute_review(
    app: AppHandle,
    database: AppDatabase,
    session_id: String,
    payload: StartReviewPayload,
    mut plan: ReviewPlan,
) {
    let total = plan.batches.len();
    let mut revision = 0u64;
    let mut limitations = std::mem::take(&mut plan.limitations);
    let mut all_findings = std::mem::take(&mut plan.deterministic_findings);
    let mut input_tokens = 0usize;
    let mut output_tokens = 0usize;
    let mut usage_estimated = false;
    let mut had_error = false;

    let _ = repository::replace_files(&database, &session_id, &plan.files);
    let _ = repository::append_findings(&database, &session_id, &all_findings);
    let first_file = plan.batches.first().and_then(|batch| batch.files.first()).map(|file| file.path.as_str());
    emit_progress(&app, &database, &session_id, &mut revision, "running", "ai-review", 0, total, first_file);

    for (index, batch) in plan.batches.iter().enumerate() {
        let prompt = build_review_prompt(batch, payload.budget_mode, payload.language.as_deref().unwrap_or("zh-CN"), payload.budget_mode.max_findings());
        let started = Instant::now();
        let call = call_review_model(&payload.model, &prompt).await;
        let duration_ms = started.elapsed().as_millis() as u64;
        match call {
            Ok((result, used_input, used_output, estimated)) => {
                usage_estimated |= estimated;
                let (findings, batch_limitations) = validate_batch_result(batch, result);
                input_tokens += used_input;
                output_tokens += used_output;
                limitations.extend(batch_limitations);
                let _ = repository::append_findings(&database, &session_id, &findings);
                all_findings.extend(findings);
                let usage = AiCallUsage {
                    batch_id: batch.id.clone(), files: batch.files.iter().map(|file| file.path.clone()).collect(),
                    input_tokens: used_input, output_tokens: used_output, estimated, duration_ms,
                    status: "completed".to_string(), error: None,
                };
                let _ = repository::append_ai_call(&database, &session_id, &payload.model.id, &usage);
            }
            Err(error) => {
                had_error = true;
                usage_estimated = true;
                limitations.push(format!("{} 审查失败: {}", batch.id, error));
                let usage = AiCallUsage {
                    batch_id: batch.id.clone(), files: batch.files.iter().map(|file| file.path.clone()).collect(),
                    input_tokens: estimate_tokens(&prompt), output_tokens: 0, estimated: true, duration_ms,
                    status: "failed".to_string(), error: Some(error),
                };
                input_tokens += usage.input_tokens;
                let _ = repository::append_ai_call(&database, &session_id, &payload.model.id, &usage);
            }
        }
        emit_progress(&app, &database, &session_id, &mut revision, "running", "ai-review", index + 1, total, batch.files.first().map(|file| file.path.as_str()));
    }

    let overview = build_overview(&all_findings, plan.rules.len());
    let status = if had_error && all_findings.is_empty() { "failed" } else if had_error { "partial" } else { "completed" };
    let _ = repository::finish_session(
        &database, &session_id, status, &overview, &limitations, input_tokens, output_tokens,
        usage_estimated, (status == "failed").then_some("All AI review batches failed"),
    );
    emit_progress(&app, &database, &session_id, &mut revision, status, "complete", total, total, None);
}

fn emit_progress(
    app: &AppHandle,
    database: &AppDatabase,
    session_id: &str,
    revision: &mut u64,
    status: &str,
    phase: &str,
    completed: usize,
    total: usize,
    current_file: Option<&str>,
) {
    *revision += 1;
    let _ = repository::update_progress(database, session_id, status, phase, completed, total, current_file);
    let _ = app.emit("local-code-review-updated", ReviewProgressEvent {
        session_id: session_id.to_string(), revision: *revision, status: status.to_string(), phase: phase.to_string(),
        completed, total, current_file: current_file.map(str::to_string),
    });
}

fn build_overview(findings: &[super::models::ReviewFinding], applied_rules: usize) -> ReviewOverview {
    let mut overview = ReviewOverview { applied_rules, ..ReviewOverview::default() };
    let mut triggered = std::collections::HashSet::new();
    for finding in findings {
        match finding.severity.as_str() {
            "critical" => overview.critical += 1,
            "major" => overview.major += 1,
            "minor" => overview.minor += 1,
            _ => overview.suggestion += 1,
        }
        if let Some(rule_id) = &finding.rule_id { triggered.insert(rule_id); }
    }
    overview.triggered_rules = triggered.len();
    overview
}

async fn call_review_model(model: &AiModelConfig, prompt: &str) -> Result<(AiReviewBatchResult, usize, usize, bool), String> {
    if !model.enabled { return Err("当前 Review 模型已禁用".to_string()); }
    let client = Client::builder().timeout(std::time::Duration::from_secs(120)).build()
        .map_err(|error| format!("创建 AI HTTP 客户端失败: {error}"))?;
    match &model.provider {
        AiProviderType::OpenaiCompatible => call_openai(&client, model, prompt).await,
        AiProviderType::Ollama => call_ollama(&client, model, prompt).await,
    }
}

#[derive(Deserialize)]
struct OpenAiUsage { prompt_tokens: Option<usize>, completion_tokens: Option<usize> }

async fn call_openai(client: &Client, model: &AiModelConfig, prompt: &str) -> Result<(AiReviewBatchResult, usize, usize, bool), String> {
    let mut body = json!({
        "model": model.model,
        "messages": [
            {"role":"system","content":"You are Halowake local code reviewer. Return valid JSON only."},
            {"role":"user","content":prompt}
        ],
        "temperature": 0.1,
        "max_tokens": 4096,
        "response_format": {"type":"json_object"}
    });
    let deepseek_v4 = is_deepseek_v4(model);
    if deepseek_v4 {
        body["thinking"] = json!({"type":"disabled"});
    } else {
        body["enable_thinking"] = json!(false);
        body["chat_template_kwargs"] = json!({"enable_thinking": false});
    }
    let endpoint = format!("{}/chat/completions", model.base_url.trim().trim_end_matches('/'));
    let mut last_error = String::new();
    let mut total_input = 0usize;
    let mut total_output = 0usize;
    let mut usage_estimated = false;
    let mut correction_applied = false;
    let mut chat_template_compatible = !deepseek_v4;
    let mut thinking_compatible = deepseek_v4;
    for attempt in 0..4 {
        let mut request = client
            .post(&endpoint)
            .header(header::ACCEPT, "application/json")
            .header(header::ACCEPT_ENCODING, "identity")
            .header(header::CONNECTION, "close")
            .json(&body);
        if let Some(key) = model.api_key.as_deref().map(str::trim).filter(|key| !key.is_empty()) {
            request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = format!("AI Review 请求失败: {error}");
                if attempt < 3 { continue; }
                break;
            }
        };
        let status = response.status();
        match response.bytes().await {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                if !status.is_success() {
                    if status.as_u16() == 400 && attempt < 3 {
                        let mut removed_incompatible_option = false;
                        if text.contains("enable_thinking") && body.get("enable_thinking").is_some() {
                            if let Some(object) = body.as_object_mut() { object.remove("enable_thinking"); }
                            removed_incompatible_option = true;
                        }
                        if text.contains("chat_template_kwargs") && body.get("chat_template_kwargs").is_some() {
                            if let Some(object) = body.as_object_mut() { object.remove("chat_template_kwargs"); }
                            chat_template_compatible = false;
                            removed_incompatible_option = true;
                        }
                        if text.contains("thinking") && body.get("thinking").is_some() {
                            if let Some(object) = body.as_object_mut() { object.remove("thinking"); }
                            thinking_compatible = false;
                            removed_incompatible_option = true;
                        }
                        if removed_incompatible_option { continue; }
                    }
                    return Err(format!("AI Review API 返回 {status}: {}", response_excerpt(&text)));
                }
                let parsed = serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|error| {
                    format!("AI Review 响应不是合法 JSON: {error}; body={}", response_excerpt(&text))
                })?;
                let usage = serde_json::from_value::<OpenAiUsage>(parsed.get("usage").cloned().unwrap_or_default()).ok();
                let content = extract_openai_content(&parsed);
                let reasoning = extract_openai_reasoning(&parsed);
                let used_input = usage.as_ref().and_then(|usage| usage.prompt_tokens).unwrap_or_else(|| estimate_tokens(prompt));
                let used_output = usage.as_ref().and_then(|usage| usage.completion_tokens).unwrap_or_else(|| estimate_tokens(&content));
                total_input += used_input;
                total_output += used_output;
                usage_estimated |= usage.as_ref().and_then(|usage| usage.prompt_tokens).is_none();
                match parse_result(&content) {
                    Ok(result) => return Ok((result, total_input, total_output, usage_estimated)),
                    Err(error) => {
                        let finish_reason = parsed.pointer("/choices/0/finish_reason").and_then(|value| value.as_str()).unwrap_or("unknown");
                        let preview = if content.trim().is_empty() { format!("reasoning={}", response_excerpt(&reasoning)) } else { format!("content={}", response_excerpt(&content)) };
                        last_error = format!("{error}; finish_reason={finish_reason}; {preview}");
                        if !correction_applied && attempt < 3 {
                            correction_applied = true;
                            body["messages"][1]["content"] = json!(format!(
                                "{prompt}\n\nRETRY REQUIREMENT: Return the complete JSON object immediately. Do not reason aloud, use markdown, or leave content empty."
                            ));
                            body["max_tokens"] = json!(4096);
                            if thinking_compatible {
                                body["thinking"] = json!({"type":"disabled"});
                            }
                            if chat_template_compatible {
                                body["chat_template_kwargs"] = json!({"enable_thinking": false});
                            }
                            continue;
                        }
                        break;
                    }
                }
            }
            Err(error) => {
                last_error = format!("读取 AI Review 响应体失败: {error}");
                if attempt < 3 { continue; }
            }
        }
    }
    Err(if last_error.is_empty() { "AI Review 未返回可解析结果".to_string() } else { last_error })
}

fn is_deepseek_v4(model: &AiModelConfig) -> bool {
    let model_name = model.model.trim().to_ascii_lowercase();
    let base_url = model.base_url.trim().to_ascii_lowercase();
    model_name.starts_with("deepseek-v4") || base_url.contains("api.deepseek.com")
}

async fn call_ollama(client: &Client, model: &AiModelConfig, prompt: &str) -> Result<(AiReviewBatchResult, usize, usize, bool), String> {
    let response = client.post(format!("{}/api/generate", model.base_url.trim().trim_end_matches('/'))).json(&json!({
        "model": model.model, "prompt": prompt, "stream": false, "format": "json", "think": false,
        "options": {"temperature":0.1,"num_predict":1800}
    })).send().await.map_err(|error| format!("Ollama Review 请求失败: {error}"))?;
    let status = response.status();
    let value: serde_json::Value = response.json().await.map_err(|error| format!("解析 Ollama Review 响应失败: {error}"))?;
    if !status.is_success() { return Err(format!("Ollama Review 返回 {status}: {}", value)); }
    let content = value.get("response").and_then(|value| value.as_str()).ok_or_else(|| "Ollama Review 响应缺少 response".to_string())?;
    let result = parse_result(content)?;
    let input = value.get("prompt_eval_count").and_then(|value| value.as_u64()).map(|value| value as usize).unwrap_or_else(|| estimate_tokens(prompt));
    let output = value.get("eval_count").and_then(|value| value.as_u64()).map(|value| value as usize).unwrap_or_else(|| estimate_tokens(content));
    let estimated = value.get("prompt_eval_count").is_none();
    Ok((result, input, output, estimated))
}

fn parse_result(value: &str) -> Result<AiReviewBatchResult, String> {
    let cleaned = value.replace("```json", "").replace("```", "");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return Err("AI Review 返回了空内容".to_string());
    }
    if let Ok(result) = serde_json::from_str(cleaned) {
        return Ok(result);
    }
    if let (Some(start), Some(end)) = (cleaned.find('{'), cleaned.rfind('}')) {
        if end > start {
            return serde_json::from_str(&cleaned[start..=end])
                .map_err(|error| format!("AI Review JSON 不符合结构: {error}"));
        }
    }
    Err("AI Review JSON 被截断或未包含完整对象".to_string())
}

fn extract_openai_content(value: &serde_json::Value) -> String {
    let message = value.pointer("/choices/0/message");
    if let Some(content) = message.and_then(|message| message.get("content")) {
        if let Some(text) = content.as_str().filter(|text| !text.trim().is_empty()) {
            return text.to_string();
        }
        if let Some(parts) = content.as_array() {
            let text = parts.iter().filter_map(|part| {
                part.get("text").and_then(|text| text.as_str())
                    .or_else(|| part.pointer("/text/value").and_then(|text| text.as_str()))
            }).collect::<Vec<_>>().join("");
            if !text.trim().is_empty() { return text; }
        }
    }
    String::new()
}

fn extract_openai_reasoning(value: &serde_json::Value) -> String {
    value.pointer("/choices/0/message")
        .and_then(|message| message.get("reasoning_content"))
        .and_then(|content| content.as_str())
        .unwrap_or_default()
        .to_string()
}

fn response_excerpt(value: &str) -> String {
    value.trim().chars().take(500).collect()
}

fn estimate_tokens(value: &str) -> usize { (value.chars().count() + 2) / 3 }

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::git::models::{AiModelConfig, AiProviderType};

    use super::{extract_openai_content, extract_openai_reasoning, is_deepseek_v4, parse_result};

    #[test]
    fn reports_empty_review_content_explicitly() {
        assert!(parse_result("  ").unwrap_err().contains("空内容"));
    }

    #[test]
    fn extracts_json_from_wrapped_content() {
        let result = parse_result("result:\n```json\n{\"schemaVersion\":1,\"batchId\":\"batch-1\",\"reviewedFiles\":[],\"findings\":[],\"limitations\":[]}\n```").unwrap();
        assert_eq!(result.batch_id, "batch-1");
    }

    #[test]
    fn reads_array_and_reasoning_content_variants() {
        let array_value = json!({"choices":[{"message":{"content":[{"type":"text","text":"{\"ok\":true}"}]}}]});
        assert_eq!(extract_openai_content(&array_value), "{\"ok\":true}");
        let reasoning_value = json!({"choices":[{"message":{"content":"","reasoning_content":"{\"ok\":true}"}}]});
        assert!(extract_openai_content(&reasoning_value).is_empty());
        assert_eq!(extract_openai_reasoning(&reasoning_value), "{\"ok\":true}");
    }

    #[test]
    fn detects_deepseek_v4_by_model_or_official_endpoint() {
        let mut model = AiModelConfig {
            id: "review".to_string(),
            name: "Review".to_string(),
            provider: AiProviderType::OpenaiCompatible,
            base_url: "https://example.com".to_string(),
            api_key: None,
            model: "deepseek-v4-pro".to_string(),
            enabled: true,
        };
        assert!(is_deepseek_v4(&model));
        model.model = "proxy-model".to_string();
        model.base_url = " https://api.deepseek.com ".to_string();
        assert!(is_deepseek_v4(&model));
    }
}
