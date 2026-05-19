pub mod minimax;
pub mod openai;
pub mod stepfun;

use async_trait::async_trait;

// Re-export everything at the module level for convenient imports
pub use minimax::MiniMaxProvider;
pub use openai::OpenAIClient;
pub use stepfun::StepFunProvider;

pub type ProviderResult<T> = Result<T, ProviderError>;

// ── Shared Types ────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ProviderError {
    Http(reqwest::Error),
    Parse(String),
    Header(String),
    Api { status: u16, message: String },
    Config(String),
    Unsupported(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Http(e) => write!(f, "HTTP error: {}", e),
            ProviderError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ProviderError::Header(msg) => write!(f, "Header error: {}", msg),
            ProviderError::Api { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            ProviderError::Config(msg) => write!(f, "Config error: {}", msg),
            ProviderError::Unsupported(msg) => write!(f, "Provider does not support {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        ProviderError::Http(e)
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
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

#[derive(Debug)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ImageResponse {
    pub urls: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct SpeechResponse {
    pub audio_data: Vec<u8>,
    pub format: String,
}
#[derive(Debug, Clone)]
pub struct VideoResponse {
    pub task_id: String,
    pub status: String,
    pub video_url: Option<String>,
}
#[derive(Debug, Clone)]
pub struct MusicResponse {
    pub audio_data: Vec<u8>,
    pub format: String,
}
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}
#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}
#[derive(Debug, Clone)]
pub struct VisionResponse {
    pub description: String,
}

/// Result sent from spawned async tasks back to the TUI event loop
#[derive(Debug)]
pub enum WorkResult {
    ChatResponse {
        content: String,
        model: String,
    },
    StreamChunk(String),
    StreamDone,
    Error(String),
    ImageGenerated {
        urls: Vec<String>,
    },
    ImageDownloaded {
        image_data: Vec<u8>,
    },
    SpeechGenerated {
        audio_data: Vec<u8>,
        format: String,
    },
    VideoGenerated {
        task_id: String,
        status: String,
        video_url: Option<String>,
    },
    MusicGenerated {
        audio_data: Vec<u8>,
        format: String,
    },
    SearchResults {
        results: Vec<SearchResult>,
    },
    VisionResult {
        description: String,
    },
}

// ── Shared Helpers ──────────────────────────────────────────────────

pub fn file_to_data_uri(path: &str) -> ProviderResult<String> {
    use base64::Engine;
    use std::io::Read;

    let path = std::path::Path::new(path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime = match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png",
    };

    let mut file = std::fs::File::open(path)
        .map_err(|e| ProviderError::Config(format!("Failed to open image: {}", e)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| ProviderError::Config(format!("Failed to read image: {}", e)))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}

// ── AIProvider Trait with Default Implementations ───────────────────

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;

    /// Return the shared OpenAI-compatible client if this provider supports it.
    /// If None, chat/vision/search default to Unsupported.
    fn openai_client(&self) -> Option<&OpenAIClient> {
        None
    }

    /// Default model for chat completions (used by default chat impl)
    fn chat_model(&self) -> &str {
        "unknown"
    }
    /// Default model for vision (used by default vision impl)
    fn vision_model(&self) -> &str {
        "unknown"
    }

    // ── Default: OpenAI-compatible chat ─────────────────────────────
    async fn chat(&self, messages: &[Message]) -> ProviderResult<CompletionResponse> {
        let client = self
            .openai_client()
            .ok_or_else(|| ProviderError::Unsupported("chat".into()))?;
        let resp = client.chat_completion(self.chat_model(), messages).await?;

        let choice = resp
            .choices
            .first()
            .ok_or_else(|| ProviderError::Parse("No choices in response".into()))?;

        Ok(CompletionResponse {
            content: choice.message.content.clone(),
            model: resp.model,
            usage: resp.usage.map(|u| UsageInfo {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    // ── Default: OpenAI-compatible vision ───────────────────────────
    async fn vision(
        &self,
        image_path: &str,
        prompt: Option<&str>,
    ) -> ProviderResult<VisionResponse> {
        let client = self
            .openai_client()
            .ok_or_else(|| ProviderError::Unsupported("vision".into()))?;

        let image_url = if image_path.starts_with("http://") || image_path.starts_with("https://") {
            image_path.to_string()
        } else {
            file_to_data_uri(image_path)?
        };

        let text = prompt.unwrap_or("Describe this image in detail.");
        let resp = client
            .vision_completion(self.vision_model(), &image_url, text)
            .await?;
        let description = resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "No description available".to_string());

        Ok(VisionResponse { description })
    }

    // ── Default: search via chat (providers should override if they have a search API) ──
    async fn search(&self, _query: &str, _count: u8) -> ProviderResult<SearchResponse> {
        Err(ProviderError::Unsupported("search".into()))
    }

    // ── No defaults: must override if supported ─────────────────────
    async fn image_generate(
        &self,
        _prompt: &str,
        _n: u8,
        _aspect_ratio: &str,
    ) -> ProviderResult<ImageResponse> {
        Err(ProviderError::Unsupported("image generation".into()))
    }
    async fn speech_synthesize(
        &self,
        _text: &str,
        _voice: &str,
        _speed: f64,
        _format: &str,
    ) -> ProviderResult<SpeechResponse> {
        Err(ProviderError::Unsupported("speech synthesis".into()))
    }
    async fn video_generate(
        &self,
        _prompt: &str,
        _duration: u8,
        _resolution: &str,
    ) -> ProviderResult<VideoResponse> {
        Err(ProviderError::Unsupported("video generation".into()))
    }
    async fn music_generate(
        &self,
        _prompt: &str,
        _lyrics: Option<&str>,
        _instrumental: bool,
    ) -> ProviderResult<MusicResponse> {
        Err(ProviderError::Unsupported("music generation".into()))
    }
}

// ── Retry Logic ─────────────────────────────────────────────────────

fn is_transient(err: &ProviderError) -> bool {
    match err {
        ProviderError::Api { status, .. } if *status >= 500 => true,
        ProviderError::Http(_) => true,
        _ => false,
    }
}

async fn retry<T, F, Fut>(operation: F) -> ProviderResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = ProviderResult<T>>,
{
    const MAX_RETRIES: usize = 3;
    const BASE_DELAY_MS: u64 = 500;

    for attempt in 0..=MAX_RETRIES {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                if attempt == MAX_RETRIES || !is_transient(&err) {
                    return Err(err);
                }
                let delay = BASE_DELAY_MS * 2_u64.pow(attempt as u32);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
    unreachable!("loop always returns")
}

pub struct RetryProvider {
    inner: Box<dyn AIProvider>,
}

impl RetryProvider {
    pub fn new(inner: Box<dyn AIProvider>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl AIProvider for RetryProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn chat(&self, messages: &[Message]) -> ProviderResult<CompletionResponse> {
        retry(|| self.inner.chat(messages)).await
    }
    async fn image_generate(
        &self,
        prompt: &str,
        n: u8,
        aspect_ratio: &str,
    ) -> ProviderResult<ImageResponse> {
        retry(|| self.inner.image_generate(prompt, n, aspect_ratio)).await
    }
    async fn speech_synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
        format: &str,
    ) -> ProviderResult<SpeechResponse> {
        retry(|| self.inner.speech_synthesize(text, voice, speed, format)).await
    }
    async fn video_generate(
        &self,
        prompt: &str,
        duration: u8,
        resolution: &str,
    ) -> ProviderResult<VideoResponse> {
        retry(|| self.inner.video_generate(prompt, duration, resolution)).await
    }
    async fn music_generate(
        &self,
        prompt: &str,
        lyrics: Option<&str>,
        instrumental: bool,
    ) -> ProviderResult<MusicResponse> {
        retry(|| self.inner.music_generate(prompt, lyrics, instrumental)).await
    }
    async fn search(&self, query: &str, count: u8) -> ProviderResult<SearchResponse> {
        retry(|| self.inner.search(query, count)).await
    }
    async fn vision(
        &self,
        image_path: &str,
        prompt: Option<&str>,
    ) -> ProviderResult<VisionResponse> {
        retry(|| self.inner.vision(image_path, prompt)).await
    }
}

// ── Factory ─────────────────────────────────────────────────────────

use crate::config::Provider as ConfigProvider;

pub fn create_provider(config: &crate::config::Config) -> ProviderResult<Box<dyn AIProvider>> {
    create_provider_with_client(config, None)
}

pub fn create_provider_with_client(
    config: &crate::config::Config,
    http_client: Option<reqwest::Client>,
) -> ProviderResult<Box<dyn AIProvider>> {
    let provider: Box<dyn AIProvider> = match &config.default_provider {
        ConfigProvider::StepFun => {
            let sf = config
                .stepfun
                .as_ref()
                .ok_or_else(|| ProviderError::Config("StepFun config missing".into()))?;
            Box::new(StepFunProvider::new(
                &sf.api_key,
                sf.base_url.as_deref(),
                sf.model.as_deref(),
                sf.models.image.as_deref(),
                sf.models.speech.as_deref(),
                http_client,
            ))
        }
        ConfigProvider::MiniMax => {
            let mm = config
                .minimax
                .as_ref()
                .ok_or_else(|| ProviderError::Config("MiniMax config missing".into()))?;
            Box::new(MiniMaxProvider::new(
                &mm.api_key,
                mm.group_id.as_deref(),
                mm.base_url.as_deref(),
                mm.model.as_deref(),
                http_client,
            ))
        }
    };
    Ok(Box::new(RetryProvider::new(provider)))
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_message_creation() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");

        let msg = Message::assistant("world");
        assert_eq!(msg.role, "assistant");

        let msg = Message::system("system prompt");
        assert_eq!(msg.role, "system");
    }

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::Config("missing key".into());
        assert!(err.to_string().contains("Config error"));

        let err = ProviderError::Api {
            status: 500,
            message: "server error".into(),
        };
        assert!(err.to_string().contains("500"));

        let err = ProviderError::Unsupported("video".into());
        assert!(err.to_string().contains("does not support"));
    }

    #[test]
    fn test_work_result_variants() {
        let variants = [
            WorkResult::ChatResponse {
                content: "hi".into(),
                model: "m".into(),
            },
            WorkResult::StreamChunk("chunk".into()),
            WorkResult::StreamDone,
            WorkResult::Error("fail".into()),
            WorkResult::ImageGenerated {
                urls: vec!["u".into()],
            },
            WorkResult::SpeechGenerated {
                audio_data: vec![1, 2, 3],
                format: "mp3".into(),
            },
            WorkResult::VideoGenerated {
                task_id: "t".into(),
                status: "ok".into(),
                video_url: None,
            },
            WorkResult::MusicGenerated {
                audio_data: vec![],
                format: "mp3".into(),
            },
            WorkResult::SearchResults { results: vec![] },
            WorkResult::VisionResult {
                description: "desc".into(),
            },
        ];
        assert_eq!(variants.len(), 10);
    }

    #[test]
    fn test_completion_response() {
        let response = CompletionResponse {
            content: "Hello!".into(),
            model: "test-model".into(),
            usage: Some(UsageInfo {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };
        assert_eq!(response.content, "Hello!");
    }

    #[test]
    fn test_create_provider_stepfun() {
        let config = crate::config::Config {
            version: 1,
            default_provider: ConfigProvider::StepFun,
            stepfun: Some(crate::config::StepFunConfig {
                api_key: "test-key".into(),
                base_url: None,
                model: None,
                models: crate::config::ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
            output_dir: None,
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "StepFun");
    }

    #[test]
    fn test_create_provider_minimax() {
        let config = crate::config::Config {
            version: 1,
            default_provider: ConfigProvider::MiniMax,
            stepfun: None,
            minimax: Some(crate::config::MiniMaxConfig {
                api_key: "test-key".into(),
                group_id: None,
                base_url: None,
                model: None,
                models: crate::config::ProviderModels::default(),
            }),
            theme: None,
            output_dir: None,
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "MiniMax");
    }

    #[test]
    fn test_create_provider_missing_config() {
        let config = crate::config::Config {
            version: 1,
            default_provider: ConfigProvider::StepFun,
            stepfun: None,
            minimax: None,
            theme: None,
            output_dir: None,
        };
        let result = create_provider(&config);
        assert!(matches!(result, Err(ProviderError::Config(_))));
    }

    #[test]
    fn test_is_transient_detection() {
        assert!(is_transient(&ProviderError::Api {
            status: 500,
            message: "err".into()
        }));
        assert!(is_transient(&ProviderError::Api {
            status: 502,
            message: "err".into()
        }));
        assert!(!is_transient(&ProviderError::Api {
            status: 400,
            message: "err".into()
        }));
        assert!(!is_transient(&ProviderError::Api {
            status: 404,
            message: "err".into()
        }));
        assert!(!is_transient(&ProviderError::Config("err".into())));
    }

    struct MockProvider {
        call_count: Arc<AtomicUsize>,
        errors_before_success: usize,
        transient: bool,
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str {
            "Mock"
        }
        async fn chat(&self, _messages: &[Message]) -> ProviderResult<CompletionResponse> {
            let count = self.call_count.fetch_add(1, Ordering::SeqCst);
            if count < self.errors_before_success {
                if self.transient {
                    Err(ProviderError::Api {
                        status: 500,
                        message: "server error".into(),
                    })
                } else {
                    Err(ProviderError::Unsupported("test".into()))
                }
            } else {
                Ok(CompletionResponse {
                    content: "ok".into(),
                    model: "mock".into(),
                    usage: None,
                })
            }
        }
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_two_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mock = MockProvider {
            call_count: counter.clone(),
            errors_before_success: 2,
            transient: true,
        };
        let retry = RetryProvider::new(Box::new(mock));
        let result = retry.chat(&[Message::user("hello")]).await;
        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_returns_last_error_after_max_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mock = MockProvider {
            call_count: counter.clone(),
            errors_before_success: 10,
            transient: true,
        };
        let retry = RetryProvider::new(Box::new(mock));
        let result = retry.chat(&[Message::user("hello")]).await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_no_retry_on_non_transient_error() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mock = MockProvider {
            call_count: counter.clone(),
            errors_before_success: 10,
            transient: false,
        };
        let retry = RetryProvider::new(Box::new(mock));
        let result = retry.chat(&[Message::user("hello")]).await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
