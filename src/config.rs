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

// ═══════════════════════════════════════════════════════════════════
// ConfigField — describes navigable/editable config fields (moved from UI)
// ═══════════════════════════════════════════════════════════════════

/// Represents the fields users can navigate and edit in the config view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    ActiveProvider,
    StepFunApiKey,
    MiniMaxApiKey,
    // Model selectors
    StepFunModelChat,
    StepFunModelImage,
    StepFunModelSpeech,
    StepFunModelVideo,
    StepFunModelMusic,
    StepFunModelVision,
    MiniMaxModelChat,
    MiniMaxModelImage,
    MiniMaxModelSpeech,
    MiniMaxModelVideo,
    MiniMaxModelMusic,
    MiniMaxModelVision,
}

impl ConfigField {
    /// Build the field list based on what's available in config
    pub fn build_fields(config: &Config) -> Vec<ConfigField> {
        let mut fields = vec![ConfigField::ActiveProvider];

        // StepFun section
        fields.push(ConfigField::StepFunApiKey);
        if let Some(ref sf) = config.stepfun {
            for (cat, field) in Self::stepfun_model_fields() {
                if sf.models.get(cat).is_some() {
                    fields.push(field);
                }
            }
        }

        // MiniMax section
        fields.push(ConfigField::MiniMaxApiKey);
        if let Some(ref mm) = config.minimax {
            for (cat, field) in Self::minimax_model_fields() {
                if mm.models.get(cat).is_some() {
                    fields.push(field);
                }
            }
        }

        fields
    }

    fn stepfun_model_fields() -> Vec<(&'static str, ConfigField)> {
        vec![
            ("chat", ConfigField::StepFunModelChat),
            ("image", ConfigField::StepFunModelImage),
            ("speech", ConfigField::StepFunModelSpeech),
            ("video", ConfigField::StepFunModelVideo),
            ("music", ConfigField::StepFunModelMusic),
            ("vision", ConfigField::StepFunModelVision),
        ]
    }

    fn minimax_model_fields() -> Vec<(&'static str, ConfigField)> {
        vec![
            ("chat", ConfigField::MiniMaxModelChat),
            ("image", ConfigField::MiniMaxModelImage),
            ("speech", ConfigField::MiniMaxModelSpeech),
            ("video", ConfigField::MiniMaxModelVideo),
            ("music", ConfigField::MiniMaxModelMusic),
            ("vision", ConfigField::MiniMaxModelVision),
        ]
    }

    /// Get the category string for model fields
    pub fn category(&self) -> Option<&'static str> {
        match self {
            ConfigField::StepFunModelChat | ConfigField::MiniMaxModelChat => Some("chat"),
            ConfigField::StepFunModelImage | ConfigField::MiniMaxModelImage => Some("image"),
            ConfigField::StepFunModelSpeech | ConfigField::MiniMaxModelSpeech => Some("speech"),
            ConfigField::StepFunModelVideo | ConfigField::MiniMaxModelVideo => Some("video"),
            ConfigField::StepFunModelMusic | ConfigField::MiniMaxModelMusic => Some("music"),
            ConfigField::StepFunModelVision | ConfigField::MiniMaxModelVision => Some("vision"),
            _ => None,
        }
    }

    /// Is this a model selection field?
    pub fn is_model(&self) -> bool {
        self.category().is_some()
    }

    /// Which provider does this field belong to?
    pub fn provider(&self) -> Option<Provider> {
        match self {
            ConfigField::StepFunApiKey
            | ConfigField::StepFunModelChat
            | ConfigField::StepFunModelImage
            | ConfigField::StepFunModelSpeech
            | ConfigField::StepFunModelVideo
            | ConfigField::StepFunModelMusic
            | ConfigField::StepFunModelVision => Some(Provider::StepFun),
            ConfigField::MiniMaxApiKey
            | ConfigField::MiniMaxModelChat
            | ConfigField::MiniMaxModelImage
            | ConfigField::MiniMaxModelSpeech
            | ConfigField::MiniMaxModelVideo
            | ConfigField::MiniMaxModelMusic
            | ConfigField::MiniMaxModelVision => Some(Provider::MiniMax),
            _ => None,
        }
    }

    /// Get the display label
    pub fn label(&self) -> &'static str {
        match self {
            ConfigField::ActiveProvider => "Active Provider",
            ConfigField::StepFunApiKey => "API Key",
            ConfigField::MiniMaxApiKey => "API Key",
            ConfigField::StepFunModelChat | ConfigField::MiniMaxModelChat => "Chat Model",
            ConfigField::StepFunModelImage | ConfigField::MiniMaxModelImage => "Image Model",
            ConfigField::StepFunModelSpeech | ConfigField::MiniMaxModelSpeech => "Speech Model",
            ConfigField::StepFunModelVideo | ConfigField::MiniMaxModelVideo => "Video Model",
            ConfigField::StepFunModelMusic | ConfigField::MiniMaxModelMusic => "Music Model",
            ConfigField::StepFunModelVision | ConfigField::MiniMaxModelVision => "Vision Model",
        }
    }

    /// Is this a section header point?
    pub fn is_section_start(&self) -> bool {
        matches!(self, ConfigField::StepFunApiKey | ConfigField::MiniMaxApiKey)
    }

    pub fn section_name(&self) -> Option<&'static str> {
        match self {
            ConfigField::StepFunApiKey => Some("StepFun"),
            ConfigField::MiniMaxApiKey => Some("MiniMax"),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// ConfigEditor — extracted from AppState for config navigation/editing
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct ConfigEditor {
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
}

impl ConfigEditor {
    pub fn new() -> Self {
        Self {
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
        }
    }

    pub fn fields(&self, config: &Config) -> Vec<ConfigField> {
        ConfigField::build_fields(config)
    }

    pub fn start_edit(&mut self, config: &Config) {
        self.editing = true;
        let fields = self.fields(config);
        let field = fields.get(self.selected).copied().unwrap_or(ConfigField::ActiveProvider);
        self.edit_buffer = self.get_field_value(config, field);
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn apply_edit(&mut self, config: &mut Config) -> Result<(), String> {
        let fields = ConfigField::build_fields(config);
        let field = fields.get(self.selected).copied().unwrap_or(ConfigField::ActiveProvider);

        match field {
            ConfigField::ActiveProvider => {
                match self.edit_buffer.to_lowercase().as_str() {
                    "stepfun" => config.default_provider = Provider::StepFun,
                    "minimax" => config.default_provider = Provider::MiniMax,
                    _ => {}
                }
            }
            ConfigField::StepFunApiKey => {
                if self.edit_buffer.is_empty() {
                    config.stepfun = None;
                } else {
                    let cfg = config.stepfun.get_or_insert_with(|| StepFunConfig {
                        api_key: String::new(),
                        base_url: None,
                        model: None,
                        models: ProviderModels::default(),
                    });
                    cfg.api_key = self.edit_buffer.clone();
                }
            }
            ConfigField::MiniMaxApiKey => {
                if self.edit_buffer.is_empty() {
                    config.minimax = None;
                } else {
                    let cfg = config.minimax.get_or_insert_with(|| MiniMaxConfig {
                        api_key: String::new(),
                        group_id: None,
                        base_url: None,
                        model: None,
                        models: ProviderModels::default(),
                    });
                    cfg.api_key = self.edit_buffer.clone();
                }
            }
            _ => {
                // Model fields
                if !self.edit_buffer.is_empty() {
                    self.apply_model_edit(config, field);
                }
            }
        }

        self.editing = false;

        // Clamp selected to valid range
        let new_fields = ConfigField::build_fields(config);
        if self.selected >= new_fields.len() {
            self.selected = new_fields.len().saturating_sub(1);
        }

        config.save().map_err(|e| format!("Failed to save config: {}", e))
    }

    fn apply_model_edit(&self, config: &mut Config, field: ConfigField) {
        let (provider_field, category) = match field {
            ConfigField::StepFunModelChat => ("stepfun", "chat"),
            ConfigField::StepFunModelImage => ("stepfun", "image"),
            ConfigField::StepFunModelSpeech => ("stepfun", "speech"),
            ConfigField::StepFunModelVideo => ("stepfun", "video"),
            ConfigField::StepFunModelMusic => ("stepfun", "music"),
            ConfigField::StepFunModelVision => ("stepfun", "vision"),
            ConfigField::MiniMaxModelChat => ("minimax", "chat"),
            ConfigField::MiniMaxModelImage => ("minimax", "image"),
            ConfigField::MiniMaxModelSpeech => ("minimax", "speech"),
            ConfigField::MiniMaxModelVideo => ("minimax", "video"),
            ConfigField::MiniMaxModelMusic => ("minimax", "music"),
            ConfigField::MiniMaxModelVision => ("minimax", "vision"),
            _ => return,
        };

        let models = match provider_field {
            "stepfun" => config.stepfun.as_mut().map(|s| &mut s.models),
            "minimax" => config.minimax.as_mut().map(|m| &mut m.models),
            _ => None,
        };
        if let Some(models) = models {
            if let Some(cat_models) = models.get_mut(category) {
                cat_models.default = Some(self.edit_buffer.clone());
            }
        }
    }

    pub fn navigate_up(&mut self, _config: &Config) {
        if !self.editing {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn navigate_down(&mut self, config: &Config) {
        if !self.editing {
            let max = self.fields(config).len().saturating_sub(1);
            self.selected = (self.selected + 1).min(max);
        }
    }

    pub fn cycle_model(&mut self, config: &mut Config, direction: i32) {
        let fields = ConfigField::build_fields(config);
        let field = match fields.get(self.selected).copied() {
            Some(f) => f,
            None => return,
        };

        let category = match field.category() {
            Some(c) => c,
            None => return,
        };
        let provider = match field.provider() {
            Some(p) => p,
            None => return,
        };

        let models: Vec<String> = match provider {
            Provider::StepFun => {
                config.stepfun.as_ref()
                    .and_then(|sf| sf.models.get(category))
                    .and_then(|cm| cm.available.as_ref())
                    .cloned()
                    .unwrap_or_default()
            }
            Provider::MiniMax => {
                config.minimax.as_ref()
                    .and_then(|mm| mm.models.get(category))
                    .and_then(|cm| cm.available.as_ref())
                    .cloned()
                    .unwrap_or_default()
            }
        };

        if models.is_empty() {
            return;
        }

        let current = match provider {
            Provider::StepFun => {
                config.stepfun.as_ref()
                    .and_then(|sf| sf.models.get(category))
                    .and_then(|cm| cm.default.clone())
                    .or_else(|| config.stepfun.as_ref().and_then(|sf| sf.model.clone()))
                    .unwrap_or_default()
            }
            Provider::MiniMax => {
                config.minimax.as_ref()
                    .and_then(|mm| mm.models.get(category))
                    .and_then(|cm| cm.default.clone())
                    .or_else(|| config.minimax.as_ref().and_then(|mm| mm.model.clone()))
                    .unwrap_or_default()
            }
        };

        let current_idx = models.iter().position(|m| m == &current).unwrap_or(0);
        let new_idx = if direction > 0 {
            (current_idx + 1) % models.len()
        } else {
            current_idx.saturating_sub(1).max(models.len() - 1)
        };
        let new_model = models[new_idx].clone();

        match provider {
            Provider::StepFun => {
                if let Some(ref mut sf) = config.stepfun {
                    if let Some(cm) = sf.models.get_mut(category) {
                        cm.default = Some(new_model.clone());
                    }
                    if category == "chat" {
                        sf.model = Some(new_model);
                    }
                }
            }
            Provider::MiniMax => {
                if let Some(ref mut mm) = config.minimax {
                    if let Some(cm) = mm.models.get_mut(category) {
                        cm.default = Some(new_model.clone());
                    }
                    if category == "chat" {
                        mm.model = Some(new_model);
                    }
                }
            }
        }
    }

    pub fn type_char(&mut self, c: char) {
        if self.editing {
            self.edit_buffer.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.editing {
            self.edit_buffer.pop();
        }
    }

    fn get_field_value(&self, config: &Config, field: ConfigField) -> String {
        match field {
            ConfigField::ActiveProvider => config.default_provider.to_string(),
            ConfigField::StepFunApiKey => {
                config.stepfun.as_ref().map(|s| s.api_key.clone()).unwrap_or_default()
            }
            ConfigField::MiniMaxApiKey => {
                config.minimax.as_ref().map(|m| m.api_key.clone()).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    pub fn get_current_model(&self, config: &Config, field: ConfigField) -> String {
        let provider = match field.provider() {
            Some(p) => p,
            None => return String::new(),
        };
        let category = match field.category() {
            Some(c) => c,
            None => return String::new(),
        };
        match provider {
            Provider::StepFun => {
                config.stepfun.as_ref()
                    .and_then(|sf| sf.models.get(category))
                    .and_then(|cm| cm.default.clone())
                    .or_else(|| config.stepfun.as_ref().and_then(|sf| sf.model.clone()))
                    .unwrap_or_default()
            }
            Provider::MiniMax => {
                config.minimax.as_ref()
                    .and_then(|mm| mm.models.get(category))
                    .and_then(|cm| cm.default.clone())
                    .or_else(|| config.minimax.as_ref().and_then(|mm| mm.model.clone()))
                    .unwrap_or_default()
            }
        }
    }

    pub fn get_available_count(&self, config: &Config, field: ConfigField) -> usize {
        let provider = match field.provider() {
            Some(p) => p,
            None => return 0,
        };
        let category = match field.category() {
            Some(c) => c,
            None => return 0,
        };
        match provider {
            Provider::StepFun => {
                config.stepfun.as_ref()
                    .and_then(|sf| sf.models.get(category))
                    .and_then(|cm| cm.available.as_ref())
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
            Provider::MiniMax => {
                config.minimax.as_ref()
                    .and_then(|mm| mm.models.get(category))
                    .and_then(|cm| cm.available.as_ref())
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
        }
    }

    pub fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "(not set)".to_string()
        } else if key.len() <= 4 {
            format!("{}***", key)
        } else {
            format!("{}***", &key[..4])
        }
    }
}

impl Default for ConfigEditor {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Error type
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

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
        assert_eq!(config.default_provider_name(), "stepfun");
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

    // ConfigEditor tests

    #[test]
    fn test_config_editor_new() {
        let editor = ConfigEditor::new();
        assert_eq!(editor.selected, 0);
        assert!(!editor.editing);
        assert!(editor.edit_buffer.is_empty());
    }

    #[test]
    fn test_config_editor_start_edit() {
        let mut editor = ConfigEditor::new();
        let config = sample_config();
        editor.start_edit(&config);
        assert!(editor.editing);
        assert_eq!(editor.edit_buffer, "stepfun");
    }

    #[test]
    fn test_config_editor_cancel_edit() {
        let mut editor = ConfigEditor::new();
        let config = sample_config();
        editor.start_edit(&config);
        editor.edit_buffer.push('x');
        editor.cancel_edit();
        assert!(!editor.editing);
        assert!(editor.edit_buffer.is_empty());
    }

    #[test]
    fn test_config_editor_navigate() {
        let mut editor = ConfigEditor::new();
        let config = Config::default_config();
        assert_eq!(editor.selected, 0);
        editor.navigate_down(&config);
        assert_eq!(editor.selected, 1);
        editor.navigate_up(&config);
        assert_eq!(editor.selected, 0);
    }

    #[test]
    fn test_config_editor_type_and_backspace() {
        let mut editor = ConfigEditor::new();
        assert!(!editor.editing);
        editor.type_char('a'); // Should be no-op when not editing
        assert!(editor.edit_buffer.is_empty());
        editor.editing = true;
        editor.type_char('a');
        editor.type_char('b');
        assert_eq!(editor.edit_buffer, "ab");
        editor.backspace();
        assert_eq!(editor.edit_buffer, "a");
    }

    #[test]
    fn test_config_editor_mask_api_key() {
        assert_eq!(ConfigEditor::mask_api_key(""), "(not set)");
        assert_eq!(ConfigEditor::mask_api_key("abcd"), "abcd***");
        assert_eq!(ConfigEditor::mask_api_key("abcdefgh"), "abcd***");
    }

    #[test]
    fn test_config_field_build_fields() {
        let config = Config::default_config();
        let fields = ConfigField::build_fields(&config);
        assert!(fields.contains(&ConfigField::ActiveProvider));
        assert!(fields.contains(&ConfigField::MiniMaxApiKey));
    }
}
