use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::ui::AppTheme;

pub struct StatusBar<'a> {
    mode: &'a str,
    position: &'a str,
    help: &'a str,
    theme: &'a AppTheme,
}

impl<'a> StatusBar<'a> {
    pub fn new(mode: &'a str, position: &'a str, help: &'a str, theme: &'a AppTheme) -> Self {
        Self {
            mode,
            position,
            help,
            theme,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Fill status bar background
        for y in area.y..(area.y + area.height) {
            for x in area.x..(area.x + area.width) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(self.theme.surface);
                }
            }
        }

        // Mode badge: highlight background + bold text
        let mode_span = Span::styled(
            format!(" {} ", self.mode),
            Style::default()
                .fg(self.theme.text_primary)
                .bg(self.theme.surface_highlight)
                .add_modifier(Modifier::BOLD),
        );

        let position_span =
            Span::styled(self.position, self.theme.style(self.theme.text_secondary));

        let help_span = Span::styled(self.help, self.theme.style(self.theme.text_muted));

        let line = Line::from(vec![
            mode_span,
            Span::raw("  "),
            position_span,
            Span::raw(" "),
        ]);

        let line_width: u16 = line
            .spans
            .iter()
            .map(|s| s.content.chars().count() as u16)
            .sum();

        let help_width = help_span.content.chars().count() as u16;
        let available = area.width.saturating_sub(line_width);

        if help_width <= available {
            let padding = available - help_width;
            let mut spans = line.spans;
            spans.push(Span::styled(
                " ".repeat(padding as usize),
                self.theme.style_bg(self.theme.surface, self.theme.surface),
            ));
            spans.push(help_span);
            Line::from(spans).render(area, buf);
        } else {
            line.render(area, buf);
        }
    }
}
