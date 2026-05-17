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

    fn group_id_param(&self) -> &str {
        self.group_id.as_deref().unwrap_or("0")
    }

    /// POST JSON to a URL, handle non-success status
    async fn post_json(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value, MiniMaxError> {
        let resp = self
            .client
            .post(url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(MiniMaxError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MiniMaxError::Api { status: status.as_u16(), message: text });
        }

        resp.json::<serde_json::Value>().await.map_err(MiniMaxError::Parse)
    }

    // ── Chat ──────────────────────────────────────────────────────

    pub async fn chat(
        &self,
        messages: &[crate::provider::Message],
    ) -> Result<ChatResponse, MiniMaxError> {
        let url = format!("{}/text/chat", self.base_url);

        let body = ChatRequest {
            group_id: self.group_id_param(),
            model: &self.model,
            messages: messages.iter().map(|m| MiniMaxMessage {
                sender_type: if m.role == "user" { "USER" } else { "BOT" }.to_string(),
                text: &m.content,
            }).collect(),
            stream: false,
            temperature: None,
            max_tokens: None,
        };

        let resp = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(MiniMaxError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MiniMaxError::Api { status: status.as_u16(), message: text });
        }

        resp.json::<ChatResponse>().await.map_err(MiniMaxError::Parse)
    }

    // ── Image Generation ──────────────────────────────────────────

    pub async fn image_generate(
        &self,
        prompt: &str,
        n: u8,
        aspect_ratio: &str,
    ) -> Result<crate::provider::ImageResponse, MiniMaxError> {
        let group_id = self.group_id.as_deref().ok_or_else(|| {
            MiniMaxError::Header("group_id required for image generation".into())
        })?;
        let url = format!("{}/image_generation?GroupId={}", self.base_url, group_id);

        let body = serde_json::json!({
            "model": "image-01",
            "prompt": prompt,
            "n": n,
            "aspect_ratio": aspect_ratio,
        });

        let data = self.post_json(&url, body).await?;
        let urls = data["data"]["image_urls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["image_url"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(crate::provider::ImageResponse { urls })
    }

    // ── Speech Synthesis ──────────────────────────────────────────

    pub async fn speech_synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
        format: &str,
    ) -> Result<crate::provider::SpeechResponse, MiniMaxError> {
        let url = format!(
            "{}/t2a_v2?GroupId={}",
            self.base_url,
            self.group_id_param()
        );

        let body = serde_json::json!({
            "model": "speech-02-hd",
            "text": text,
            "stream": false,
            "voice_setting": {
                "voice_id": voice,
                "speed": speed,
            },
            "audio_setting": {
                "sample_rate": 32000,
                "format": format,
                "channel": 1,
            },
        });

        let data = self.post_json(&url, body).await?;
        let audio_hex = data["data"]["audio"].as_str().unwrap_or("");
        let audio_data = hex_to_bytes(audio_hex);

        Ok(crate::provider::SpeechResponse {
            audio_data,
            format: format.to_string(),
        })
    }

    // ── Video Generation ──────────────────────────────────────────

    pub async fn video_generate(
        &self,
        prompt: &str,
        _duration: u8,
        _resolution: &str,
    ) -> Result<crate::provider::VideoResponse, MiniMaxError> {
        let url = format!(
            "{}/video_generation?GroupId={}",
            self.base_url,
            self.group_id_param()
        );

        let body = serde_json::json!({
            "model": "video-01",
            "prompt": prompt,
        });

        let data = self.post_json(&url, body).await?;
        let task_id = data["task_id"].as_str().unwrap_or("unknown").to_string();

        Ok(crate::provider::VideoResponse {
            task_id,
            status: "processing".to_string(),
            video_url: None,
        })
    }

    // ── Music Generation ──────────────────────────────────────────

    pub async fn music_generate(
        &self,
        prompt: &str,
        lyrics: Option<&str>,
        instrumental: bool,
    ) -> Result<crate::provider::MusicResponse, MiniMaxError> {
        let url = format!(
            "{}/music_generation?GroupId={}",
            self.base_url,
            self.group_id_param()
        );

        let mut body = serde_json::json!({
            "model": "music-01",
            "prompt": prompt,
            "instrumental": instrumental,
        });

        if let Some(lyrics_text) = lyrics {
            body["lyrics"] = serde_json::json!(lyrics_text);
        }

        let data = self.post_json(&url, body).await?;
        let audio_hex = data["data"]["audio"].as_str().unwrap_or("");
        let audio_data = hex_to_bytes(audio_hex);

        Ok(crate::provider::MusicResponse {
            audio_data,
            format: "mp3".to_string(),
        })
    }

    // ── Search ─────────────────────────────────────────────────────

    pub async fn search(
        &self,
        query: &str,
        _count: u8,
    ) -> Result<crate::provider::SearchResponse, MiniMaxError> {
        let url = format!(
            "{}/text/chatcompletion_v2?GroupId={}",
            self.base_url,
            self.group_id_param()
        );

        let body = serde_json::json!({
            "model": "MiniMax-Text-01",
            "messages": [{ "role": "user", "content": query }],
            "plugins": ["web_search"],
        });

        let data = self.post_json(&url, body).await?;

        let search_results = data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| {
                vec![crate::provider::SearchResult {
                    title: query.to_string(),
                    url: String::new(),
                    snippet: s.to_string(),
                }]
            })
            .unwrap_or_default();

        Ok(crate::provider::SearchResponse {
            results: search_results,
        })
    }

    // ── Vision ─────────────────────────────────────────────────────

    pub async fn vision(
        &self,
        image_url: &str,
        prompt: Option<&str>,
    ) -> Result<crate::provider::VisionResponse, MiniMaxError> {
        let url = format!(
            "{}/text/chatcompletion_v2?GroupId={}",
            self.base_url,
            self.group_id_param()
        );

        let user_content = serde_json::json!([
            { "type": "image_url", "image_url": { "url": image_url } },
            {
                "type": "text",
                "text": prompt.unwrap_or("Describe this image in detail.")
            },
        ]);

        let body = serde_json::json!({
            "model": "MiniMax-VL-01",
            "messages": [{ "role": "user", "content": user_content }],
        });

        let data = self.post_json(&url, body).await?;
        let description = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No description available")
            .to_string();

        Ok(crate::provider::VisionResponse { description })
    }
}

// ── Helper ─────────────────────────────────────────────────────────

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| {
            if i + 2 <= hex.len() {
                u8::from_str_radix(&hex[i..i + 2], 16).ok()
            } else {
                None
            }
        })
        .collect()
}

// ── Chat request/response types ────────────────────────────────────

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

// ── Error type ─────────────────────────────────────────────────────

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

// ── Tests ──────────────────────────────────────────────────────────

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

    #[tokio::test]
    async fn test_image_requires_group_id() {
        let client = MiniMaxClient::new("test-key");
        let result = client.image_generate("a cat", 1, "1:1").await;
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("group_id"), "Expected group_id error, got: {}", msg);
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(
            hex_to_bytes("48656c6c6f"),
            vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]
        );
        assert_eq!(hex_to_bytes(""), Vec::<u8>::new());
        assert_eq!(hex_to_bytes("ff00"), vec![255, 0]);
    }

    #[test]
    fn test_hex_to_bytes_odd_length() {
        let result = hex_to_bytes("f");
        assert!(result.is_empty() || result.len() == 1);
    }

    #[test]
    fn test_group_id_param() {
        let client = MiniMaxClient::with_config("key", Some("g1"), None, None);
        assert_eq!(client.group_id_param(), "g1");

        let client = MiniMaxClient::new("key");
        assert_eq!(client.group_id_param(), "0");
    }
}
