use std::io::Read;
use std::path::Path;
use base64::Engine;
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

    // ── Search ─────────────────────────────────────────────────────

    pub async fn search(
        &self,
        query: &str,
        n: u8,
    ) -> Result<crate::provider::SearchResponse, StepFunError> {
        let url = format!("{}/search", self.base_url);
        let body = serde_json::json!({
            "query": query,
            "n": n,
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

        let json: SearchApiResponse = response.json().await.map_err(StepFunError::Parse)?;
        let results: Vec<crate::provider::SearchResult> = json
            .results
            .into_iter()
            .map(|r| crate::provider::SearchResult {
                title: if r.title.is_empty() { r.url.clone() } else { r.title },
                url: r.url,
                snippet: if r.snippet.is_empty() { r.content } else { r.snippet },
            })
            .collect();

        Ok(crate::provider::SearchResponse { results })
    }

    // ── Vision ─────────────────────────────────────────────────────

    pub async fn vision(
        &self,
        image_path: &str,
        prompt: Option<&str>,
    ) -> Result<crate::provider::VisionResponse, StepFunError> {
        let image_url = if image_path.starts_with("http://") || image_path.starts_with("https://") {
            image_path.to_string()
        } else {
            Self::file_to_data_uri(image_path)?
        };

        let text = prompt.unwrap_or("Describe this image");
        let user_content = serde_json::json!([
            { "type": "text", "text": text },
            { "type": "image_url", "image_url": { "url": image_url } },
        ]);

        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": "step-1v-8k",
            "messages": [{
                "role": "user",
                "content": user_content,
            }],
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

        let completion: ChatCompletionResponse = response.json().await.map_err(StepFunError::Parse)?;
        let description = completion
            .choices
            .first()
            .and_then(|c| {
                if c.message.content.is_empty() { None } else { Some(c.message.content.clone()) }
            })
            .unwrap_or_else(|| "No description available".to_string());

        Ok(crate::provider::VisionResponse { description })
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

// ── Search API types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SearchApiResponse {
    query: String,
    n: u8,
    results: Vec<SearchResultRaw>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SearchResultRaw {
    url: String,
    position: u32,
    time: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    title: String,
}

// ── Helper functions ───────────────────────────────────────────────

impl StepFunClient {
    fn file_to_data_uri(path: &str) -> Result<String, StepFunError> {
        let path = Path::new(path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");

        let mime_type = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let mut file = std::fs::File::open(path)
            .map_err(|e| StepFunError::Header(format!("Failed to open image file: {}", e)))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| StepFunError::Header(format!("Failed to read image file: {}", e)))?;

        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(format!("data:{};base64,{}", mime_type, encoded))
    }
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

    // ── Search and Vision tests using mockito ───────────────────────

    #[tokio::test]
    async fn test_search_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/search")
            .match_header("authorization", "Bearer test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{
                "query": "rust programming",
                "n": 2,
                "results": [
                    {
                        "url": "https://example.com/rust",
                        "position": 1,
                        "time": "2024-01-01",
                        "snippet": "Rust is a systems programming language",
                        "content": "",
                        "title": "Rust Programming"
                    },
                    {
                        "url": "https://example.com/rust2",
                        "position": 2,
                        "time": "2024-01-02",
                        "snippet": "",
                        "content": "Rust memory safety features",
                        "title": ""
                    }
                ]
            }"#)
            .create();

        let client = StepFunClient::with_config("test-key", Some(&server.url()), None, None);
        let result = client.search("rust programming", 2).await.unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].title, "Rust Programming");
        assert_eq!(result.results[0].url, "https://example.com/rust");
        assert_eq!(result.results[0].snippet, "Rust is a systems programming language");
        // Second result: empty title -> use URL, empty snippet -> use content
        assert_eq!(result.results[1].title, "https://example.com/rust2");
        assert_eq!(result.results[1].snippet, "Rust memory safety features");

        mock.assert();
    }

    #[tokio::test]
    async fn test_search_api_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/search")
            .with_status(401)
            .with_body(r#"{"error": "Unauthorized"}"#)
            .create();

        let client = StepFunClient::with_config("bad-key", Some(&server.url()), None, None);
        let result = client.search("test", 1).await;
        assert!(result.is_err());
        mock.assert();
    }

    #[tokio::test]
    async fn test_vision_with_url() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "step-1v-8k",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "This is a beautiful sunset over the ocean."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 10,
                    "total_tokens": 110
                }
            }"#)
            .create();

        let client = StepFunClient::with_config("test-key", Some(&server.url()), None, None);
        let result = client
            .vision("https://example.com/image.jpg", Some("Describe this image"))
            .await
            .unwrap();

        assert_eq!(result.description, "This is a beautiful sunset over the ocean.");
        mock.assert();
    }

    #[tokio::test]
    async fn test_vision_with_file() {
        // Create a temporary test image file
        let temp_dir = std::env::temp_dir().join("stepfun_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let image_path = temp_dir.join("test.png");
        // Write a minimal valid PNG file (1x1 pixel, white)
        let minimal_png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&image_path, minimal_png).unwrap();

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{
                "id": "chatcmpl-456",
                "object": "chat.completion",
                "created": 1234567891,
                "model": "step-1v-8k",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "A small white square."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 50,
                    "completion_tokens": 5,
                    "total_tokens": 55
                }
            }"#)
            .create();

        let client = StepFunClient::with_config("test-key", Some(&server.url()), None, None);
        let result = client
            .vision(image_path.to_str().unwrap(), None)
            .await
            .unwrap();

        assert_eq!(result.description, "A small white square.");
        mock.assert();

        // Cleanup
        std::fs::remove_file(&image_path).ok();
        std::fs::remove_dir(&temp_dir).ok();
    }

    #[test]
    fn test_file_to_data_uri_png() {
        let temp_dir = std::env::temp_dir().join("stepfun_test_uri");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let image_path = temp_dir.join("test.png");
        let content = b"fake png data";
        std::fs::write(&image_path, content).unwrap();

        let data_uri = StepFunClient::file_to_data_uri(image_path.to_str().unwrap()).unwrap();
        assert!(data_uri.starts_with("data:image/png;base64,"));
        let encoded = &data_uri["data:image/png;base64,".len()..];
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(decoded, content);

        // Cleanup
        std::fs::remove_file(&image_path).ok();
        std::fs::remove_dir(&temp_dir).ok();
    }

    #[test]
    fn test_file_to_data_uri_jpg() {
        let temp_dir = std::env::temp_dir().join("stepfun_test_uri");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let image_path = temp_dir.join("test.jpg");
        let content = b"fake jpg data";
        std::fs::write(&image_path, content).unwrap();

        let data_uri = StepFunClient::file_to_data_uri(image_path.to_str().unwrap()).unwrap();
        assert!(data_uri.starts_with("data:image/jpeg;base64,"));

        // Cleanup
        std::fs::remove_file(&image_path).ok();
        std::fs::remove_dir(&temp_dir).ok();
    }

    #[test]
    fn test_file_to_data_uri_unknown_ext() {
        let temp_dir = std::env::temp_dir().join("stepfun_test_uri");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let image_path = temp_dir.join("test.xyz");
        let content = b"fake data";
        std::fs::write(&image_path, content).unwrap();

        let data_uri = StepFunClient::file_to_data_uri(image_path.to_str().unwrap()).unwrap();
        assert!(data_uri.starts_with("data:image/png;base64,"));

        // Cleanup
        std::fs::remove_file(&image_path).ok();
        std::fs::remove_dir(&temp_dir).ok();
    }

    #[test]
    fn test_file_to_data_uri_not_found() {
        let result = StepFunClient::file_to_data_uri("/nonexistent/path/image.png");
        assert!(result.is_err());
    }
}
