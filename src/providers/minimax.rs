use async_trait::async_trait;

use super::openai::OpenAIClient;
use super::{AIProvider, ProviderResult};
use super::{
    ImageResponse, MusicResponse, SearchResponse, SearchResult, SpeechResponse, VideoResponse,
};

// ── Default model constants ─────────────────────────────────────────

const DEFAULT_CHAT_MODEL: &str = "MiniMax-M2.7";
const DEFAULT_SPEECH_MODEL: &str = "speech-2.8-hd";
const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com/v1";

// ── Provider ────────────────────────────────────────────────────────

pub struct MiniMaxProvider {
    client: OpenAIClient,
    model: String,
    speech_model: String,
}

impl MiniMaxProvider {
    pub fn new(
        api_key: &str,
        _group_id: Option<&str>, // kept for config compat, no longer used
        base_url: Option<&str>,
        model: Option<&str>,
        http_client: Option<reqwest::Client>,
    ) -> Self {
        let base = base_url.unwrap_or(DEFAULT_BASE_URL);
        Self {
            client: OpenAIClient::new(base, api_key, http_client),
            model: model.unwrap_or(DEFAULT_CHAT_MODEL).to_string(),
            speech_model: DEFAULT_SPEECH_MODEL.to_string(),
        }
    }
}

#[async_trait]
impl AIProvider for MiniMaxProvider {
    fn name(&self) -> &str {
        "MiniMax"
    }

    // MiniMax now uses standard OpenAI-compatible /v1/chat/completions.
    // Return the shared client to get default chat + vision impls.
    fn openai_client(&self) -> Option<&OpenAIClient> {
        Some(&self.client)
    }

    fn chat_model(&self) -> &str {
        &self.model
    }
    fn vision_model(&self) -> &str {
        &self.model
    }

    // ── Image generation ────────────────────────────────────────────
    async fn image_generate(
        &self,
        prompt: &str,
        n: u8,
        aspect_ratio: &str,
    ) -> ProviderResult<ImageResponse> {
        let url = format!("{}/image_generation", self.client.base_url);
        let body = serde_json::json!({
            "model": "image-01",
            "prompt": prompt,
            "n": n,
            "aspect_ratio": aspect_ratio,
            "response_format": "url",
        });

        let data = self.client.post_json_raw(&url, body).await?;
        let urls = data["data"]["image_urls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ImageResponse { urls })
    }

    // ── Speech synthesis (t2a_v2) ───────────────────────────────────
    async fn speech_synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
        format: &str,
    ) -> ProviderResult<SpeechResponse> {
        let url = format!("{}/t2a_v2", self.client.base_url);
        let body = serde_json::json!({
            "model": self.speech_model,
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

        let data = self.client.post_json_raw(&url, body).await?;
        let audio_hex = data["data"]["audio"].as_str().unwrap_or("");
        let audio_data = hex_to_bytes(audio_hex);

        Ok(SpeechResponse {
            audio_data,
            format: format.to_string(),
        })
    }

    // ── Video generation ────────────────────────────────────────────
    async fn video_generate(
        &self,
        prompt: &str,
        _duration: u8,
        _resolution: &str,
    ) -> ProviderResult<VideoResponse> {
        let url = format!("{}/video_generation", self.client.base_url);
        let body = serde_json::json!({
            "model": "MiniMax-Hailuo-2.3",
            "prompt": prompt,
        });

        let data = self.client.post_json_raw(&url, body).await?;
        let task_id = data["task_id"].as_str().unwrap_or("unknown").to_string();

        Ok(VideoResponse {
            task_id,
            status: "processing".to_string(),
            video_url: None,
        })
    }

    // ── Music generation ────────────────────────────────────────────
    async fn music_generate(
        &self,
        prompt: &str,
        lyrics: Option<&str>,
        instrumental: bool,
    ) -> ProviderResult<MusicResponse> {
        let url = format!("{}/music_generation", self.client.base_url);
        let mut body = serde_json::json!({
            "model": "music-2.6",
            "prompt": prompt,
            "is_instrumental": instrumental,
        });

        if let Some(lyrics_text) = lyrics {
            body["lyrics"] = serde_json::json!(lyrics_text);
        }

        let data = self.client.post_json_raw(&url, body).await?;
        let audio_hex = data["data"]["audio"].as_str().unwrap_or("");
        let audio_data = hex_to_bytes(audio_hex);

        Ok(MusicResponse {
            audio_data,
            format: "mp3".to_string(),
        })
    }

    // ── Search (via chat with web_search plugin) ────────────────────
    async fn search(&self, query: &str, _count: u8) -> ProviderResult<SearchResponse> {
        let url = format!("{}/chat/completions", self.client.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{ "role": "user", "content": query }],
            "plugins": ["web_search"],
        });

        let data = self.client.post_json_raw(&url, body).await?;

        let results = data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| {
                vec![SearchResult {
                    title: query.to_string(),
                    url: String::new(),
                    snippet: s.to_string(),
                }]
            })
            .unwrap_or_default();

        Ok(SearchResponse { results })
    }
}

// ── Helper ───────────────────────────────────────────────────────────

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    if hex.is_empty() {
        return Vec::new();
    }
    let hex = if hex.len() % 2 != 0 {
        format!("0{hex}")
    } else {
        hex.to_string()
    };
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Message;

    #[test]
    fn test_minimax_provider_name() {
        let provider = MiniMaxProvider::new("test-key", None, None, None, None);
        assert_eq!(provider.name(), "MiniMax");
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
        assert_eq!(hex_to_bytes("f"), vec![0x0f]);
    }

    #[test]
    fn test_provider_has_openai_client() {
        let provider = MiniMaxProvider::new("test-key", None, None, None, None);
        assert!(provider.openai_client().is_some());
    }

    #[tokio::test]
    async fn test_chat_uses_openai_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_body(
                r#"{
                "id": "chatcmpl-123",
                "model": "test-model",
                "choices": [{
                    "message": { "role": "assistant", "content": "Hello!" },
                    "finish_reason": "stop"
                }],
                "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
            }"#,
            )
            .create();

        let provider = MiniMaxProvider::new(
            "test-key",
            None,
            Some(&server.url()),
            Some("test-model"),
            None,
        );
        let result = provider.chat(&[Message::user("hi")]).await.unwrap();

        assert_eq!(result.content, "Hello!");
        assert_eq!(result.model, "test-model");
        assert!(result.usage.is_some());
        mock.assert();
    }

    #[tokio::test]
    async fn test_image_generate() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/image_generation")
            .with_status(200)
            .with_body(
                r#"{
                "data": { "image_urls": ["https://example.com/img.png"] },
                "base_resp": { "status_code": 0 }
            }"#,
            )
            .create();

        let provider = MiniMaxProvider::new("test-key", None, Some(&server.url()), None, None);
        let result = provider.image_generate("a cat", 1, "1:1").await.unwrap();

        assert_eq!(result.urls, vec!["https://example.com/img.png"]);
        mock.assert();
    }

    #[tokio::test]
    async fn test_speech_uses_t2a_v2() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/t2a_v2")
            .with_status(200)
            .with_body(
                r#"{
                "data": { "audio": "48656c6c6f", "status": 2 },
                "base_resp": { "status_code": 0 }
            }"#,
            )
            .create();

        let provider = MiniMaxProvider::new("test-key", None, Some(&server.url()), None, None);
        let result = provider
            .speech_synthesize("hello", "male-qn-qingse", 1.0, "mp3")
            .await
            .unwrap();

        assert_eq!(result.audio_data, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
        assert_eq!(result.format, "mp3");
        mock.assert();
    }

    #[tokio::test]
    async fn test_video_generate() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/video_generation")
            .with_status(200)
            .with_body(
                r#"{
                "task_id": "12345",
                "base_resp": { "status_code": 0 }
            }"#,
            )
            .create();

        let provider = MiniMaxProvider::new("test-key", None, Some(&server.url()), None, None);
        let result = provider.video_generate("sunset", 6, "1080P").await.unwrap();

        assert_eq!(result.task_id, "12345");
        assert_eq!(result.status, "processing");
        mock.assert();
    }

    #[tokio::test]
    async fn test_music_generate() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/music_generation")
            .with_status(200)
            .with_body(
                r#"{
                "data": { "audio": "ff00", "status": 2 },
                "base_resp": { "status_code": 0 }
            }"#,
            )
            .create();

        let provider = MiniMaxProvider::new("test-key", None, Some(&server.url()), None, None);
        let result = provider
            .music_generate("jazz", Some("la la la"), false)
            .await
            .unwrap();

        assert_eq!(result.audio_data, vec![255, 0]);
        mock.assert();
    }

    #[tokio::test]
    async fn test_search_via_chat() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_body(
                r#"{
                "choices": [{ "message": { "content": "Search result" } }]
            }"#,
            )
            .create();

        let provider = MiniMaxProvider::new("test-key", None, Some(&server.url()), None, None);
        let result = provider.search("test", 5).await.unwrap();

        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].snippet, "Search result");
        mock.assert();
    }
}
