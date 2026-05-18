use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding},
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
        .constraints([Constraint::Length(20), Constraint::Min(1)])
        .split(main_layout[0]);

    AppLayout {
        sidebar: content_layout[0],
        main: content_layout[1],
        status: main_layout[1],
    }
}

/// Full semantic color palette derived from config.
/// All widgets should source colors from here — no hardcoded `Color::Cyan` in views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppTheme {
    pub is_dark: bool,
    pub background: Color,
    pub surface: Color,
    pub surface_highlight: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub user_msg_bg: Color,
    pub assistant_msg_bg: Color,
    pub system_msg_bg: Color,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl AppTheme {
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            background: Color::Rgb(13, 17, 23),      // #0d1117
            surface: Color::Rgb(22, 27, 34),         // #161b22
            surface_highlight: Color::Rgb(33, 38, 45), // #21262d
            border: Color::Rgb(48, 54, 61),          // #30363d
            border_focused: Color::Rgb(88, 166, 255), // #58a6ff
            text_primary: Color::Rgb(230, 237, 243),  // #e6edf3
            text_secondary: Color::Rgb(139, 148, 158), // #8b949e
            text_muted: Color::Rgb(72, 79, 88),      // #484f58
            accent: Color::Rgb(88, 166, 255),        // #58a6ff
            accent_dim: Color::Rgb(56, 139, 253),    // #388bfd
            success: Color::Rgb(63, 185, 80),        // #3fb950
            warning: Color::Rgb(210, 153, 34),       // #d29922
            error: Color::Rgb(247, 129, 102),        // #f78166
            user_msg_bg: Color::Rgb(22, 27, 34),     // #161b22 (surface)
            assistant_msg_bg: Color::Rgb(13, 17, 23), // #0d1117 (background)
            system_msg_bg: Color::Rgb(51, 31, 31),   // #331f1f soft red-tinted
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            background: Color::Rgb(255, 255, 255),   // #ffffff
            surface: Color::Rgb(246, 248, 250),      // #f6f8fa
            surface_highlight: Color::Rgb(234, 238, 242), // #eaeef2
            border: Color::Rgb(208, 215, 222),       // #d0d7de
            border_focused: Color::Rgb(9, 105, 218), // #0969da
            text_primary: Color::Rgb(31, 35, 40),    // #1f2328
            text_secondary: Color::Rgb(101, 109, 118), // #656d76
            text_muted: Color::Rgb(140, 149, 159),   // #8c959f
            accent: Color::Rgb(9, 105, 218),         // #0969da
            accent_dim: Color::Rgb(84, 174, 255),    // #54aeff
            success: Color::Rgb(26, 127, 55),        // #1a7f37
            warning: Color::Rgb(154, 103, 0),        // #9a6700
            error: Color::Rgb(207, 34, 46),          // #cf222e
            user_msg_bg: Color::Rgb(246, 248, 250),  // #f6f8fa (surface)
            assistant_msg_bg: Color::Rgb(255, 255, 255), // #ffffff (background)
            system_msg_bg: Color::Rgb(255, 255, 255), // #ffffff
        }
    }

    pub fn from_config(theme: Option<&crate::config::ThemeConfig>) -> Self {
        let is_dark = theme.and_then(|t| t.dark_mode).unwrap_or(true);
        let accent = theme
            .and_then(|t| t.accent_color.as_deref())
            .and_then(parse_color);

        let mut palette = if is_dark { Self::dark() } else { Self::light() };
        if let Some(accent) = accent {
            palette.accent = accent;
            palette.border_focused = accent;
            palette.accent_dim = accent;
        }
        palette
    }

    pub fn style(&self, fg: Color) -> Style {
        Style::default().fg(fg)
    }

    pub fn style_bg(&self, fg: Color, bg: Color) -> Style {
        Style::default().fg(fg).bg(bg)
    }
}

fn parse_color(name: &str) -> Option<Color> {
    let lower = name.trim().to_lowercase();
    if let Some(hex) = lower.strip_prefix('#') {
        if hex.len() == 6
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            )
        {
            return Some(Color::Rgb(r, g, b));
        }
        return None;
    }
    match lower.as_str() {
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
        Self::render_status_bar(f, layout.status, mode, position, help, theme);
    }

    pub fn render_sidebar(f: &mut Frame, area: Rect, current_view: View, theme: &AppTheme) {
        // Fill sidebar background
        for y in area.y..(area.y + area.height) {
            for x in area.x..(area.x + area.width) {
                if let Some(cell) = f.buffer_mut().cell_mut((x, y)) {
                    cell.set_bg(theme.surface);
                }
            }
        }

        let items: Vec<ListItem> = View::all()
            .iter()
            .map(|v| {
                ListItem::new(Line::from(Span::styled(
                    format!("  {}", v.name()),
                    theme.style(theme.text_primary),
                )))
            })
            .collect();

        let sidebar = List::new(items)
            .highlight_symbol("▸ ")
            .highlight_style(
                theme
                    .style(theme.text_primary)
                    .add_modifier(Modifier::BOLD)
                    .bg(theme.surface_highlight),
            )
            .block(
                Block::default()
                    .padding(Padding::vertical(1))
                    .borders(Borders::RIGHT)
                    .border_style(theme.style(theme.border)),
            );

        let mut state = ratatui::widgets::ListState::default();
        let selected = View::all().iter().position(|v| *v == current_view).unwrap_or(0);
        state.select(Some(selected));

        f.render_stateful_widget(sidebar, area, &mut state);
    }

    pub fn render_status_bar(f: &mut Frame, area: Rect, mode: &str, position: &str, help: &str, theme: &AppTheme) {
        let status = StatusBar::new(mode, position, help, theme);
        f.render_widget(status, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default() {
        let theme = AppTheme::from_config(None);
        assert!(theme.is_dark);
        assert_eq!(theme.accent, AppTheme::dark().accent);
    }

    #[test]
    fn test_theme_custom() {
        let config = crate::config::ThemeConfig {
            accent_color: Some("green".to_string()),
            dark_mode: Some(false),
        };
        let theme = AppTheme::from_config(Some(&config));
        assert!(!theme.is_dark);
        assert_eq!(theme.accent, Color::Green);
    }

    #[test]
    fn test_theme_unknown_color() {
        let config = crate::config::ThemeConfig {
            accent_color: Some("unknown".to_string()),
            dark_mode: None,
        };
        let theme = AppTheme::from_config(Some(&config));
        assert!(theme.is_dark);
        assert_eq!(theme.accent, AppTheme::dark().accent);
    }

    #[test]
    fn test_compute_layout() {
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let layout = compute_layout(area);
        assert_eq!(layout.sidebar.width, 20);
        assert_eq!(layout.main.width, 60);
        assert_eq!(layout.status.height, 1);
    }
}
