use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

#[derive(Debug, Clone)]
pub enum ButtonState {
    Default,
    Hover,
    Active,
    Disabled,
}

pub struct Button<'a> {
    label: &'a str,
    state: ButtonState,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            state: ButtonState::Default,
        }
    }

    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = state;
        self
    }

    fn style(&self) -> Style {
        match &self.state {
            ButtonState::Default => Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray),
            ButtonState::Hover => Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan),
            ButtonState::Active => Style::default()
                .fg(Color::Black)
                .bg(Color::Green),
            ButtonState::Disabled => Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Black),
        }
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = self.style();
        let label = if area.width < self.label.len() as u16 + 4 {
            &self.label[..(area.width.saturating_sub(4)) as usize]
        } else {
            self.label
        };
        let content = format!(" {} ", label);
        Paragraph::new(content)
            .style(style)
            .alignment(ratatui::layout::Alignment::Center)
            .render(area, buf);
    }
}
