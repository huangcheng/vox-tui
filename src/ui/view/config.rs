use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::config::{Config, Provider as ConfigProvider, StepFunConfig, MiniMaxConfig};
use crate::ui::AppTheme;

/// Represents the fields users can navigate and edit in the config view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    ActiveProvider,
    StepFunApiKey,
    MiniMaxApiKey,
    // Model selectors — only shown when the provider has models for this category
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
    pub fn provider(&self) -> Option<crate::config::Provider> {
        match self {
            ConfigField::StepFunApiKey
            | ConfigField::StepFunModelChat
            | ConfigField::StepFunModelImage
            | ConfigField::StepFunModelSpeech
            | ConfigField::StepFunModelVideo
            | ConfigField::StepFunModelMusic
            | ConfigField::StepFunModelVision => Some(crate::config::Provider::StepFun),
            ConfigField::MiniMaxApiKey
            | ConfigField::MiniMaxModelChat
            | ConfigField::MiniMaxModelImage
            | ConfigField::MiniMaxModelSpeech
            | ConfigField::MiniMaxModelVideo
            | ConfigField::MiniMaxModelMusic
            | ConfigField::MiniMaxModelVision => Some(crate::config::Provider::MiniMax),
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

    /// Is this a section header point? (field before which we insert a provider header)
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

pub struct ConfigView<'a> {
    config: &'a mut Config,
    fields: Vec<ConfigField>,
    selected: usize,
    editing: bool,
    edit_buffer: String,
    theme: &'a AppTheme,
}

impl<'a> ConfigView<'a> {
    pub fn new(config: &'a mut Config, theme: &'a AppTheme) -> Self {
        let fields = ConfigField::build_fields(config);
        Self {
            config,
            fields,
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            theme,
        }
    }

    pub fn with_selected(mut self, selected: usize) -> Self {
        self.selected = selected.min(self.fields.len().saturating_sub(1));
        self
    }

    fn current_field(&self) -> ConfigField {
        self.fields[self.selected]
    }

    fn start_edit(&mut self) {
        let field = self.current_field();
        if field.is_model() {
            return;
        }
        self.editing = true;
        self.edit_buffer = self.get_current_value();
    }

    fn get_current_value(&self) -> String {
        match self.current_field() {
            ConfigField::ActiveProvider => self.config.default_provider.to_string(),
            ConfigField::StepFunApiKey => {
                self.config.stepfun.as_ref().map(|s| s.api_key.clone()).unwrap_or_default()
            }
            ConfigField::MiniMaxApiKey => {
                self.config.minimax.as_ref().map(|m| m.api_key.clone()).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    fn apply_edit(&mut self) {
        match self.current_field() {
            ConfigField::ActiveProvider => {
                match self.edit_buffer.to_lowercase().as_str() {
                    "stepfun" => self.config.default_provider = ConfigProvider::StepFun,
                    "minimax" => self.config.default_provider = ConfigProvider::MiniMax,
                    _ => {}
                }
            }
            ConfigField::StepFunApiKey => {
                if self.edit_buffer.is_empty() {
                    self.config.stepfun = None;
                } else {
                    let cfg = self.config.stepfun.get_or_insert_with(|| StepFunConfig {
                        api_key: String::new(),
                        base_url: None,
                        model: None,
                        models: crate::config::ProviderModels::default(),
                    });
                    cfg.api_key = self.edit_buffer.clone();
                }
            }
            ConfigField::MiniMaxApiKey => {
                if self.edit_buffer.is_empty() {
                    self.config.minimax = None;
                } else {
                    let cfg = self.config.minimax.get_or_insert_with(|| MiniMaxConfig {
                        api_key: String::new(),
                        group_id: None,
                        base_url: None,
                        model: None,
                        models: crate::config::ProviderModels::default(),
                    });
                    cfg.api_key = self.edit_buffer.clone();
                }
            }
            _ => {}
        }
    }

    fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;

        if self.editing {
            match key.code {
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                    true
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    true
                }
                KeyCode::Enter => {
                    self.apply_edit();
                    self.editing = false;
                    true
                }
                KeyCode::Esc => {
                    self.cancel_edit();
                    true
                }
                _ => false,
            }
        } else {
            let field = self.current_field();
            match key.code {
                KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    true
                }
                KeyCode::Down => {
                    self.selected = (self.selected + 1).min(self.fields.len() - 1);
                    true
                }
                KeyCode::Left if field.is_model() => {
                    self.cycle_model(-1);
                    true
                }
                KeyCode::Right if field.is_model() => {
                    self.cycle_model(1);
                    true
                }
                KeyCode::Enter => {
                    self.start_edit();
                    true
                }
                _ => false,
            }
        }
    }

    fn cycle_model(&mut self, direction: i32) {
        let field = self.current_field();
        let category = match field.category() {
            Some(c) => c,
            None => return,
        };
        let provider = match field.provider() {
            Some(p) => p,
            None => return,
        };

        let models = match provider {
            crate::config::Provider::StepFun => {
                self.config.stepfun.as_ref()
                    .and_then(|sf| sf.models.get(category))
                    .and_then(|cm| cm.available.as_ref())
                    .cloned()
                    .unwrap_or_default()
            }
            crate::config::Provider::MiniMax => {
                self.config.minimax.as_ref()
                    .and_then(|mm| mm.models.get(category))
                    .and_then(|cm| cm.available.as_ref())
                    .cloned()
                    .unwrap_or_default()
            }
        };

        if models.is_empty() {
            return;
        }

        let current = self.get_current_model(&provider, category);
        let current_idx = models.iter().position(|m| m == &current).unwrap_or(0);

        let new_idx = if direction > 0 {
            (current_idx + 1) % models.len()
        } else {
            current_idx.saturating_sub(1).max(models.len() - 1)
        };
        let new_model = models[new_idx].clone();

        self.set_model(&provider, category, new_model);
    }

    fn get_current_model(&self, provider: &crate::config::Provider, category: &str) -> String {
        match provider {
            crate::config::Provider::StepFun => {
                self.config.stepfun.as_ref()
                    .and_then(|sf| sf.models.get(category))
                    .and_then(|cm| cm.default.clone())
                    .or_else(|| self.config.stepfun.as_ref().and_then(|sf| sf.model.clone()))
                    .unwrap_or_default()
            }
            crate::config::Provider::MiniMax => {
                self.config.minimax.as_ref()
                    .and_then(|mm| mm.models.get(category))
                    .and_then(|cm| cm.default.clone())
                    .or_else(|| self.config.minimax.as_ref().and_then(|mm| mm.model.clone()))
                    .unwrap_or_default()
            }
        }
    }

    fn set_model(&mut self, provider: &crate::config::Provider, category: &str, model: String) {
        let config = &mut self.config;
        match provider {
            crate::config::Provider::StepFun => {
                if let Some(ref mut sf) = config.stepfun {
                    let category_model = match category {
                        "chat" => sf.models.chat.as_mut(),
                        "image" => sf.models.image.as_mut(),
                        "speech" => sf.models.speech.as_mut(),
                        "video" => sf.models.video.as_mut(),
                        "music" => sf.models.music.as_mut(),
                        "vision" => sf.models.vision.as_mut(),
                        _ => None,
                    };
                    if let Some(cm) = category_model {
                        cm.default = Some(model.clone());
                    }
                    if category == "chat" {
                        sf.model = Some(model);
                    }
                }
            }
            crate::config::Provider::MiniMax => {
                if let Some(ref mut mm) = config.minimax {
                    let category_model = match category {
                        "chat" => mm.models.chat.as_mut(),
                        "image" => mm.models.image.as_mut(),
                        "speech" => mm.models.speech.as_mut(),
                        "video" => mm.models.video.as_mut(),
                        "music" => mm.models.music.as_mut(),
                        "vision" => mm.models.vision.as_mut(),
                        _ => None,
                    };
                    if let Some(cm) = category_model {
                        cm.default = Some(model.clone());
                    }
                    if category == "chat" {
                        mm.model = Some(model);
                    }
                }
            }
        }
    }

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "(not set)".to_string()
        } else if key.len() <= 4 {
            format!("{}***", key)
        } else {
            format!("{}***", &key[..4])
        }
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let theme = self.theme;

        let items: Vec<ListItem> = self.fields.iter().enumerate().flat_map(|(idx, field)| {
            let is_selected = idx == self.selected;

            let mut result = Vec::new();

            if let Some(name) = field.section_name() {
                result.push(ListItem::new(format!("── {} ──", name))
                    .style(Style::default().fg(Color::DarkGray)));
            }

            let is_editing = is_selected && self.editing;
            let content = self.render_field_content(field, is_selected, is_editing);

            let style = if is_editing {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::REVERSED)
            } else if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            result.push(ListItem::new(content).style(style));
            result
        }).collect();

        let block = Block::default()
            .title(" Configuration ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let list = List::new(items)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);

        let help_text = if self.editing {
            "↑↓ Navigate  Enter: Confirm  Esc: Cancel  Type to edit"
        } else {
            "↑↓ Navigate  ←→: Cycle models  Enter: Edit  q: Quit"
        };

        let help_para = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray));

        let help_area = ratatui::layout::Rect::new(
            area.x + 1,
            area.bottom() - 1,
            area.width.saturating_sub(2),
            1,
        );

        f.render_widget(help_para, help_area);
    }

    fn render_field_content(&self, field: &ConfigField, is_selected: bool, is_editing: bool) -> String {
        let prefix = if is_selected { "► " } else { "  " };

        match field {
            ConfigField::ActiveProvider => {
                let value = self.config.default_provider.to_string();
                if is_editing {
                    format!("{}Active Provider: {}{}", prefix, self.edit_buffer, "█")
                } else {
                    format!("{}Active Provider: {}", prefix, value)
                }
            }
            ConfigField::StepFunApiKey => {
                let value = match self.config.stepfun.as_ref() {
                    Some(s) => Self::mask_api_key(&s.api_key),
                    None => "(not set)".to_string(),
                };
                if is_editing {
                    format!("{}API Key: {}{}", prefix, self.edit_buffer, "█")
                } else {
                    format!("{}API Key: {}", prefix, value)
                }
            }
            ConfigField::MiniMaxApiKey => {
                let value = match self.config.minimax.as_ref() {
                    Some(m) => Self::mask_api_key(&m.api_key),
                    None => "(not set)".to_string(),
                };
                if is_editing {
                    format!("{}API Key: {}{}", prefix, self.edit_buffer, "█")
                } else {
                    format!("{}API Key: {}", prefix, value)
                }
            }
            _ => {
                let provider = field.provider().unwrap();
                let category = field.category().unwrap();
                let current = self.get_current_model(&provider, category);
                let available_count = match &provider {
                    crate::config::Provider::StepFun => {
                        self.config.stepfun.as_ref()
                            .and_then(|sf| sf.models.get(category))
                            .and_then(|cm| cm.available.as_ref())
                            .map(|v| v.len())
                            .unwrap_or(0)
                    }
                    crate::config::Provider::MiniMax => {
                        self.config.minimax.as_ref()
                            .and_then(|mm| mm.models.get(category))
                            .and_then(|cm| cm.available.as_ref())
                            .map(|v| v.len())
                            .unwrap_or(0)
                    }
                };
                let arrows = if available_count > 1 { " ← →" } else { "" };
                format!("{}{}: {}{}", prefix, field.label(), current, arrows)
            }
        }
    }
}
