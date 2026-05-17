use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct StatusBar<'a> {
    mode: &'a str,
    position: &'a str,
    help: &'a str,
}

impl<'a> StatusBar<'a> {
    pub fn new(mode: &'a str, position: &'a str, help: &'a str) -> Self {
        Self {
            mode,
            position,
            help,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_text = format!(" {} ", self.mode);
        let help_text = format!(" {} ", self.help);

        let line_width = area.width as usize;
        let left_len = self.mode.len() + 2;
        let right_len = self.help.len() + 2;
        let center_max = line_width.saturating_sub(left_len + right_len);

        let center_text = if self.position.chars().count() > center_max {
            self.position.chars().take(center_max).collect::<String>()
        } else {
            self.position.to_string()
        };

        let line = Line::from(vec![
            Span::styled(mode_text, Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(center_text),
            Span::styled(help_text, Style::default().fg(Color::DarkGray)),
        ]);

        Paragraph::new(line)
            .style(Style::default().bg(Color::DarkGray))
            .render(area, buf);
    }
}
