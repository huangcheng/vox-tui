use crate::config::Provider;

/// Typed capability identifiers — compile-time exhaustiveness instead of string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Chat,
    ImageGenerate,
    SpeechSynthesize,
    VideoGenerate,
    MusicGenerate,
    Search,
    Vision,
}

impl Capability {
    /// Human-readable name used in error messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::Chat => "chat",
            Capability::ImageGenerate => "image_generate",
            Capability::SpeechSynthesize => "speech_synthesize",
            Capability::VideoGenerate => "video_generate",
            Capability::MusicGenerate => "music_generate",
            Capability::Search => "search",
            Capability::Vision => "vision",
        }
    }
}

/// Static capability registry per provider.
/// Checked BEFORE any API call to give instant feedback.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    chat: bool,
    image_generate: bool,
    speech_synthesize: bool,
    video_generate: bool,
    music_generate: bool,
    search: bool,
    vision: bool,
}

impl ProviderCapabilities {
    pub fn for_provider(provider: &Provider) -> &'static Self {
        match provider {
            Provider::MiniMax => &MINIMAX_CAPABILITIES,
            Provider::StepFun => &STEPFUN_CAPABILITIES,
        }
    }

    /// Check if a capability is supported. Returns error message if not.
    pub fn require(&self, capability: Capability, provider: &Provider) -> Result<(), String> {
        let provider_name = match provider {
            Provider::StepFun => "StepFun",
            Provider::MiniMax => "MiniMax",
        };
        let supported = match capability {
            Capability::Chat => self.chat,
            Capability::ImageGenerate => self.image_generate,
            Capability::SpeechSynthesize => self.speech_synthesize,
            Capability::VideoGenerate => self.video_generate,
            Capability::MusicGenerate => self.music_generate,
            Capability::Search => self.search,
            Capability::Vision => self.vision,
        };
        if supported {
            Ok(())
        } else {
            Err(format!(
                "{provider_name} does not support {}",
                capability.as_str()
            ))
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
        assert!(
            cap.require(Capability::VideoGenerate, &Provider::MiniMax)
                .is_ok()
        );
    }

    #[test]
    fn test_require_unsupported() {
        let cap = ProviderCapabilities::for_provider(&Provider::StepFun);
        let result = cap.require(Capability::VideoGenerate, &Provider::StepFun);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("StepFun does not support video_generate")
        );
    }
}
