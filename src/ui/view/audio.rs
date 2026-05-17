use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::AppTheme;

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
        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        let text_area = chunks[0];
        let status_area = chunks[1];
        let input_area = chunks[2];

        let text_widget = Paragraph::new(self.text)
            .block(
                Block::default()
                    .title(" Text to Speak ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.accent)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(text_widget, text_area);

        let status_display = if self.is_generating {
            "🔄 Generating speech..."
        } else if self.is_playing {
            "🔊 Playing..."
        } else {
            self.status_text
        };

        let status_widget = Paragraph::new(status_display)
            .style(Style::default().fg(if self.is_generating || self.is_playing {
                self.theme.accent
            } else {
                Color::DarkGray
            }))
            .block(
                Block::default()
                    .title(" Status ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );

        f.render_widget(status_widget, status_area);

        let input = Paragraph::new("Type text above and press Enter to generate speech")
            .block(
                Block::default()
                    .title(" Input ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::DarkGray));

        f.render_widget(input, input_area);
    }
}
