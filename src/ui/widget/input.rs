use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
};

use crate::ui::AppTheme;

pub struct InputField<'a> {
    label: &'a str,
    value: &'a str,
    cursor_pos: u16,
    focused: bool,
    placeholder: Option<&'a str>,
    theme: &'a AppTheme,
}

impl<'a> InputField<'a> {
    pub fn new(label: &'a str, value: &'a str, theme: &'a AppTheme) -> Self {
        Self {
            label,
            value,
            cursor_pos: value.chars().count() as u16,
            focused: false,
            placeholder: None,
            theme,
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
                Span::styled(placeholder, self.theme.style(self.theme.text_muted))
            } else {
                Span::raw("")
            }
        } else {
            Span::styled(self.value, self.theme.style(self.theme.text_primary))
        };

        let cursor = if self.focused {
            Span::styled(
                "▏",
                self.theme
                    .style(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        };

        let content = Line::from(vec![display_value, cursor]);

        let border_type = BorderType::Plain;

        Paragraph::new(content)
            .block(
                Block::default()
                    .title(format!(" {} ", self.label))
                    .title_style(
                        self.theme
                            .style(self.theme.accent)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_type(border_type)
                    .border_style(if self.focused {
                        self.theme.style(self.theme.border_focused)
                    } else {
                        self.theme.style(self.theme.border)
                    })
                    .padding(Padding::uniform(1)),
            )
            .render(area, buf);
    }
}
