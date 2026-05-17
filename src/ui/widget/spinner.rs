use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

static SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    frame: u32,
    label: Option<String>,
}

impl Spinner {
    pub fn new() -> Self {
        Self { frame: 0, label: None }
    }

    pub fn frame(mut self, frame: u32) -> Self {
        self.frame = frame;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn current_char(&self) -> &str {
        let idx = (self.frame / 3) as usize % SPINNER_CHARS.len();
        SPINNER_CHARS[idx]
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Spinner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if let Some(label) = &self.label {
            format!("{} {}", self.current_char(), label)
        } else {
            self.current_char().to_string()
        };

        Paragraph::new(text)
            .style(Style::default().fg(Color::Yellow))
            .render(area, buf);
    }
}
