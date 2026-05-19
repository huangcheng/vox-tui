use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
};

use crate::config::{Config, ConfigField};
use crate::ui::AppTheme;

pub struct ConfigView<'a> {
    config: &'a Config,
    fields: Vec<ConfigField>,
    selected: usize,
    editing: bool,
    edit_buffer: &'a str,
    theme: &'a AppTheme,
}

impl<'a> ConfigView<'a> {
    pub fn new(config: &'a Config, theme: &'a AppTheme) -> Self {
        let fields = ConfigField::build_fields(config);
        Self {
            config,
            fields,
            selected: 0,
            editing: false,
            edit_buffer: "",
            theme,
        }
    }

    pub fn with_selected(mut self, selected: usize) -> Self {
        self.selected = selected.min(self.fields.len().saturating_sub(1));
        self
    }

    pub fn with_editing(mut self, editing: bool) -> Self {
        self.editing = editing;
        self
    }

    pub fn with_edit_buffer(mut self, buffer: &'a str) -> Self {
        self.edit_buffer = buffer;
        self
    }

    fn current_field(&self) -> ConfigField {
        self.fields[self.selected]
    }

    fn get_current_model(&self, provider: &crate::config::Provider, category: &str) -> String {
        match provider {
            crate::config::Provider::StepFun => self
                .config
                .stepfun
                .as_ref()
                .and_then(|sf| sf.models.get(category))
                .map(|s| s.to_string())
                .or_else(|| self.config.stepfun.as_ref().and_then(|sf| sf.model.clone()))
                .unwrap_or_default(),
            crate::config::Provider::MiniMax => self
                .config
                .minimax
                .as_ref()
                .and_then(|mm| mm.models.get(category))
                .map(|s| s.to_string())
                .or_else(|| self.config.minimax.as_ref().and_then(|mm| mm.model.clone()))
                .unwrap_or_default(),
        }
    }

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "(not set)".to_string()
        } else {
            let visible: String = key.chars().take(4).collect();
            format!("{visible}***")
        }
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let theme = self.theme;

        let items: Vec<ListItem> = self
            .fields
            .iter()
            .enumerate()
            .flat_map(|(idx, field)| {
                let is_selected = idx == self.selected;
                let mut result = Vec::new();

                // Section header before first field of each section
                if let Some(name) = field.section_name() {
                    result.push(ListItem::new(""));
                    result.push(
                        ListItem::new(format!(" {} ", name)).style(
                            theme
                                .style(theme.accent)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        ),
                    );
                }

                if *field == ConfigField::ActiveProvider && idx == 0 {
                    result.push(ListItem::new(""));
                    result.push(
                        ListItem::new(" Provider ").style(
                            theme
                                .style(theme.accent)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        ),
                    );
                }

                let content = self.render_field_content(field, is_selected);

                let style = if is_selected {
                    theme
                        .style(theme.accent)
                        .add_modifier(Modifier::BOLD)
                        .bg(theme.surface_highlight)
                } else {
                    theme.style(theme.text_primary)
                };

                result.push(ListItem::new(content).style(style));
                result
            })
            .collect();

        let block = Block::default()
            .title(" Configuration ")
            .borders(Borders::ALL)
            .border_style(theme.style(theme.border))
            .padding(Padding::horizontal(1));

        let list = List::new(items).block(block);

        f.render_widget(list, area);

        let help_text = if self.editing {
            "Enter: Save  Esc: Cancel"
        } else {
            "↑↓ Navigate  ←→: Cycle  Enter: Edit  q: Quit"
        };

        let help_para = Paragraph::new(help_text).style(theme.style(theme.text_muted));

        let help_area = ratatui::layout::Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );

        f.render_widget(help_para, help_area);

        if self.editing {
            self.render_popup(f, area);
        }
    }

    fn render_popup(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let theme = self.theme;
        let field = self.current_field();

        // Dim background
        let dim_block = Block::default().bg(theme.background);
        f.render_widget(
            dim_block.style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );

        let popup_area = Self::centered_popup_area(area);
        f.render_widget(Clear, popup_area);

        let title = match field {
            ConfigField::StepFunApiKey => "Edit StepFun API Key",
            ConfigField::MiniMaxApiKey => "Edit MiniMax API Key",
            _ => "Edit Field",
        };

        let edit_content = self.edit_buffer;
        let display_text = format!("{}|", edit_content);

        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(theme.style(theme.border_focused))
            .padding(Padding::horizontal(1));

        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let input_para = Paragraph::new(display_text).style(theme.style(theme.text_primary));
        let input_area = Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), 1);
        f.render_widget(input_para, input_area);

        let help = "Enter: Save  Esc: Cancel";
        let help_para = Paragraph::new(help).style(theme.style(theme.text_muted));
        let help_area = Rect::new(
            inner.x + 1,
            inner.bottom().saturating_sub(1),
            inner.width.saturating_sub(2),
            1,
        );
        f.render_widget(help_para, help_area);
    }

    fn centered_popup_area(area: Rect) -> Rect {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(7),
                Constraint::Percentage(30),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[1]);

        horizontal[1]
    }

    fn render_field_content(&self, field: &ConfigField, is_selected: bool) -> String {
        let prefix = if is_selected { "▎ " } else { "  " };

        match field {
            ConfigField::ActiveProvider => {
                let value = self.config.default_provider.to_string();
                let configured = self.config.configured_providers();
                let arrows = if configured.len() > 1 { " ◀ ▶" } else { "" };
                format!("{}{}{}", prefix, value, arrows)
            }
            ConfigField::StepFunApiKey => {
                let value = match self.config.stepfun.as_ref() {
                    Some(s) => Self::mask_api_key(&s.api_key),
                    None => "(not set)".to_string(),
                };
                format!("{}API Key: {}", prefix, value)
            }
            ConfigField::MiniMaxApiKey => {
                let value = match self.config.minimax.as_ref() {
                    Some(m) => Self::mask_api_key(&m.api_key),
                    None => "(not set)".to_string(),
                };
                format!("{}API Key: {}", prefix, value)
            }
            _ => {
                let provider = field.provider().unwrap();
                let category = field.category().unwrap();
                let current = self.get_current_model(&provider, category);
                let available_count =
                    crate::models::get_available_models(&provider, category).len();
                let arrows = if available_count > 1 { " ◀ ▶" } else { "" };
                format!("{}{}: {}{}", prefix, field.label(), current, arrows)
            }
        }
    }
}
