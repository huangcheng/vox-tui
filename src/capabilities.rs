use crate::config::Provider;

/// Static capability registry per provider.
/// Checked BEFORE any API call to give instant feedback.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub chat: bool,
    pub image_generate: bool,
    pub speech_synthesize: bool,
    pub video_generate: bool,
    pub music_generate: bool,
    pub search: bool,
    pub vision: bool,
}

impl ProviderCapabilities {
    pub fn for_provider(provider: &Provider) -> &'static Self {
        match provider {
            Provider::MiniMax => &MINIMAX_CAPABILITIES,
            Provider::StepFun => &STEPFUN_CAPABILITIES,
        }
    }

    /// Check if a capability is supported. Returns error message if not.
    pub fn require(&self, capability: &str, provider: &crate::config::Provider) -> Result<(), String> {
        let provider_name = match provider {
            crate::config::Provider::StepFun => "StepFun",
            crate::config::Provider::MiniMax => "MiniMax",
        };
        let supported = match capability {
            "chat" => self.chat,
            "image_generate" => self.image_generate,
            "speech_synthesize" => self.speech_synthesize,
            "video_generate" => self.video_generate,
            "music_generate" => self.music_generate,
            "search" => self.search,
            "vision" => self.vision,
            _ => return Err(format!("Unknown capability: {capability}")),
        };
        if supported {
            Ok(())
        } else {
            Err(format!("{provider_name} does not support {capability}"))
        }
    }
}

static MINIMAX_CAPABILITIES: ProviderCapabilities = ProviderCapabilities {
    chat: true,
    image_generate: true,
    speech_synthesize: true,
    video_generate: true,
    music_generate: true,
    search: true,
    vision: true,
};

static STEPFUN_CAPABILITIES: ProviderCapabilities = ProviderCapabilities {
    chat: true,
    image_generate: true,
    speech_synthesize: true,
    video_generate: false,
    music_generate: false,
    search: true,
    vision: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimax_has_all_capabilities() {
        let cap = ProviderCapabilities::for_provider(&Provider::MiniMax);
        assert!(cap.chat);
        assert!(cap.video_generate);
        assert!(cap.music_generate);
    }

    #[test]
    fn test_stepfun_lacks_video_and_music() {
        let cap = ProviderCapabilities::for_provider(&Provider::StepFun);
        assert!(!cap.video_generate);
        assert!(!cap.music_generate);
        assert!(cap.chat);
        assert!(cap.image_generate);
    }

    #[test]
    fn test_require_supported() {
        let cap = ProviderCapabilities::for_provider(&Provider::MiniMax);
        assert!(cap.require("video_generate", &Provider::MiniMax).is_ok());
    }

    #[test]
    fn test_require_unsupported() {
        let cap = ProviderCapabilities::for_provider(&Provider::StepFun);
        let result = cap.require("video_generate", &Provider::StepFun);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("StepFun does not support video_generate"));
    }
}
