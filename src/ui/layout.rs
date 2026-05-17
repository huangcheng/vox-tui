use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::ui::widget::StatusBar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Image,
    Audio,
    Config,
}

impl View {
    pub fn all() -> &'static [View] {
        &[View::Chat, View::Image, View::Audio, View::Config]
    }

    pub fn name(&self) -> &'static str {
        match self {
            View::Chat => "Chat",
            View::Image => "Image",
            View::Audio => "Audio",
            View::Config => "Config",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            View::Chat => "💬",
            View::Image => "🖼️",
            View::Audio => "🔊",
            View::Config => "⚙️",
        }
    }

    pub fn next(&self) -> View {
        let views = View::all();
        let idx = views.iter().position(|v| v == self).unwrap_or(0);
        views[(idx + 1) % views.len()]
    }
}

pub struct AppLayout {
    pub sidebar: Rect,
    pub main: Rect,
    pub status: Rect,
}

pub fn compute_layout(area: Rect) -> AppLayout {
    let main_layout = RatatuiLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let content_layout = RatatuiLayout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(1)])
        .split(main_layout[0]);

    AppLayout {
        sidebar: content_layout[0],
        main: content_layout[1],
        status: main_layout[1],
    }
}

pub struct AppTheme {
    pub accent: Color,
    pub is_dark: bool,
}

impl AppTheme {
    pub fn from_config(theme: Option<&crate::config::ThemeConfig>) -> Self {
        let (accent, is_dark) = match theme {
            Some(t) => {
                let accent = t.accent_color.as_deref()
                    .and_then(parse_color)
                    .unwrap_or(Color::Cyan);
                let is_dark = t.dark_mode.unwrap_or(true);
                (accent, is_dark)
            }
            None => (Color::Cyan, true),
        };
        Self { accent, is_dark }
    }
}

fn parse_color(name: &str) -> Option<Color> {
    match name.to_lowercase().as_str() {
        "cyan" => Some(Color::Cyan),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "red" => Some(Color::Red),
        "yellow" => Some(Color::Yellow),
        "white" => Some(Color::White),
        _ => None,
    }
}

pub struct Layout;

impl Layout {
    pub fn render(f: &mut Frame, current_view: View, mode: &str, position: &str, help: &str, theme: &AppTheme) {
        let area = f.area();
        let layout = compute_layout(area);

        Self::render_sidebar(f, layout.sidebar, current_view, theme);
        Self::render_main_placeholder(f, layout.main, current_view);
        Self::render_status_bar(f, layout.status, mode, position, help, theme);
    }

    pub fn render_sidebar(f: &mut Frame, area: Rect, current_view: View, theme: &AppTheme) {
        let items: Vec<ListItem> = View::all()
            .iter()
            .map(|v| {
                let style = if *v == current_view {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(v.icon(), style),
                    Span::raw(" "),
                    Span::styled(v.name(), style),
                ]))
            })
            .collect();

        let sidebar = List::new(items)
            .block(
                Block::default()
                    .title(" Navigation ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );

        f.render_widget(sidebar, area);
    }

    pub fn render_main_placeholder(f: &mut Frame, area: Rect, current_view: View) {
        let placeholder = Paragraph::new(format!(
            "{} View\n\nThis is the {} view placeholder.\nContent will be implemented in later tasks.",
            current_view.name(),
            current_view.name()
        ))
        .block(
            Block::default()
                .title(format!(" {} ", current_view.name()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        f.render_widget(placeholder, area);
    }

    pub fn render_status_bar(f: &mut Frame, area: Rect, mode: &str, position: &str, help: &str, _theme: &AppTheme) {
        let status = StatusBar::new(mode, position, help);
        f.render_widget(status, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default() {
        let theme = AppTheme::from_config(None);
        assert_eq!(theme.accent, Color::Cyan);
        assert!(theme.is_dark);
    }

    #[test]
    fn test_theme_custom() {
        let config = crate::config::ThemeConfig {
            accent_color: Some("green".to_string()),
            dark_mode: Some(false),
        };
        let theme = AppTheme::from_config(Some(&config));
        assert_eq!(theme.accent, Color::Green);
        assert!(!theme.is_dark);
    }

    #[test]
    fn test_theme_unknown_color() {
        let config = crate::config::ThemeConfig {
            accent_color: Some("unknown".to_string()),
            dark_mode: None,
        };
        let theme = AppTheme::from_config(Some(&config));
        assert_eq!(theme.accent, Color::Cyan);
        assert!(theme.is_dark);
    }

    #[test]
    fn test_compute_layout() {
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let layout = compute_layout(area);
        assert_eq!(layout.sidebar.width, 24);
        assert_eq!(layout.main.width, 56);
        assert_eq!(layout.status.height, 1);
    }
}
