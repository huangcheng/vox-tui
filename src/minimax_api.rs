use crate::minimax::MiniMaxError;

const BASE_URL: &str = "https://api.minimax.chat/v1";

pub struct MiniMaxMultimodal {
    api_key: String,
    group_id: Option<String>,
    client: reqwest::Client,
}

impl MiniMaxMultimodal {
    pub fn new(api_key: impl Into<String>, group_id: Option<String>) -> Self {
        Self {
            api_key: api_key.into(),
            group_id,
            client: reqwest::Client::new(),
        }
    }

    fn headers(&self) -> Result<reqwest::header::HeaderMap, MiniMaxError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.api_key)
                .parse::<reqwest::header::HeaderValue>()
                .map_err(|e| MiniMaxError::Header(e.to_string()))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json"
                .parse::<reqwest::header::HeaderValue>()
                .map_err(|e| MiniMaxError::Header(e.to_string()))?,
        );
        Ok(headers)
    }

    pub async fn image_generate(
        &self,
        prompt: &str,
        n: u8,
        aspect_ratio: &str,
    ) -> Result<crate::provider::ImageResponse, MiniMaxError> {
        let group_id = self.group_id.as_deref().ok_or_else(|| {
            MiniMaxError::Header("group_id required for image generation".into())
        })?;
        let url = format!("{}/image_generation?GroupId={}", BASE_URL, group_id);

        let body = serde_json::json!({
            "model": "image-01",
            "prompt": prompt,
            "n": n,
            "aspect_ratio": aspect_ratio,
        });

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
            return Err(MiniMaxError::Header(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(MiniMaxError::Parse)?;
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

    pub async fn speech_synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
        format: &str,
    ) -> Result<crate::provider::SpeechResponse, MiniMaxError> {
        let url = format!(
            "{}/t2a_v2?GroupId={}",
            BASE_URL,
            self.group_id.as_deref().unwrap_or("0")
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
            return Err(MiniMaxError::Header(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(MiniMaxError::Parse)?;
        let audio_hex = data["data"]["audio"].as_str().unwrap_or("");
        let audio_data = hex_to_bytes(audio_hex);

        Ok(crate::provider::SpeechResponse {
            audio_data,
            format: format.to_string(),
        })
    }

    pub async fn search(
        &self,
        query: &str,
        _count: u8,
    ) -> Result<crate::provider::SearchResponse, MiniMaxError> {
        let url = format!(
            "{}/text/chatcompletion_v2?GroupId={}",
            BASE_URL,
            self.group_id.as_deref().unwrap_or("0")
        );

        let body = serde_json::json!({
            "model": "MiniMax-Text-01",
            "messages": [{ "role": "user", "content": query }],
            "plugins": ["web_search"],
        });

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
            return Err(MiniMaxError::Header(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(MiniMaxError::Parse)?;

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

    pub async fn vision(
        &self,
        image_url: &str,
        prompt: Option<&str>,
    ) -> Result<crate::provider::VisionResponse, MiniMaxError> {
        let url = format!(
            "{}/text/chatcompletion_v2?GroupId={}",
            BASE_URL,
            self.group_id.as_deref().unwrap_or("0")
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
            return Err(MiniMaxError::Header(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(MiniMaxError::Parse)?;
        let description = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No description available")
            .to_string();

        Ok(crate::provider::VisionResponse { description })
    }

    pub async fn video_generate(
        &self,
        prompt: &str,
        _duration: u8,
        _resolution: &str,
    ) -> Result<crate::provider::VideoResponse, MiniMaxError> {
        let url = format!(
            "{}/video_generation?GroupId={}",
            BASE_URL,
            self.group_id.as_deref().unwrap_or("0")
        );

        let body = serde_json::json!({
            "model": "video-01",
            "prompt": prompt,
        });

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
            return Err(MiniMaxError::Header(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(MiniMaxError::Parse)?;
        let task_id = data["task_id"].as_str().unwrap_or("unknown").to_string();

        Ok(crate::provider::VideoResponse {
            task_id,
            status: "processing".to_string(),
            video_url: None,
        })
    }

    pub async fn music_generate(
        &self,
        prompt: &str,
        lyrics: Option<&str>,
        instrumental: bool,
    ) -> Result<crate::provider::MusicResponse, MiniMaxError> {
        let url = format!(
            "{}/music_generation?GroupId={}",
            BASE_URL,
            self.group_id.as_deref().unwrap_or("0")
        );

        let mut body = serde_json::json!({
            "model": "music-01",
            "prompt": prompt,
            "instrumental": instrumental,
        });

        if let Some(lyrics_text) = lyrics {
            body["lyrics"] = serde_json::json!(lyrics_text);
        }

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
            return Err(MiniMaxError::Header(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(MiniMaxError::Parse)?;
        let audio_hex = data["data"]["audio"].as_str().unwrap_or("");
        let audio_data = hex_to_bytes(audio_hex);

        Ok(crate::provider::MusicResponse {
            audio_data,
            format: "mp3".to_string(),
        })
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multimodal_new() {
        let mm = MiniMaxMultimodal::new("test-key", Some("group123".into()));
        assert_eq!(mm.api_key, "test-key");
        assert_eq!(mm.group_id, Some("group123".into()));
    }

    #[tokio::test]
    async fn test_multimodal_new_no_group() {
        let mm = MiniMaxMultimodal::new("test-key", None);
        assert!(mm.group_id.is_none());
    }

    #[tokio::test]
    async fn test_multimodal_headers() {
        let mm = MiniMaxMultimodal::new("test-key", None);
        let headers = mm.headers().unwrap();
        assert!(headers.contains_key(reqwest::header::AUTHORIZATION));
        assert!(headers.contains_key(reqwest::header::CONTENT_TYPE));
    }

    #[tokio::test]
    async fn test_multimodal_image_requires_group_id() {
        let mm = MiniMaxMultimodal::new("test-key", None);
        let result = mm.image_generate("a cat", 1, "1:1").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("group_id"),
            "Expected group_id error, got: {}",
            msg
        );
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
}
