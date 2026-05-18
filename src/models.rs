use crate::config::Provider;

/// Known models per provider per capability.
/// This lives in CODE, not config — updated with each CLI release.
pub struct KnownModels {
    pub models: &'static [&'static str],
    pub default: &'static str,
}

impl KnownModels {
    /// Check if a model name is in the known list.
    pub fn is_known(&self, model: &str) -> bool {
        self.models.contains(&model)
    }
}

pub fn get_known_models(provider: &Provider, capability: &str) -> Option<KnownModels> {
    match provider {
        Provider::MiniMax => minimax_models(capability),
        Provider::StepFun => stepfun_models(capability),
    }
}

/// Get available model names as a Vec<String> for a provider/capability.
/// Used by TUI for cycling through options.
pub fn get_available_models(provider: &Provider, capability: &str) -> Vec<String> {
    get_known_models(provider, capability)
        .map(|km| km.models.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

fn minimax_models(capability: &str) -> Option<KnownModels> {
    match capability {
        "chat" => Some(KnownModels {
            models: &["MiniMax-M2.7", "MiniMax-M2.5", "MiniMax-M2.1", "MiniMax-M2"],
            default: "MiniMax-M2.7",
        }),
        "image" => Some(KnownModels {
            models: &["image-01", "image-01-live"],
            default: "image-01",
        }),
        "speech" => Some(KnownModels {
            models: &["speech-01", "speech-02-turbo", "speech-2.6-hd", "speech-2.8-hd"],
            default: "speech-01",
        }),
        "video" => Some(KnownModels {
            models: &["MiniMax-Hailuo-2.3", "MiniMax-Hailuo-02"],
            default: "MiniMax-Hailuo-2.3",
        }),
        "music" => Some(KnownModels {
            models: &["music-2.6"],
            default: "music-2.6",
        }),
        "vision" => Some(KnownModels {
            models: &["vision-01"],
            default: "vision-01",
        }),
        _ => None,
    }
}

fn stepfun_models(capability: &str) -> Option<KnownModels> {
    match capability {
        "chat" => Some(KnownModels {
            models: &[
                "step-1-8k", "step-1-32k", "step-1-128k", "step-1-flash",
                "step-2-16k", "step-2-32k", "step-3.5-flash",
            ],
            default: "step-1-8k",
        }),
        "image" => Some(KnownModels {
            models: &["step-image-edit-2", "step-2x-large", "step-1x-medium"],
            default: "step-image-edit-2",
        }),
        "speech" => Some(KnownModels {
            models: &["step-tts-2", "step-tts-mini", "stepaudio-2.5-tts"],
            default: "step-tts-2",
        }),
        "vision" => Some(KnownModels {
            models: &["step-1v-8k"],
            default: "step-1v-8k",
        }),
        "search" => Some(KnownModels {
            models: &["step-search"],
            default: "step-search",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimax_chat_models() {
        let km = get_known_models(&Provider::MiniMax, "chat").unwrap();
        assert!(km.is_known("MiniMax-M2.7"));
        assert!(!km.is_known("unknown-model"));
    }

    #[test]
    fn test_stepfun_no_video() {
        assert!(get_known_models(&Provider::StepFun, "video").is_none());
    }

    #[test]
    fn test_unknown_capability() {
        assert!(get_known_models(&Provider::MiniMax, "telepathy").is_none());
    }
}
