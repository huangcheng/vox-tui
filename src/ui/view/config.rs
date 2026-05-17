use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::config::Config;
use crate::ui::AppTheme;

pub struct ConfigView<'a> {
    config: &'a Config,
    selected_section: usize,
    theme: &'a AppTheme,
}

impl<'a> ConfigView<'a> {
    pub fn new(config: &'a Config, theme: &'a AppTheme) -> Self {
        Self {
            config,
            selected_section: 0,
            theme,
        }
    }

    pub fn selected_section(mut self, idx: usize) -> Self {
        self.selected_section = idx;
        self
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let sections: Vec<ListItem> = vec![
            ListItem::new(format!("Provider: {}", self.config.default_provider)),
            ListItem::new(""),
            ListItem::new(match &self.config.stepfun {
                Some(s) => format!("StepFun API Key: {}*** (set)", &s.api_key[..4.min(s.api_key.len())]),
                None => "StepFun: Not configured".to_string(),
            }),
            ListItem::new(match &self.config.stepfun {
                Some(s) => format!("Model: {}", s.model.as_deref().unwrap_or("default")),
                None => String::new(),
            }),
            ListItem::new(""),
            ListItem::new(match &self.config.minimax {
                Some(m) => format!("MiniMax API Key: {}*** (set)", &m.api_key[..4.min(m.api_key.len())]),
                None => "MiniMax: Not configured".to_string(),
            }),
            ListItem::new(match &self.config.minimax {
                Some(m) => format!("Model: {}", m.model.as_deref().unwrap_or("default")),
                None => String::new(),
            }),
        ];

        let config_list = List::new(sections)
            .block(
                Block::default()
                    .title(" Configuration ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.accent)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(config_list, area);
    }
}
