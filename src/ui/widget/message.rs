use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget, Wrap},
    Frame,
};

use crate::ui::AppTheme;

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
    pub timestamp: Option<String>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            is_streaming: false,
            timestamp: Self::now_hm(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            is_streaming: false,
            timestamp: Self::now_hm(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            is_streaming: false,
            timestamp: Self::now_hm(),
        }
    }

    fn now_hm() -> Option<String> {
        let now = chrono::Local::now();
        Some(now.format("%H:%M").to_string())
    }

    pub fn streaming(mut self) -> Self {
        self.is_streaming = true;
        self
    }

    pub fn with_timestamp(mut self, ts: impl Into<String>) -> Self {
        self.timestamp = Some(ts.into());
        self
    }

    pub fn role_label(&self) -> &str {
        match self.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
        }
    }

    fn bubble_bg(&self, theme: &AppTheme) -> ratatui::style::Color {
        match self.role {
            MessageRole::User => theme.user_msg_bg,
            MessageRole::Assistant => theme.assistant_msg_bg,
            MessageRole::System => theme.system_msg_bg,
        }
    }

    fn role_color(&self, theme: &AppTheme) -> ratatui::style::Color {
        match self.role {
            MessageRole::User => theme.accent,
            MessageRole::Assistant => theme.success,
            MessageRole::System => theme.warning,
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

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        if self.messages.is_empty() {
            self.render_empty(f, area, theme);
            return;
        }

        // Reserve 1 column for scrollbar
        let content_width = area.width.saturating_sub(1);
        let content_area = Rect {
            width: content_width,
            ..area
        };
        let scrollbar_area = Rect {
            x: area.x + content_width,
            width: 1,
            ..area
        };

        // Compute total height of all messages to determine scroll bounds
        let mut total_height: u16 = 0;
        let mut heights: Vec<u16> = Vec::new();
        for msg in self.messages.iter() {
            let h = self.message_height(msg, content_width, theme);
            heights.push(h);
            total_height = total_height.saturating_add(h);
        }

        let scroll = self.scroll_offset.min(total_height.saturating_sub(1));

        // Render messages, offset by scroll
        let mut y = content_area.y.saturating_sub(scroll);
        for (idx, msg) in self.messages.iter().enumerate() {
            let h = heights[idx];
            let msg_area = Rect {
                x: content_area.x,
                y,
                width: content_width,
                height: h,
            };

            // Only render if visible within content_area
            if y < content_area.y + content_area.height && y + h > content_area.y {
                self.render_message_bubble(f, msg_area, msg, theme);
            }

            y = y.saturating_add(h);
        }

        // Scrollbar
        let visible = content_area.height as usize;
        let total = total_height as usize;
        if total > visible {
                let mut state = ScrollbarState::new(total)
                .position(scroll as usize);
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("░"))
                .thumb_symbol("▓");
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut state);
        }
    }

    fn render_empty(&self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let text = Text::from(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled("◆", theme.style(theme.accent).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled(
                "No messages yet",
                theme.style(theme.text_secondary).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Start typing below to begin chatting.",
                theme.style(theme.text_muted),
            )),
            Line::from(Span::styled(
                "Press Tab to switch views · q to quit",
                theme.style(theme.text_muted).add_modifier(Modifier::DIM)),
            ),
        ]);
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .render(area, f.buffer_mut());
    }

    fn message_height(&self, msg: &ChatMessage, width: u16, _theme: &AppTheme) -> u16 {
        let bubble_width = width.saturating_sub(8); // margin + accent bar + padding + border on both sides
        if bubble_width == 0 {
            return 5;
        }
        let content = if msg.is_streaming {
            format!("{}▌", msg.content)
        } else {
            msg.content.clone()
        };
        // header + wrapped content + border top/bottom + padding top/bottom + gap
        let content_lines = Self::wrapped_line_count(&content, bubble_width);
        1u16.saturating_add(content_lines).saturating_add(4).saturating_add(1)
    }

    fn wrapped_line_count(text: &str, width: u16) -> u16 {
        if width == 0 {
            return 1;
        }
        if text.is_empty() {
            return 1;
        }
        text.lines()
            .map(|line| {
                let chars = line.chars().count() as u16;
                chars.div_ceil(width).max(1)
            })
            .sum()
    }

    fn message_text(&self, msg: &ChatMessage, theme: &AppTheme) -> Text<'static> {
        let ts = msg.timestamp.clone().unwrap_or_default();
        let header = format!("{}  {}", msg.role_label(), ts);
        let content = if msg.is_streaming {
            format!("{}▌", msg.content)
        } else {
            msg.content.clone()
        };

        let mut text = Text::from(vec![Line::from(vec![
            Span::styled(
                header,
                theme
                    .style(msg.role_color(theme))
                    .add_modifier(Modifier::BOLD),
            ),
        ])]);
        for line in content.lines() {
            text.lines.push(Line::from(Span::styled(
                line.to_string(),
                theme.style(theme.text_primary),
            )));
        }
        text
    }

    fn render_message_bubble(&self, f: &mut Frame, area: Rect, msg: &ChatMessage, theme: &AppTheme) {
        // Margin: 2 chars on each side
        let bubble_outer = Rect {
            x: area.x + 2,
            y: area.y,
            width: area.width.saturating_sub(4),
            height: area.height,
        };

        if bubble_outer.width == 0 || bubble_outer.height == 0 {
            return;
        }

        // Left accent bar (1 char wide)
        let accent_color = msg.role_color(theme);
        let accent_bar_width = 1u16;
        let bubble_inner_width = bubble_outer.width.saturating_sub(accent_bar_width + 1); // +1 for right margin

        let accent_area = Rect {
            x: bubble_outer.x,
            y: bubble_outer.y,
            width: accent_bar_width,
            height: bubble_outer.height,
        };

        let content_area = Rect {
            x: bubble_outer.x + accent_bar_width,
            y: bubble_outer.y,
            width: bubble_inner_width,
            height: bubble_outer.height,
        };

        // Draw left accent indicator (▎ left-quarter block)
        for y in accent_area.y..(accent_area.y + accent_area.height) {
            if let Some(cell) = f.buffer_mut().cell_mut((accent_area.x, y)) {
                cell.set_symbol("▎");
                cell.set_fg(accent_color);
            }
        }

        // Draw bubble background for full area
        let bg = msg.bubble_bg(theme);
        for y in bubble_outer.y..(bubble_outer.y + bubble_outer.height) {
            for x in bubble_outer.x..(bubble_outer.x + bubble_outer.width) {
                if let Some(cell) = f.buffer_mut().cell_mut((x, y)) {
                    cell.set_bg(bg);
                }
            }
        }

        // Render text inside the bubble with border
        let text = self.message_text(msg, theme);
        let bubble = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .border_type(BorderType::Plain)
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(theme.style(theme.border))
                    .padding(Padding::uniform(1))
                    .bg(bg),
            );

        bubble.render(content_area, f.buffer_mut());
    }
}

// Legacy Widget impl for backward compatibility (used in tests / simple cases)
impl Widget for MessageList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|msg| {
                let role_span = Span::styled(
                    format!("[{}] ", msg.role_label()),
                    Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(Modifier::BOLD),
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

                if area.width > 40 {
                    result.push(Line::from(""));
                }

                result
            })
            .collect();

        if lines.is_empty() {
            Paragraph::new("No messages yet. Start a conversation!")
                .style(Style::default().fg(ratatui::style::Color::DarkGray))
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
