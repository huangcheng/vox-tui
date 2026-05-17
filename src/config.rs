use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepFunConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniMaxConfig {
    pub api_key: String,
    pub group_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
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

    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path().ok_or(ConfigError::NoConfigDir)?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config: Config = toml::from_str(&content).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
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
        match &self.default_provider {
            Provider::StepFun => {
                let cfg = self.stepfun.as_ref().ok_or(ConfigError::MissingProviderConfig("stepfun"))?;
                if cfg.api_key.is_empty() {
                    return Err(ConfigError::EmptyField("stepfun.api_key"));
                }
            }
            Provider::MiniMax => {
                let cfg = self.minimax.as_ref().ok_or(ConfigError::MissingProviderConfig("minimax"))?;
                if cfg.api_key.is_empty() {
                    return Err(ConfigError::EmptyField("minimax.api_key"));
                }
            }
        }
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
    fn test_config_validation_missing_provider() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: None,
            minimax: None,
            theme: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_empty_key() {
        let config = Config {
            default_provider: Provider::StepFun,
            stepfun: Some(StepFunConfig {
                api_key: String::new(),
                base_url: None,
                model: None,
            }),
            minimax: None,
            theme: None,
        };
        assert!(config.validate().is_err());
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
            }),
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: None,
                base_url: None,
                model: None,
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
            }),
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: None,
                base_url: None,
                model: None,
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
            }),
            minimax: Some(MiniMaxConfig {
                api_key: "mm-test-key".to_string(),
                group_id: Some("group-123".to_string()),
                base_url: None,
                model: None,
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
}
