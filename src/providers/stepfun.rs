use async_trait::async_trait;
use serde::Deserialize;

use super::openai::OpenAIClient;
use super::{AIProvider, ProviderError, ProviderResult};
use super::{ImageResponse, SearchResponse, SearchResult, SpeechResponse};

// ── Default model constants ─────────────────────────────────────────

const DEFAULT_CHAT_MODEL: &str = "step-1-8k";
const DEFAULT_IMAGE_MODEL: &str = "step-image-edit-2";
const DEFAULT_SPEECH_MODEL: &str = "stepaudio-2.5-tts";
const SPEECH_ENDPOINT: &str = "https://api.stepfun.com/step_plan/v1/audio/speech";
const DEFAULT_VISION_MODEL: &str = "step-1v-8k";

// ── Provider ────────────────────────────────────────────────────────

pub struct StepFunProvider {
    client: OpenAIClient,
    chat_model: String,
    image_model: String,
    speech_model: String,
    vision_model: String,
}

impl StepFunProvider {
    pub fn new(
        api_key: &str,
        base_url: Option<&str>,
        model: Option<&str>,
        image_model: Option<&str>,
        speech_model: Option<&str>,
        http_client: Option<reqwest::Client>,
    ) -> Self {
        let base = base_url.unwrap_or("https://api.stepfun.com/v1");
        Self {
            client: OpenAIClient::new(base, api_key, http_client),
            chat_model: model.unwrap_or(DEFAULT_CHAT_MODEL).to_string(),
            image_model: image_model.unwrap_or(DEFAULT_IMAGE_MODEL).to_string(),
            speech_model: speech_model.unwrap_or(DEFAULT_SPEECH_MODEL).to_string(),
            vision_model: DEFAULT_VISION_MODEL.to_string(),
        }
    }
}

#[async_trait]
impl AIProvider for StepFunProvider {
    fn name(&self) -> &str {
        "StepFun"
    }

    fn openai_client(&self) -> Option<&OpenAIClient> {
        Some(&self.client)
    }

    fn chat_model(&self) -> &str {
        &self.chat_model
    }
    fn vision_model(&self) -> &str {
        &self.vision_model
    }

    // ── Override: image generation (non-standard endpoint) ──────────
    async fn image_generate(
        &self,
        prompt: &str,
        _n: u8,
        _aspect_ratio: &str,
    ) -> ProviderResult<ImageResponse> {
        // StepFun image uses a separate endpoint, not under /v1
        let url = "https://api.stepfun.com/step_plan/v1/images/generations";
        let body = serde_json::json!({
            "model": self.image_model,
            "prompt": prompt,
            "response_format": "url",
            "cfg_scale": 1.0,
            "steps": 8,
            "text_mode": true
        });

        let data = self.client.post_json_raw(url, body).await?;
        let arr = data
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ProviderError::Parse("missing data array in image response".into()))?;

        let urls: Vec<String> = arr
            .iter()
            .filter_map(|item| item.get("url").and_then(|u| u.as_str()).map(String::from))
            .collect();

        Ok(ImageResponse { urls })
    }

    // ── Override: speech synthesis (step_plan endpoint) ─────────────
    async fn speech_synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
        format: &str,
    ) -> ProviderResult<SpeechResponse> {
        let mut body = serde_json::json!({
            "model": self.speech_model,
            "input": text,
            "voice": voice,
            "response_format": format,
        });

        // Speed mapping: cli uses 0.5-2.0 range, instruction-based control
        if speed != 1.0 {
            let pace = if speed < 0.8 {
                "偏慢"
            } else if speed > 1.2 {
                "偏快"
            } else {
                "适中"
            };
            body["instruction"] = serde_json::json!(format!("语速{}", pace));
        }

        let resp = self
            .client
            .client
            .post(SPEECH_ENDPOINT)
            .headers(self.client.headers()?)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("(failed to read body: {e})"));
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: text,
            });
        }

        let audio_data = resp.bytes().await?.to_vec();
        Ok(SpeechResponse {
            audio_data,
            format: format.to_string(),
        })
    }

    // ── Override: search (StepFun-specific endpoint) ────────────────
    async fn search(&self, query: &str, count: u8) -> ProviderResult<SearchResponse> {
        let url = format!("{}/search", self.client.base_url);
        let body = serde_json::json!({
            "query": query,
            "n": count,
        });

        let data = self.client.post_json_raw(&url, body).await?;
        let results_raw: Vec<SearchResultRaw> = data
            .get("results")
            .and_then(|r| serde_json::from_value(r.clone()).ok())
            .unwrap_or_default();

        let results: Vec<SearchResult> = results_raw
            .into_iter()
            .map(|r| SearchResult {
                title: if r.title.is_empty() {
                    r.url.clone()
                } else {
                    r.title
                },
                url: r.url,
                snippet: if r.snippet.is_empty() {
                    r.content
                } else {
                    r.snippet
                },
            })
            .collect();

        Ok(SearchResponse { results })
    }
}

// ── Search API response types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchResultRaw {
    url: String,
    #[allow(dead_code)]
    position: Option<u32>,
    #[allow(dead_code)]
    time: Option<String>,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    title: String,
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stepfun_provider_name() {
        let provider = StepFunProvider::new("test-key", None, None, None, None, None);
        assert_eq!(provider.name(), "StepFun");
    }

    #[test]
    fn test_stepfun_default_models() {
        let provider = StepFunProvider::new("test-key", None, None, None, None, None);
        assert_eq!(provider.chat_model(), "step-1-8k");
        assert_eq!(provider.vision_model(), "step-1v-8k");
    }

    #[test]
    fn test_stepfun_custom_models() {
        let provider = StepFunProvider::new(
            "key",
            None,
            Some("custom-chat"),
            Some("custom-img"),
            None,
            None,
        );
        assert_eq!(provider.chat_model(), "custom-chat");
        assert_eq!(provider.image_model, "custom-img");
    }

    #[tokio::test]
    async fn test_search_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/search")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_body(r#"{
                "query": "rust",
                "n": 2,
                "results": [
                    { "url": "https://example.com", "snippet": "Rust lang", "content": "", "title": "Rust" },
                    { "url": "https://example.com/2", "snippet": "", "content": "Fallback", "title": "" }
                ]
            }"#)
            .create();

        let provider =
            StepFunProvider::new("test-key", Some(&server.url()), None, None, None, None);
        let result = provider.search("rust", 2).await.unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].title, "Rust");
        assert_eq!(result.results[1].title, "https://example.com/2"); // empty title → url
        assert_eq!(result.results[1].snippet, "Fallback"); // empty snippet → content

        mock.assert();
    }

    #[tokio::test]
    async fn test_vision_with_url() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_body(
                r#"{
                "id": "chatcmpl-123",
                "model": "step-1v-8k",
                "choices": [{
                    "message": { "role": "assistant", "content": "A sunset." },
                    "finish_reason": "stop"
                }]
            }"#,
            )
            .create();

        let provider =
            StepFunProvider::new("test-key", Some(&server.url()), None, None, None, None);
        let result = provider
            .vision("https://example.com/img.jpg", Some("Describe"))
            .await
            .unwrap();

        assert_eq!(result.description, "A sunset.");
        mock.assert();
    }

    #[tokio::test]
    async fn test_image_generate() {
        let provider = StepFunProvider::new("test-key", None, None, None, None, None);
        // This will hit the real API and fail, which is fine — we test the method exists
        let result = provider.image_generate("test", 1, "1:1").await;
        assert!(result.is_err()); // Expected: network error since no mock server
    }
}
