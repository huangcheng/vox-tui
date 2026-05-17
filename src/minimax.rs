use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "https://api.minimax.chat/v1";
pub const DEFAULT_MODEL: &str = "abab6.5s-chat";

#[derive(Debug, Clone)]
pub struct MiniMaxClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    group_id: Option<String>,
    model: String,
}

impl MiniMaxClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_config(api_key, None, None, None)
    }

    pub fn with_config(
        api_key: impl Into<String>,
        group_id: Option<&str>,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or(DEFAULT_BASE_URL).to_string(),
            api_key: api_key.into(),
            group_id: group_id.map(String::from),
            model: model.unwrap_or(DEFAULT_MODEL).to_string(),
        }
    }

    fn headers(&self) -> Result<HeaderMap, MiniMaxError> {
        let mut headers = HeaderMap::new();
        let auth_value = HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|e| MiniMaxError::Header(e.to_string()))?;
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    pub async fn chat(
        &self,
        messages: &[crate::provider::Message],
    ) -> Result<ChatResponse, MiniMaxError> {
        let url = format!("{}/text/chat", self.base_url);

        let group_id_str = self.group_id.as_deref().unwrap_or("");

        let body = ChatRequest {
            group_id: group_id_str,
            model: &self.model,
            messages: messages.iter().map(|m| MiniMaxMessage {
                sender_type: if m.role == "user" { "USER" } else { "BOT" }.to_string(),
                text: &m.content,
            }).collect(),
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
            .map_err(MiniMaxError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(MiniMaxError::Api { status: status.as_u16(), message: text });
        }

        response.json::<ChatResponse>().await.map_err(MiniMaxError::Parse)
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    group_id: &'a str,
    model: &'a str,
    messages: Vec<MiniMaxMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct MiniMaxMessage<'a> {
    sender_type: String,
    text: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub reply: String,
    pub model: String,
    pub usage: MiniMaxUsage,
}

#[derive(Debug, Deserialize)]
pub struct MiniMaxUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug)]
pub enum MiniMaxError {
    Http(reqwest::Error),
    Parse(reqwest::Error),
    Header(String),
    Api { status: u16, message: String },
}

impl std::fmt::Display for MiniMaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiniMaxError::Http(e) => write!(f, "HTTP error: {}", e),
            MiniMaxError::Parse(e) => write!(f, "Parse error: {}", e),
            MiniMaxError::Header(msg) => write!(f, "Header error: {}", msg),
            MiniMaxError::Api { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
        }
    }
}

impl std::error::Error for MiniMaxError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_minimax_client_new() {
        let client = MiniMaxClient::new("test-key");
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
        assert_eq!(client.model, DEFAULT_MODEL);
        assert!(client.group_id.is_none());
    }

    #[tokio::test]
    async fn test_minimax_client_with_config() {
        let client = MiniMaxClient::with_config(
            "test-key",
            Some("group-123"),
            Some("https://custom.api.com/v1"),
            Some("custom-model"),
        );
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, "https://custom.api.com/v1");
        assert_eq!(client.model, "custom-model");
        assert_eq!(client.group_id, Some("group-123".to_string()));
    }

    #[tokio::test]
    async fn test_minimax_client_headers() {
        let client = MiniMaxClient::new("secret-key");
        let headers = client.headers().expect("headers should be valid for ASCII key");
        assert_eq!(
            headers.get(AUTHORIZATION).unwrap(),
            "Bearer secret-key"
        );
        assert_eq!(
            headers.get(CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_minimax_error_display() {
        let err = MiniMaxError::Api { status: 401, message: "Unauthorized".into() };
        assert!(err.to_string().contains("401"));
        assert!(err.to_string().contains("Unauthorized"));
    }
}
