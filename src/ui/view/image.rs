use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::AppTheme;

pub struct ImageView<'a> {
    prompt: &'a str,
    is_generating: bool,
    preview_text: Option<&'a str>,
    theme: &'a AppTheme,
}

impl<'a> ImageView<'a> {
    pub fn new(prompt: &'a str, theme: &'a AppTheme) -> Self {
        Self {
            prompt,
            is_generating: false,
            preview_text: None,
            theme,
        }
    }

    pub fn generating(mut self, is_generating: bool) -> Self {
        self.is_generating = is_generating;
        self
    }

    pub fn preview(mut self, text: &'a str) -> Self {
        self.preview_text = Some(text);
        self
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        let preview_area = chunks[0];
        let input_area = chunks[1];

        let preview = if self.is_generating {
            Paragraph::new("Generating image...")
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .title(" Preview ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
        } else if let Some(text) = self.preview_text {
            Paragraph::new(text)
                .style(Style::default().fg(Color::Green))
                .block(
                    Block::default()
                        .title(" Preview ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
        } else {
            Paragraph::new("Image generation preview will appear here.\n\nEnter a prompt below to generate an image.")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .title(" Preview ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
        };

        f.render_widget(preview, preview_area);

        let input_label = if self.is_generating {
            "▌ Generating..."
        } else {
            "Prompt"
        };

        let input = Paragraph::new(self.prompt)
            .block(
                Block::default()
                    .title(format!(" {} ", input_label))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if self.is_generating {
                        Color::Yellow
                    } else {
                        self.theme.accent
                    })),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(input, input_area);
    }
}
