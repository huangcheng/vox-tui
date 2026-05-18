use async_trait::async_trait;
use crate::stepfun::ChatMessage;
use crate::config::Provider as ConfigProvider;
pub use crate::minimax::MiniMaxError;

pub type ProviderResult<T> = Result<T, ProviderError>;

/// Result sent from spawned async tasks back to the TUI event loop
#[derive(Debug)]
pub enum WorkResult {
    ChatResponse { content: String, model: String },
    StreamChunk(String),
    StreamDone,
    Error(String),
    ImageGenerated { urls: Vec<String> },
    ImageDownloaded { image_data: Vec<u8> },
    SpeechGenerated { audio_data: Vec<u8>, format: String },
    VideoGenerated { task_id: String, status: String, video_url: Option<String> },
    MusicGenerated { audio_data: Vec<u8>, format: String },
    SearchResults { results: Vec<SearchResult> },
    VisionResult { description: String },
}

#[derive(Debug)]
pub enum ProviderError {
    StepFun(crate::stepfun::StepFunError),
    MiniMax(crate::minimax::MiniMaxError),
    Config(String),
    Unknown(String),
    Unsupported(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::StepFun(e) => write!(f, "StepFun error: {}", e),
            ProviderError::MiniMax(e) => write!(f, "MiniMax error: {}", e),
            ProviderError::Config(msg) => write!(f, "Config error: {}", msg),
            ProviderError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
            ProviderError::Unsupported(msg) => write!(f, "Provider does not support {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn chat(&self, messages: &[Message]) -> ProviderResult<CompletionResponse>;
    async fn image_generate(&self, prompt: &str, n: u8, aspect_ratio: &str) -> ProviderResult<ImageResponse>;
    async fn speech_synthesize(&self, text: &str, voice: &str, speed: f64, format: &str) -> ProviderResult<SpeechResponse>;
    async fn video_generate(&self, prompt: &str, duration: u8, resolution: &str) -> ProviderResult<VideoResponse>;
    async fn music_generate(&self, prompt: &str, lyrics: Option<&str>, instrumental: bool) -> ProviderResult<MusicResponse>;
    async fn search(&self, query: &str, count: u8) -> ProviderResult<SearchResponse>;
    async fn vision(&self, image_path: &str, prompt: Option<&str>) -> ProviderResult<VisionResponse>;
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

impl From<ChatMessage> for Message {
    fn from(msg: ChatMessage) -> Self {
        Message {
            role: msg.role,
            content: msg.content,
        }
    }
}

impl From<Message> for ChatMessage {
    fn from(msg: Message) -> Self {
        ChatMessage {
            role: msg.role,
            content: msg.content,
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

// ── StepFun Provider ────────────────────────────────────────────────

pub struct StepFunProvider {
    client: crate::stepfun::StepFunClient,
}

impl StepFunProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: crate::stepfun::StepFunClient::new(api_key),
        }
    }

    pub fn with_config(api_key: impl Into<String>, base_url: Option<&str>, model: Option<&str>, image_model: Option<&str>) -> Self {
        Self {
            client: crate::stepfun::StepFunClient::with_config(api_key, base_url, model, image_model),
        }
    }
}

#[async_trait]
impl AIProvider for StepFunProvider {
    fn name(&self) -> &str {
        "StepFun"
    }

    async fn chat(&self, messages: &[Message]) -> ProviderResult<CompletionResponse> {
        let api_messages: Vec<ChatMessage> = messages.iter().cloned().map(|m| ChatMessage {
            role: m.role,
            content: m.content,
        }).collect();

        let response = self.client.chat(&api_messages).await
            .map_err(ProviderError::StepFun)?;

        let choice = response.choices.first()
            .ok_or_else(|| ProviderError::Unknown("No choices in response".into()))?;

        Ok(CompletionResponse {
            content: choice.message.content.clone(),
            model: response.model,
            usage: response.usage.map(|u| UsageInfo {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn image_generate(&self, prompt: &str, _n: u8, _aspect_ratio: &str) -> ProviderResult<ImageResponse> {
        self.client.image_generate(prompt).await
            .map_err(ProviderError::StepFun)
    }

    async fn speech_synthesize(&self, text: &str, voice: &str, speed: f64, format: &str) -> ProviderResult<SpeechResponse> {
        self.client.speech_synthesize(text, voice, speed, format).await
            .map_err(ProviderError::StepFun)
    }

    async fn video_generate(&self, _prompt: &str, _duration: u8, _resolution: &str) -> ProviderResult<VideoResponse> {
        Err(ProviderError::Unsupported("video generation".into()))
    }

    async fn music_generate(&self, _prompt: &str, _lyrics: Option<&str>, _instrumental: bool) -> ProviderResult<MusicResponse> {
        Err(ProviderError::Unsupported("music generation".into()))
    }

    async fn search(&self, _query: &str, _count: u8) -> ProviderResult<SearchResponse> {
        Err(ProviderError::Unsupported("search".into()))
    }

    async fn vision(&self, _image_path: &str, _prompt: Option<&str>) -> ProviderResult<VisionResponse> {
        Err(ProviderError::Unsupported("vision".into()))
    }
}

// ── MiniMax Provider ────────────────────────────────────────────────

pub struct MiniMaxProvider {
    client: crate::minimax::MiniMaxClient,
}

impl MiniMaxProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: crate::minimax::MiniMaxClient::new(api_key),
        }
    }

    pub fn with_config(
        api_key: impl Into<String>,
        group_id: Option<&str>,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Self {
        Self {
            client: crate::minimax::MiniMaxClient::with_config(api_key, group_id, base_url, model),
        }
    }
}

#[async_trait]
impl AIProvider for MiniMaxProvider {
    fn name(&self) -> &str {
        "MiniMax"
    }

    async fn chat(&self, messages: &[Message]) -> ProviderResult<CompletionResponse> {
        let response = self.client.chat(messages).await
            .map_err(ProviderError::MiniMax)?;

        Ok(CompletionResponse {
            content: response.reply,
            model: response.model,
            usage: Some(UsageInfo {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            }),
        })
    }

    async fn image_generate(&self, prompt: &str, n: u8, aspect_ratio: &str) -> ProviderResult<ImageResponse> {
        self.client.image_generate(prompt, n, aspect_ratio).await
            .map_err(ProviderError::MiniMax)
    }

    async fn speech_synthesize(&self, text: &str, voice: &str, speed: f64, format: &str) -> ProviderResult<SpeechResponse> {
        self.client.speech_synthesize(text, voice, speed, format).await
            .map_err(ProviderError::MiniMax)
    }

    async fn video_generate(&self, prompt: &str, duration: u8, resolution: &str) -> ProviderResult<VideoResponse> {
        self.client.video_generate(prompt, duration, resolution).await
            .map_err(ProviderError::MiniMax)
    }

    async fn music_generate(&self, prompt: &str, lyrics: Option<&str>, instrumental: bool) -> ProviderResult<MusicResponse> {
        self.client.music_generate(prompt, lyrics, instrumental).await
            .map_err(ProviderError::MiniMax)
    }

    async fn search(&self, query: &str, count: u8) -> ProviderResult<SearchResponse> {
        self.client.search(query, count).await
            .map_err(ProviderError::MiniMax)
    }

    async fn vision(&self, image_path: &str, prompt: Option<&str>) -> ProviderResult<VisionResponse> {
        self.client.vision(image_path, prompt).await
            .map_err(ProviderError::MiniMax)
    }
}

// ── Retry logic ─────────────────────────────────────────────────────

fn is_transient(err: &ProviderError) -> bool {
    match err {
        ProviderError::StepFun(crate::stepfun::StepFunError::Api { status, .. }) if *status >= 500 => true,
        ProviderError::MiniMax(MiniMaxError::Api { status, .. }) if *status >= 500 => true,
        ProviderError::StepFun(crate::stepfun::StepFunError::Http(_)) => true,
        ProviderError::MiniMax(MiniMaxError::Http(_)) => true,
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

    async fn image_generate(&self, prompt: &str, n: u8, aspect_ratio: &str) -> ProviderResult<ImageResponse> {
        retry(|| self.inner.image_generate(prompt, n, aspect_ratio)).await
    }

    async fn speech_synthesize(&self, text: &str, voice: &str, speed: f64, format: &str) -> ProviderResult<SpeechResponse> {
        retry(|| self.inner.speech_synthesize(text, voice, speed, format)).await
    }

    async fn video_generate(&self, prompt: &str, duration: u8, resolution: &str) -> ProviderResult<VideoResponse> {
        retry(|| self.inner.video_generate(prompt, duration, resolution)).await
    }

    async fn music_generate(&self, prompt: &str, lyrics: Option<&str>, instrumental: bool) -> ProviderResult<MusicResponse> {
        retry(|| self.inner.music_generate(prompt, lyrics, instrumental)).await
    }

    async fn search(&self, query: &str, count: u8) -> ProviderResult<SearchResponse> {
        retry(|| self.inner.search(query, count)).await
    }

    async fn vision(&self, image_path: &str, prompt: Option<&str>) -> ProviderResult<VisionResponse> {
        retry(|| self.inner.vision(image_path, prompt)).await
    }
}

// ── Factory ─────────────────────────────────────────────────────────

pub fn create_provider(config: &crate::config::Config) -> ProviderResult<Box<dyn AIProvider>> {
    let provider: Box<dyn AIProvider> = match &config.default_provider {
        ConfigProvider::StepFun => {
            let stepfun = config.stepfun.as_ref()
                .ok_or_else(|| ProviderError::Config("StepFun config missing".into()))?;
            let image_model = stepfun.models.image.as_ref().and_then(|m| m.default.as_deref());
            Box::new(StepFunProvider::with_config(
                &stepfun.api_key,
                stepfun.base_url.as_deref(),
                stepfun.model.as_deref(),
                image_model,
            ))
        }
        ConfigProvider::MiniMax => {
            let minimax = config.minimax.as_ref()
                .ok_or_else(|| ProviderError::Config("MiniMax config missing".into()))?;
            Box::new(MiniMaxProvider::with_config(
                &minimax.api_key,
                minimax.group_id.as_deref(),
                minimax.base_url.as_deref(),
                minimax.model.as_deref(),
            ))
        }
    };
    Ok(Box::new(RetryProvider::new(provider)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, StepFunConfig, MiniMaxConfig};

    #[test]
    fn test_message_creation() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");

        let msg = Message::assistant("world");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "world");

        let msg = Message::system("system prompt");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "system prompt");
    }

    #[test]
    fn test_message_conversion() {
        let chat_msg = ChatMessage::user("test");
        let msg = Message::from(chat_msg);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "test");

        let msg = Message::assistant("response");
        let chat_msg = ChatMessage::from(msg);
        assert_eq!(chat_msg.role, "assistant");
        assert_eq!(chat_msg.content, "response");
    }

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::Config("missing key".into());
        assert!(err.to_string().contains("Config error"));
        assert!(err.to_string().contains("missing key"));

        let err = ProviderError::Unknown("unexpected".into());
        assert!(err.to_string().contains("Unknown error"));
    }

    #[test]
    fn test_create_provider_stepfun() {
        let config = Config {
            default_provider: ConfigProvider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "test-key".into(),
                base_url: None,
                model: None,
                models: crate::config::ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.name(), "StepFun");
    }

    #[test]
    fn test_create_provider_minimax() {
        let config = Config {
            default_provider: ConfigProvider::MiniMax,
            stepfun: None,
            minimax: Some(MiniMaxConfig {
                api_key: "test-key".into(),
                group_id: None,
                base_url: None,
                model: None,
                models: crate::config::ProviderModels::default(),
            }),
            theme: None,
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.name(), "MiniMax");
    }

    #[test]
    fn test_create_provider_missing_config() {
        let config = Config {
            default_provider: ConfigProvider::StepFun,
            stepfun: None,
            minimax: None,
            theme: None,
        };
        let result = create_provider(&config);
        assert!(result.is_err());
        match result {
            Err(ProviderError::Config(msg)) => assert!(msg.contains("StepFun")),
            _ => panic!("Expected Config error"),
        }
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
        assert_eq!(response.model, "test-model");
        assert!(response.usage.is_some());
    }

    #[test]
    fn test_provider_error_unsupported() {
        let err = ProviderError::Unsupported("image generation".into());
        assert!(err.to_string().contains("Provider does not support"));
        assert!(err.to_string().contains("image generation"));
    }

    #[test]
    fn test_work_result_variants() {
        // Construct each variant to ensure the enum compiles.
        let variants = [
            WorkResult::ChatResponse { content: "hi".into(), model: "m".into() },
            WorkResult::StreamChunk("chunk".into()),
            WorkResult::StreamDone,
            WorkResult::Error("fail".into()),
            WorkResult::ImageGenerated { urls: vec!["u".into()] },
            WorkResult::SpeechGenerated { audio_data: vec![1, 2, 3], format: "mp3".into() },
            WorkResult::VideoGenerated { task_id: "t".into(), status: "ok".into(), video_url: None },
            WorkResult::MusicGenerated { audio_data: vec![], format: "mp3".into() },
            WorkResult::SearchResults { results: vec![] },
            WorkResult::VisionResult { description: "desc".into() },
        ];
        assert_eq!(variants.len(), 10);
    }

    // ── Retry logic tests ───────────────────────────────────────────────

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

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
                    Err(ProviderError::StepFun(crate::stepfun::StepFunError::Api {
                        status: 500,
                        message: "server error".into(),
                    }))
                } else {
                    Err(ProviderError::Unknown("non-transient".into()))
                }
            } else {
                Ok(CompletionResponse {
                    content: "ok".into(),
                    model: "mock".into(),
                    usage: None,
                })
            }
        }

        async fn image_generate(&self, _prompt: &str, _n: u8, _aspect_ratio: &str) -> ProviderResult<ImageResponse> {
            Err(ProviderError::Unsupported("test".into()))
        }

        async fn speech_synthesize(&self, _text: &str, _voice: &str, _speed: f64, _format: &str) -> ProviderResult<SpeechResponse> {
            Err(ProviderError::Unsupported("test".into()))
        }

        async fn video_generate(&self, _prompt: &str, _duration: u8, _resolution: &str) -> ProviderResult<VideoResponse> {
            Err(ProviderError::Unsupported("test".into()))
        }

        async fn music_generate(&self, _prompt: &str, _lyrics: Option<&str>, _instrumental: bool) -> ProviderResult<MusicResponse> {
            Err(ProviderError::Unsupported("test".into()))
        }

        async fn search(&self, _query: &str, _count: u8) -> ProviderResult<SearchResponse> {
            Err(ProviderError::Unsupported("test".into()))
        }

        async fn vision(&self, _image_path: &str, _prompt: Option<&str>) -> ProviderResult<VisionResponse> {
            Err(ProviderError::Unsupported("test".into()))
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
        assert_eq!(counter.load(Ordering::SeqCst), 3); // initial + 2 retries
    }

    #[tokio::test]
    async fn test_retry_returns_last_error_after_max_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mock = MockProvider {
            call_count: counter.clone(),
            errors_before_success: 10, // more than max retries
            transient: true,
        };
        let retry = RetryProvider::new(Box::new(mock));
        let result = retry.chat(&[Message::user("hello")]).await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 4); // initial + 3 retries
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
        assert_eq!(counter.load(Ordering::SeqCst), 1); // only initial attempt
    }

    #[test]
    fn test_is_transient_detection() {
        assert!(is_transient(&ProviderError::StepFun(crate::stepfun::StepFunError::Api {
            status: 500,
            message: "err".into(),
        })));
        assert!(is_transient(&ProviderError::StepFun(crate::stepfun::StepFunError::Api {
            status: 502,
            message: "err".into(),
        })));
        assert!(is_transient(&ProviderError::MiniMax(MiniMaxError::Api {
            status: 503,
            message: "err".into(),
        })));
        assert!(!is_transient(&ProviderError::StepFun(crate::stepfun::StepFunError::Api {
            status: 400,
            message: "err".into(),
        })));
        assert!(!is_transient(&ProviderError::MiniMax(MiniMaxError::Api {
            status: 404,
            message: "err".into(),
        })));
        assert!(!is_transient(&ProviderError::Unknown("err".into())));
    }
}