use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout},
    widgets::{Block, Borders},
    Frame,
};

use crate::ui::widget::{ChatMessage, InputField, MessageList};
use crate::ui::AppTheme;

pub struct ChatView<'a> {
    messages: &'a [ChatMessage],
    input_text: &'a str,
    is_streaming: bool,
    scroll_offset: u16,
    theme: &'a AppTheme,
}

impl<'a> ChatView<'a> {
    pub fn new(messages: &'a [ChatMessage], input_text: &'a str, theme: &'a AppTheme) -> Self {
        Self {
            messages,
            input_text,
            is_streaming: false,
            scroll_offset: 0,
            theme,
        }
    }

    pub fn streaming(mut self, is_streaming: bool) -> Self {
        self.is_streaming = is_streaming;
        self
    }

    pub fn scroll_offset(mut self, offset: u16) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1), // gap
                Constraint::Length(5),
            ])
            .split(area);

        let messages_area = chunks[0];
        let input_area = chunks[2];

        // Subtle divider between messages and input
        let divider = Block::default()
            .borders(Borders::TOP)
            .border_style(self.theme.style(self.theme.border));
        f.render_widget(divider, chunks[1]);

        let message_list = MessageList::new(self.messages).scroll_offset(self.scroll_offset);
        message_list.render(f, messages_area, self.theme);

        let input_label = if self.is_streaming {
            "Typing..."
        } else {
            "Prompt"
        };

        let input = InputField::new(input_label, self.input_text, self.theme)
            .focused(true);
        f.render_widget(input, input_area);
    }
}
