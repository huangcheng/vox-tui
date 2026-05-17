use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::widget::{InputField, Spinner};
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

        if self.is_generating {
            let block = Block::default()
                .title(" Preview ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(preview_area);
            f.render_widget(block, preview_area);
            let spinner = Spinner::new().label("Generating image...");
            f.render_widget(spinner, inner);
        } else if let Some(text) = self.preview_text {
            let preview = Paragraph::new(text)
                .style(Style::default().fg(Color::Green))
                .block(
                    Block::default()
                        .title(" Preview ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                );
            f.render_widget(preview, preview_area);
        } else {
            let preview = Paragraph::new("Image generation preview will appear here.\n\nEnter a prompt below to generate an image.")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .title(" Preview ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
            f.render_widget(preview, preview_area);
        }

        let input_label = if self.is_generating {
            "▌ Generating..."
        } else {
            "Prompt"
        };

        let input = InputField::new(input_label, self.prompt)
            .focused(!self.is_generating);
        f.render_widget(input, input_area);
    }
}
