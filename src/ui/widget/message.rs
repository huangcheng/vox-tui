use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub is_streaming: bool,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            is_streaming: false,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            is_streaming: false,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            is_streaming: false,
        }
    }

    pub fn streaming(mut self) -> Self {
        self.is_streaming = true;
        self
    }

    fn role_style(&self) -> Style {
        match self.role {
            MessageRole::User => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            MessageRole::Assistant => Style::default()
                .fg(Color::Green),
            MessageRole::System => Style::default()
                .fg(Color::Yellow),
        }
    }

    fn role_label(&self) -> &str {
        match self.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
        }
    }
}

pub struct MessageList<'a> {
    messages: &'a [ChatMessage],
    scroll_offset: u16,
}

impl<'a> MessageList<'a> {
    pub fn new(messages: &'a [ChatMessage]) -> Self {
        Self {
            messages,
            scroll_offset: 0,
        }
    }

    pub fn scroll_offset(mut self, offset: u16) -> Self {
        self.scroll_offset = offset;
        self
    }
}

impl Widget for MessageList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|msg| {
                let role_span = Span::styled(
                    format!("[{}] ", msg.role_label()),
                    msg.role_style(),
                );

                let mut spans = vec![role_span];

                if msg.is_streaming {
                    spans.push(Span::styled(
                        format!("{}▌", msg.content),
                        Style::default().add_modifier(Modifier::DIM),
                    ));
                } else {
                    spans.push(Span::raw(&msg.content));
                }

                let mut result = vec![Line::from(spans)];

                // Add blank line between messages for readability
                if area.width > 40 {
                    result.push(Line::from(""));
                }

                result
            })
            .collect();

        if lines.is_empty() {
            Paragraph::new("No messages yet. Start a conversation!")
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
        } else {
            Paragraph::new(lines)
                .scroll((self.scroll_offset, 0))
                .render(area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_list_with_scroll() {
        let messages = vec![ChatMessage::user("Hello")];
        let list = MessageList::new(&messages).scroll_offset(1);
        assert_eq!(list.scroll_offset, 1);
    }

    #[test]
    fn test_message_list_default_scroll() {
        let messages = vec![ChatMessage::user("Hello")];
        let list = MessageList::new(&messages);
        assert_eq!(list.scroll_offset, 0);
    }
}
