use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "https://api.stepfun.com/v1";
pub const DEFAULT_MODEL: &str = "step-1-8k";
pub const DEFAULT_IMAGE_MODEL: &str = "step-image-edit-2";

#[derive(Debug, Clone)]
pub struct StepFunClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    image_model: String,
}

impl StepFunClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_config(api_key, None, None, None)
    }

    pub fn with_config(api_key: impl Into<String>, base_url: Option<&str>, model: Option<&str>, image_model: Option<&str>) -> Self {
        let base_url = base_url.unwrap_or(DEFAULT_BASE_URL);
        let base_url = if base_url.ends_with("/v1") {
            base_url.to_string()
        } else {
            format!("{}/v1", base_url.trim_end_matches('/'))
        };
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key: api_key.into(),
            model: model.unwrap_or(DEFAULT_MODEL).to_string(),
            image_model: image_model.unwrap_or(DEFAULT_IMAGE_MODEL).to_string(),
        }
    }

    fn headers(&self) -> Result<HeaderMap, StepFunError> {
        let mut headers = HeaderMap::new();
        let auth_value = HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|e| StepFunError::Header(e.to_string()))?;
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatCompletionResponse, StepFunError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatCompletionRequest {
            model: &self.model,
            messages,
            stream: false,
            temperature: None,
            max_tokens: None,
        };

        let response = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(StepFunError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(StepFunError::Api { status: status.as_u16(), message: text });
        }

        response.json::<ChatCompletionResponse>().await.map_err(StepFunError::Parse)
    }

    pub async fn image_generate(&self, prompt: &str) -> Result<crate::provider::ImageResponse, StepFunError> {
        let url = "https://api.stepfun.com/step_plan/v1/images/generations";
        let body = serde_json::json!({
            "model": self.image_model,
            "prompt": prompt,
            "response_format": "url",
            "cfg_scale": 1.0,
            "steps": 8,
            "text_mode": true
        });

        let response = self
            .client
            .post(url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(StepFunError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(StepFunError::Api { status: status.as_u16(), message: text });
        }

        let json: serde_json::Value = response.json().await.map_err(StepFunError::Parse)?;
        let data = json.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| StepFunError::Api {
                status: 0,
                message: "missing data array in response".into(),
            })?;

        let urls: Vec<String> = data
            .iter()
            .filter_map(|item| item.get("url").and_then(|u| u.as_str()).map(String::from))
            .collect();

        Ok(crate::provider::ImageResponse { urls })
    }

    pub async fn speech_synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
        format: &str,
    ) -> Result<crate::provider::SpeechResponse, StepFunError> {
        let url = format!("{}/audio/speech", self.base_url);
        let body = serde_json::json!({
            "model": "step-tts-2",
            "input": text,
            "voice": voice,
            "response_format": format,
            "speed": speed,
        });

        let response = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(StepFunError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(StepFunError::Api { status: status.as_u16(), message: text });
        }

        let audio_data = response.bytes().await.map_err(StepFunError::Http)?.to_vec();
        Ok(crate::provider::SpeechResponse {
            audio_data,
            format: format.to_string(),
        })
    }

    #[allow(dead_code)]
    pub async fn chat_stream(
        &self,
        messages: &[ChatMessage],
    ) -> Result<impl futures_util::Stream<Item = Result<StreamChunk, StepFunError>>, StepFunError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatCompletionRequest {
            model: &self.model,
            messages,
            stream: true,
            temperature: None,
            max_tokens: None,
        };

        let response = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(StepFunError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(StepFunError::Api { status: status.as_u16(), message: text });
        }

        let stream = response
            .bytes_stream()
            .filter_map(|chunk| async {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        match parse_sse_chunk(&text) {
                            Ok(Some(s)) => Some(Ok(s)),
                            Ok(None) => None,
                            Err(e) => Some(Err(e)),
                        }
                    }
                    Err(e) => Some(Err(StepFunError::Http(e))),
                }
            });

        Ok(stream)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum StreamChunk {
    Content(String),
    Done,
}

#[allow(dead_code)]
fn parse_sse_chunk(text: &str) -> Result<Option<StreamChunk>, StepFunError> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                return Ok(Some(StreamChunk::Done));
            }
            if let Ok(parsed) = serde_json::from_str::<SSEData>(data)
                && let Some(choice) = parsed.choices.first()
                && let Some(content) = &choice.delta.content
            {
                return Ok(Some(StreamChunk::Content(content.clone())));
            }
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SSEData {
    choices: Vec<SSEChoice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SSEChoice {
    delta: SSEDelta,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SSEDelta {
    content: Option<String>,
}

#[derive(Debug)]
pub enum StepFunError {
    Http(reqwest::Error),
    Parse(reqwest::Error),
    Header(String),
    Api { status: u16, message: String },
}

impl std::fmt::Display for StepFunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepFunError::Http(e) => write!(f, "HTTP error: {}", e),
            StepFunError::Parse(e) => write!(f, "Parse error: {}", e),
            StepFunError::Header(msg) => write!(f, "Header error: {}", msg),
            StepFunError::Api { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
        }
    }
}

impl std::error::Error for StepFunError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_content() {
        let chunk = r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#;
        let result = parse_sse_chunk(chunk).unwrap();
        assert!(matches!(result, Some(StreamChunk::Content(s)) if s == "Hello"));
    }

    #[test]
    fn test_parse_sse_done() {
        let chunk = "data: [DONE]";
        let result = parse_sse_chunk(chunk).unwrap();
        assert!(matches!(result, Some(StreamChunk::Done)));
    }

    #[test]
    fn test_parse_sse_empty() {
        let chunk = "";
        let result = parse_sse_chunk(chunk).unwrap();
        assert!(result.is_none());
    }
}
