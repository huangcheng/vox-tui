use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout as RatatuiLayout},
    style::Stylize,
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::ui::AppTheme;
use crate::ui::widget::Spinner;

pub struct AudioView<'a> {
    text: &'a str,
    is_playing: bool,
    is_generating: bool,
    status_text: &'a str,
    theme: &'a AppTheme,
}

impl<'a> AudioView<'a> {
    pub fn new(text: &'a str, status_text: &'a str, theme: &'a AppTheme) -> Self {
        Self {
            text,
            is_playing: false,
            is_generating: false,
            status_text,
            theme,
        }
    }

    pub fn playing(mut self, is_playing: bool) -> Self {
        self.is_playing = is_playing;
        self
    }

    pub fn generating(mut self, is_generating: bool) -> Self {
        self.is_generating = is_generating;
        self
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let theme = self.theme;

        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1), // gap
                Constraint::Length(4),
                Constraint::Length(1), // gap
                Constraint::Length(1),
            ])
            .split(area);

        let text_area = chunks[0];
        let status_area = chunks[2];
        let hint_area = chunks[4];

        // Common block style
        let block = || {
            Block::default()
                .border_type(BorderType::Plain)
                .borders(Borders::ALL)
                .border_style(theme.style(theme.border))
                .padding(Padding::uniform(1))
                .bg(theme.surface)
        };

        // Text to speak
        let text_widget = Paragraph::new(self.text)
            .block(
                block().title(" Text ").title_style(
                    theme
                        .style(theme.accent)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
            )
            .style(theme.style(theme.text_primary));
        f.render_widget(text_widget, text_area);

        // Status block
        if self.is_generating {
            let inner = block().inner(status_area);
            f.render_widget(
                block().title(" Status ").title_style(
                    theme
                        .style(theme.accent)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                status_area,
            );
            let spinner = Spinner::new().label("Generating speech...").theme(theme);
            f.render_widget(spinner, inner);
        } else {
            let status_display = if self.is_playing {
                "▶ Playing..."
            } else {
                self.status_text
            };

            let status_color = if self.is_playing {
                theme.accent
            } else {
                theme.text_secondary
            };

            let status_widget = Paragraph::new(status_display)
                .style(theme.style(status_color))
                .block(
                    block().title(" Status ").title_style(
                        theme
                            .style(theme.accent)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                );

            f.render_widget(status_widget, status_area);
        }

        // Inline hint
        let hint = Paragraph::new("◆ Press Enter to synthesize speech")
            .style(theme.style(theme.text_muted))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(hint, hint_area);
    }
}
