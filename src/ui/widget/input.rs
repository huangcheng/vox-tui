use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct InputField<'a> {
    label: &'a str,
    value: &'a str,
    cursor_pos: u16,
    focused: bool,
    placeholder: Option<&'a str>,
}

impl<'a> InputField<'a> {
    pub fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            cursor_pos: value.len() as u16,
            focused: false,
            placeholder: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    pub fn cursor_position(mut self, pos: u16) -> Self {
        self.cursor_pos = pos;
        self
    }
}

impl Widget for InputField<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let display_value = if self.value.is_empty() {
            if let Some(placeholder) = self.placeholder {
                Span::styled(placeholder, Style::default().fg(Color::DarkGray))
            } else {
                Span::raw("")
            }
        } else {
            Span::raw(self.value)
        };

        let cursor = if self.focused {
            Span::styled("█", Style::default().fg(Color::Cyan))
        } else {
            Span::raw("")
        };

        let content = Line::from(vec![
            Span::styled(format!("{}: ", self.label), Style::default().fg(Color::Cyan)),
            display_value,
            cursor,
        ]);

        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if self.focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .render(area, buf);
    }
}
