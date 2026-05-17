use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return "(not set)".to_string();
    }
    let visible = key.len().min(4);
    format!("{}***", &key[..visible])
}

const DEFAULT_CONFIG_TOML: &str = r#"
default_provider = "minimax"

[stepfun]
base_url = "https://api.stepfun.com"
timeout = 120

[stepfun.models.chat]
default = "step-1o-pro-20250506"
available = ["step-1o-pro-20250506", "step-1o-mini"]

[stepfun.models.image]
default = "step-1x-high"
available = ["step-1x-high", "step-1x-medium"]

[stepfun.models.speech]
default = "step-tts"
available = ["step-tts", "step-tts-mini"]

[stepfun.models.video]
default = "step-video"

[stepfun.models.music]
default = "step-music"

[minimax]
base_url = "https://api.minimax.chat"
timeout = 120

[minimax.models.chat]
default = "MiniMax-Text-01"
available = ["MiniMax-Text-01", "MiniMax-Text-01-Turbo"]

[minimax.models.image]
default = "image-01"

[minimax.models.speech]
default = "speech-01"
available = ["speech-01", "speech-02-turbo"]

[minimax.models.video]
default = "video-01"

[minimax.models.music]
default = "music-01"

[minimax.models.vision]
default = "vision-01"
"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    StepFun,
    MiniMax,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::StepFun => write!(f, "stepfun"),
            Provider::MiniMax => write!(f, "minimax"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryModels {
    pub default: Option<String>,
    pub available: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderModels {
    pub chat: Option<CategoryModels>,
    pub image: Option<CategoryModels>,
    pub speech: Option<CategoryModels>,
    pub video: Option<CategoryModels>,
    pub music: Option<CategoryModels>,
    pub vision: Option<CategoryModels>,
}

impl ProviderModels {
    pub fn get(&self, category: &str) -> Option<&CategoryModels> {
        match category {
            "chat" => self.chat.as_ref(),
            "image" => self.image.as_ref(),
            "speech" => self.speech.as_ref(),
            "video" => self.video.as_ref(),
            "music" => self.music.as_ref(),
            "vision" => self.vision.as_ref(),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, category: &str) -> Option<&mut CategoryModels> {
        match category {
            "chat" => self.chat.as_mut(),
            "image" => self.image.as_mut(),
            "speech" => self.speech.as_mut(),
            "video" => self.video.as_mut(),
            "music" => self.music.as_mut(),
            "vision" => self.vision.as_mut(),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StepFunConfig {
    #[serde(default)]
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub models: ProviderModels,
}

impl std::fmt::Debug for StepFunConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepFunConfig")
            .field("api_key", &mask_key(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("models", &self.models)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MiniMaxConfig {
    #[serde(default)]
    pub api_key: String,
    pub group_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub models: ProviderModels,
}

impl std::fmt::Debug for MiniMaxConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniMaxConfig")
            .field("api_key", &mask_key(&self.api_key))
            .field("group_id", &self.group_id)
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("models", &self.models)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(alias = "provider")]
    pub default_provider: Provider,
    pub stepfun: Option<StepFunConfig>,
    pub minimax: Option<MiniMaxConfig>,
    pub theme: Option<ThemeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub accent_color: Option<String>,
    pub dark_mode: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: Provider::MiniMax,
            stepfun: None,
            minimax: None,
            theme: None,
        }
    }
}

impl Config {
    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("APPDATA").map(PathBuf::from).map(|p| p.join("vox"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var_os("HOME").map(PathBuf::from).map(|p| p.join(".config").join("vox"))
        }
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    pub fn default_config() -> Self {
        toml::from_str(DEFAULT_CONFIG_TOML)
            .unwrap_or_default()
    }

    /// Load user config merged over defaults
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path().ok_or(ConfigError::NoConfigDir)?;

        if !path.exists() {
            return Ok(Self::default_config());
        }

        let user_content = fs::read_to_string(&path).map_err(ConfigError::Io)?;
        let user_config: Config = toml::from_str(&user_content).map_err(ConfigError::Parse)?;

        let mut config = Self::default_config();
        config.merge(user_config);

        config.validate()?;
        Ok(config)
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config: Config = toml::from_str(&content).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    /// Merge user config over self (user values take precedence)
    pub fn merge(&mut self, user: Config) {
        self.default_provider = user.default_provider;

        if let Some(sf) = user.stepfun {
            if let Some(ref mut self_sf) = self.stepfun {
                if !sf.api_key.is_empty() {
                    self_sf.api_key = sf.api_key;
                }
                if sf.base_url.is_some() {
                    self_sf.base_url = sf.base_url;
                }
                if sf.model.is_some() {
                    self_sf.model = sf.model;
                }
                self_sf.models = sf.models;
            } else {
                self.stepfun = Some(sf);
            }
        }

        if let Some(mm) = user.minimax {
            if let Some(ref mut self_mm) = self.minimax {
                if !mm.api_key.is_empty() {
                    self_mm.api_key = mm.api_key;
                }
                if mm.group_id.is_some() {
                    self_mm.group_id = mm.group_id;
                }
                if mm.base_url.is_some() {
                    self_mm.base_url = mm.base_url;
                }
                if mm.model.is_some() {
                    self_mm.model = mm.model;
                }
                self_mm.models = mm.models;
            } else {
                self.minimax = Some(mm);
            }
        }

        if let Some(theme) = user.theme {
            self.theme = Some(theme);
        }
    }

    /// Get the default model for a category from the current provider
    pub fn get_model_for(&self, category: &str) -> Option<String> {
        match self.default_provider {
            Provider::StepFun => self.stepfun.as_ref().and_then(|sf| {
                sf.models
                    .get(category)
                    .and_then(|c| c.default.clone())
                    .or(sf.model.clone())
            }),
            Provider::MiniMax => self.minimax.as_ref().and_then(|mm| {
                mm.models
                    .get(category)
                    .and_then(|c| c.default.clone())
                    .or(mm.model.clone())
            }),
        }
    }

    /// Get available models for a category from the current provider
    pub fn get_available_models(&self, category: &str) -> Vec<String> {
        match self.default_provider {
            Provider::StepFun => self
                .stepfun
                .as_ref()
                .and_then(|sf| sf.models.get(category))
                .map(|c| c.available.clone().unwrap_or_default())
                .unwrap_or_default(),
            Provider::MiniMax => self
                .minimax
                .as_ref()
                .and_then(|mm| mm.models.get(category))
                .map(|c| c.available.clone().unwrap_or_default())
                .unwrap_or_default(),
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path().ok_or(ConfigError::NoConfigDir)?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        let content = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        fs::write(path, content).map_err(ConfigError::Io)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        Ok(())
    }

    pub fn configured_providers(&self) -> Vec<Provider> {
        let mut providers = Vec::new();
        if self.stepfun.is_some() { providers.push(Provider::StepFun); }
        if self.minimax.is_some() { providers.push(Provider::MiniMax); }
        providers
    }

    pub fn has_provider(&self, provider: &Provider) -> bool {
        match provider {
            Provider::StepFun => self.stepfun.is_some(),
            Provider::MiniMax => self.minimax.is_some(),
        }
    }

    pub fn default_provider_name(&self) -> &'static str {
        match self.default_provider {
            Provider::StepFun => "stepfun",
            Provider::MiniMax => "minimax",
        }
    }

    pub fn get_stepfun_key(&self) -> Option<&str> {
        self.stepfun.as_ref().map(|s| s.api_key.as_str())
    }

    pub fn get_minimax_key(&self) -> Option<&str> {
        self.minimax.as_ref().map(|m| m.api_key.as_str())
    }

    pub fn get_stepfun_base_url(&self) -> Option<&str> {
        self.stepfun.as_ref().and_then(|s| s.base_url.as_deref())
    }

    pub fn get_minimax_base_url(&self) -> Option<&str> {
        self.minimax.as_ref().and_then(|m| m.base_url.as_deref())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NoConfigDir,
    Io(io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    MissingProviderConfig(&'static str),
    EmptyField(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoConfigDir => write!(f, "No config directory available"),
            ConfigError::Io(e) => write!(f, "I/O error: {}", e),
            ConfigError::Parse(e) => write!(f, "Parse error: {}", e),
            ConfigError::Serialize(e) => write!(f, "Serialize error: {}", e),
            ConfigError::MissingProviderConfig(p) => {
                write!(f, "Missing config for provider: {}", p)
            }
            ConfigError::EmptyField(field) => {
                write!(f, "Empty required field: {}", field)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "sk-test-key".to_string(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
        }
    }

    #[test]
    fn test_config_validation_valid() {
        let config = sample_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_lenient() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: None,
            minimax: None,
            theme: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_key_allowed() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: String::new(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_roundtrip() {
        let config = sample_config();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.default_provider, parsed.default_provider);
        assert_eq!(config.stepfun.as_ref().unwrap().api_key, parsed.stepfun.as_ref().unwrap().api_key);
    }

    #[test]
    fn test_config_configured_providers_both() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "sk-test-key".to_string(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: None,
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            theme: None,
        };
        let providers = config.configured_providers();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&Provider::StepFun));
        assert!(providers.contains(&Provider::MiniMax));
    }

    #[test]
    fn test_config_configured_providers_single() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "sk-test-key".to_string(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
        };
        let providers = config.configured_providers();
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&Provider::StepFun));
    }

    #[test]
    fn test_config_configured_providers_none() {
        let config = Config {
            default_provider: Provider::MiniMax,
            stepfun: None,
            minimax: None,
            theme: None,
        };
        let providers = config.configured_providers();
        assert!(providers.is_empty());
    }

    #[test]
    fn test_config_has_provider() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "sk-test-key".to_string(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: None,
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            theme: None,
        };
        assert!(config.has_provider(&Provider::StepFun));
        assert!(config.has_provider(&Provider::MiniMax));
    }

    #[test]
    fn test_config_default_provider_name() {
        let config_stepfun = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "sk-test-key".to_string(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
        };
        assert_eq!(config_stepfun.default_provider_name(), "stepfun");

        let config_minimax = Config {
            default_provider: Provider::MiniMax,
            stepfun: None,
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: None,
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            theme: None,
        };
        assert_eq!(config_minimax.default_provider_name(), "minimax");
    }

    #[test]
    fn test_config_backward_compat() {
        let toml_str = r#"
provider = "stepfun"

[stepfun]
api_key = "sk-backward-compat"
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.default_provider, Provider::StepFun);
        assert!(parsed.stepfun.is_some());
        assert_eq!(parsed.stepfun.as_ref().unwrap().api_key, "sk-backward-compat");
    }

    #[test]
    fn test_config_roundtrip_new_field() {
        let config = Config {
            default_provider: Provider::MiniMax,
            stepfun: Some(StepFunConfig {
                api_key: "sk-test-key".to_string(),
                base_url: Some("https://stepfun.example.com".to_string()),
                model: Some("stepfun-model".to_string()),
                models: ProviderModels::default(),
            }),
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: Some("group-123".to_string()),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            theme: Some(ThemeConfig {
                accent_color: Some("#ff0000".to_string()),
                dark_mode: Some(true),
            }),
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.default_provider, parsed.default_provider);
        assert_eq!(config.stepfun.as_ref().unwrap().api_key, parsed.stepfun.as_ref().unwrap().api_key);
        assert_eq!(config.minimax.as_ref().unwrap().api_key, parsed.minimax.as_ref().unwrap().api_key);
        assert_eq!(config.theme.as_ref().unwrap().accent_color, parsed.theme.as_ref().unwrap().accent_color);
    }

    #[test]
    fn test_default_config_loads() {
        let config = Config::default_config();
        assert_eq!(config.default_provider, Provider::MiniMax);
        assert!(config.minimax.is_some());
        let mm = config.minimax.as_ref().unwrap();
        assert!(mm.models.chat.is_some());
    }

    #[test]
    fn test_merge_user_over_default() {
        let mut config = Config::default_config();
        let user = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: "sk-real-key".to_string(),
                base_url: None,
                model: None,
                models: ProviderModels::default(),
            }),
            minimax: None,
            theme: None,
        };
        config.merge(user);
        assert_eq!(config.default_provider, Provider::StepFun);
        assert_eq!(config.stepfun.as_ref().unwrap().api_key, "sk-real-key");
    }

    #[test]
    fn test_get_model_for_category() {
        let config = Config::default_config();
        let chat_model = config.get_model_for("chat");
        assert!(chat_model.is_some());
    }

    #[test]
    fn test_get_available_models() {
        let config = Config::default_config();
        let models = config.get_available_models("chat");
        assert!(!models.is_empty());
    }

    #[test]
    fn test_stepfun_debug_masks_api_key() {
        let cfg = StepFunConfig {
            api_key: "sk-secret-key-12345".to_string(),
            base_url: Some("https://api.stepfun.com".into()),
            model: None,
            models: ProviderModels::default(),
        };
        let debug = format!("{:?}", cfg);
        assert!(debug.contains("sk-s***"), "should mask api_key, got: {}", debug);
        assert!(!debug.contains("secret-key-12345"), "should NOT contain full key, got: {}", debug);
        assert!(debug.contains("stepfun.com"), "should contain base_url");
    }

    #[test]
    fn test_minimax_debug_masks_api_key() {
        let cfg = MiniMaxConfig {
            api_key: "mm-super-secret".to_string(),
            group_id: None,
            base_url: None,
            model: None,
            models: ProviderModels::default(),
        };
        let debug = format!("{:?}", cfg);
        assert!(debug.contains("mm-s***"), "should mask api_key, got: {}", debug);
        assert!(!debug.contains("super-secret"), "should NOT contain full key, got: {}", debug);
    }

    #[test]
    fn test_debug_empty_key_shows_not_set() {
        let cfg = StepFunConfig {
            api_key: String::new(),
            base_url: None,
            model: None,
            models: ProviderModels::default(),
        };
        let debug = format!("{:?}", cfg);
        assert!(debug.contains("(not set)"), "empty key should show '(not set)', got: {}", debug);
    }

    #[test]
    fn test_mask_key_utility() {
        assert_eq!(mask_key(""), "(not set)");
        assert_eq!(mask_key("ab"), "ab***");
        assert_eq!(mask_key("abcdefgh"), "abcd***");
    }
}
