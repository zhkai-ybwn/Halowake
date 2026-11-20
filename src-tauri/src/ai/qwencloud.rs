use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::json;

use crate::git::models::{AiModelConfig, GitAiAnalysis};

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

fn clean_json(text: &str) -> String {
    text.replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string()
}

pub(crate) fn is_official_openai_endpoint(base_url: &str) -> bool {
    Url::parse(base_url).is_ok_and(|url| url.host_str() == Some("api.openai.com"))
}

pub async fn call_openai_compatible_generate(
    config: &AiModelConfig,
    prompt: &str,
) -> Result<GitAiAnalysis, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let base_url = config.base_url.trim().trim_end_matches('/');
    let mut body = json!({
        "model": config.model.trim(),
        "messages": [
            {
                "role": "system",
                "content": "You are a precise git commit assistant. Always return valid JSON."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "response_format": { "type": "json_object" }
    });
    if !is_official_openai_endpoint(base_url) {
        body["temperature"] = json!(0.2);
        body["extra_body"] = json!({ "enable_thinking": false });
    }

    let mut request = client
        .post(format!("{base_url}/chat/completions"))
        .header("Content-Type", "application/json")
        .json(&body);

    if let Some(api_key) = config.api_key.as_ref().map(|key| key.trim()).filter(|key| !key.is_empty()) {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = request
        .send()
        .await
        .map_err(|e| format!("OpenAI compatible request failed: {}", e))?;

    let status = resp.status();

    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI compatible API error {}: {}", status, text));
    }

    let value: ChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse response failed: {}", e))?;

    let raw = value
        .choices
        .get(0)
        .ok_or("Missing choices")?
        .message
        .content
        .clone();

    let cleaned = clean_json(&raw);

    let parsed: GitAiAnalysis = serde_json::from_str(&cleaned)
        .map_err(|e| format!("JSON parse failed: {}\nraw: {}", e, cleaned))?;

    Ok(parsed)
}
