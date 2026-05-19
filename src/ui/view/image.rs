use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout as RatatuiLayout},
    style::Stylize,
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};
use ratatui_image::{Image, protocol::Protocol};

use crate::ui::AppTheme;
use crate::ui::widget::{InputField, Spinner};

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
        self.render_with_image(f, area, None);
    }

    pub fn render_with_image(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        image_protocol: Option<&Protocol>,
    ) {
        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1), // gap
                Constraint::Length(5),
            ])
            .split(area);

        let preview_area = chunks[0];
        let input_area = chunks[2];
        let theme = self.theme;

        // Common preview block — subtle, consistent
        let preview_block = Block::default()
            .border_type(BorderType::Plain)
            .borders(Borders::ALL)
            .border_style(theme.style(theme.border))
            .padding(Padding::uniform(1))
            .bg(theme.surface);

        if self.is_generating {
            let inner = preview_block.inner(preview_area);
            f.render_widget(preview_block, preview_area);

            let spinner = Spinner::new().label("Generating image...").theme(theme);
            f.render_widget(spinner, inner);
        } else if let Some(proto) = image_protocol {
            let inner = preview_block.inner(preview_area);
            f.render_widget(preview_block, preview_area);

            let image_widget = Image::new(proto);
            f.render_widget(image_widget, inner);
        } else if let Some(text) = self.preview_text {
            let preview = Paragraph::new(text)
                .style(theme.style(theme.text_primary))
                .block(preview_block);
            f.render_widget(preview, preview_area);
        } else {
            let preview = Paragraph::new("Image generation preview will appear here.\n\nEnter a prompt below to generate an image.")
                .style(theme.style(theme.text_secondary))
                .alignment(ratatui::layout::Alignment::Center)
                .block(preview_block);
            f.render_widget(preview, preview_area);
        }

        let input_label = if self.is_generating {
            "Generating..."
        } else {
            "Prompt"
        };

        let input = InputField::new(input_label, self.prompt, theme).focused(true);
        f.render_widget(input, input_area);
    }
}
