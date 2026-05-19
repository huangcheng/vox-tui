use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use super::{Message, ProviderError, ProviderResult};

/// Shared HTTP client for OpenAI-compatible chat/vision/search endpoints.
///
/// Both StepFun and MiniMax (v2) use the `/chat/completions` endpoint format,
/// so we extract the common logic here.
#[derive(Debug, Clone)]
pub struct OpenAIClient {
    pub client: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
}

impl OpenAIClient {
    pub fn new(base_url: &str, api_key: &str, client: Option<reqwest::Client>) -> Self {
        let base_url = if base_url.ends_with("/v1") {
            base_url.to_string()
        } else {
            format!("{}/v1", base_url.trim_end_matches('/'))
        };
        Self {
            client: client.unwrap_or_else(reqwest::Client::new),
            base_url,
            api_key: api_key.to_string(),
        }
    }

    pub fn headers(&self) -> ProviderResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        let auth_value = HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|e| ProviderError::Header(e.to_string()))?;
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    /// POST JSON, handle non-success, parse as serde_json::Value
    pub async fn post_json_raw(&self, url: &str, body: serde_json::Value) -> ProviderResult<serde_json::Value> {
        let resp = self.client
            .post(url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("(failed to read body: {e})"));
            return Err(ProviderError::Api { status: status.as_u16(), message: text });
        }

        resp.json::<serde_json::Value>().await.map_err(|e| ProviderError::Parse(e.to_string()))
    }

    /// OpenAI-compatible chat completion: POST `/chat/completions`
    pub async fn chat_completion(&self, model: &str, messages: &[Message]) -> ProviderResult<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let api_messages: Vec<ApiMessage> = messages.iter().map(|m| ApiMessage {
            role: &m.role,
            content: &m.content,
        }).collect();

        let body = ChatCompletionRequest {
            model,
            messages: &api_messages,
            stream: false,
            temperature: None,
            max_tokens: None,
        };

        let resp = self.client
            .post(&url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("(failed to read body: {e})"));
            return Err(ProviderError::Api { status: status.as_u16(), message: text });
        }

        resp.json::<ChatCompletionResponse>().await.map_err(|e| ProviderError::Parse(e.to_string()))
    }

    /// OpenAI-compatible vision completion: same endpoint with multimodal content
    pub async fn vision_completion(&self, model: &str, image_url: &str, prompt: &str) -> ProviderResult<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let user_content = serde_json::json!([
            { "type": "text", "text": prompt },
            { "type": "image_url", "image_url": { "url": image_url } },
        ]);

        let body = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": user_content }],
        });

        let data = self.post_json_raw(&url, body).await?;
        let response: ChatCompletionResponse = serde_json::from_value(data)
            .map_err(|e| ProviderError::Parse(e.to_string()))?;
        Ok(response)
    }
}

// ── OpenAI-compatible API types ─────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: &'a [ApiMessage<'a>],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ApiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    #[allow(dead_code)]
    pub id: Option<String>,
    #[allow(dead_code)]
    pub object: Option<String>,
    #[allow(dead_code)]
    pub created: Option<u64>,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<ApiUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: Option<u32>,
    pub message: ApiMessageResponse,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiMessageResponse {
    pub role: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
