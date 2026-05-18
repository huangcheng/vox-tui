use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget},
};

use crate::ui::AppTheme;

static SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner<'a> {
    frame: u32,
    label: Option<String>,
    theme: Option<&'a AppTheme>,
}

impl<'a> Spinner<'a> {
    pub fn new() -> Self {
        Self {
            frame: 0,
            label: None,
            theme: None,
        }
    }

    pub fn frame(mut self, frame: u32) -> Self {
        self.frame = frame;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn theme(mut self, theme: &'a AppTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn current_char(&self) -> &str {
        let idx = (self.frame / 3) as usize % SPINNER_CHARS.len();
        SPINNER_CHARS[idx]
    }
}

impl Default for Spinner<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Spinner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if let Some(label) = &self.label {
            format!("{} {}", self.current_char(), label)
        } else {
            self.current_char().to_string()
        };

        let style = if let Some(theme) = self.theme {
            theme.style(theme.warning)
        } else {
            Style::default()
        };

        Paragraph::new(text)
            .style(style)
            .render(area, buf);
    }
}
