# UI Codemap — `src/ui/`

The TUI rendering layer for `vox-tui`, a multi-provider AI multimedia CLI. Built with **ratatui** + **crossterm**, gated behind the `tui` cargo feature.

---

## Responsibility

The `ui` module owns all terminal rendering and layout. It translates application state (chat history, image generation progress, audio synthesis status, config values) into a structured terminal interface with:

- **Three-region layout**: fixed 20-char sidebar, flexible main content area, 1-row status bar
- **Four views**: Chat, Image, Audio, Config — cycled via `View::next()`
- **Semantic theming**: dark/light palettes with configurable accent color, no hardcoded colors in views
- **Reusable widgets**: message bubbles, text inputs, status bars, loading spinners

### Module Structure

```
src/ui/
├── mod.rs          # Public API: re-exports View, AppTheme, Layout, all views
├── layout.rs       # Layout engine, theme system, sidebar/status bar rendering (~289 lines)
├── view/
│   ├── mod.rs      # Re-exports: ChatView, ImageView, AudioView, ConfigView, ConfigField
│   ├── chat.rs     # Chat view: message list + input area (~71 lines)
│   ├── image.rs    # Image generation view with preview (~102 lines)
│   ├── audio.rs    # Audio/speech synthesis view (~108 lines)
│   └── config.rs   # Config editor with popup editing (~258 lines)
└── widget/
    ├── mod.rs      # Re-exports: InputField, ChatMessage, MessageList, MessageRole, Spinner, StatusBar
    ├── message.rs  # Chat message widget with bubble rendering (~387 lines)
    ├── input.rs    # Text input widget with cursor (~86 lines)
    ├── status_bar.rs # Status bar widget (~88 lines)
    └── spinner.rs  # Loading spinner widget (~72 lines)
```

---

## Design

### Theme System (`layout.rs`)

The `AppTheme` struct provides a complete semantic color palette — all widgets source colors from here, never hardcoded.

```rust
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
```

Two built-in palettes: `AppTheme::dark()` (GitHub dark mode inspired) and `AppTheme::light()`. The `AppTheme::from_config()` constructor reads `ThemeConfig` to override `is_dark` and `accent_color` (supports `#RRGGBB` hex or named colors like `cyan`, `green`, `blue`).

Helper methods:
- `style(fg)` → `Style` with foreground color
- `style_bg(fg, bg)` → `Style` with foreground + background

### Layout Engine (`layout.rs`)

```rust
pub struct AppLayout {
    pub sidebar: Rect,  // 20 chars wide, full height minus status bar
    pub main: Rect,     // Remaining width, full height minus status bar
    pub status: Rect,   // Full width, 1 row at bottom
}

pub fn compute_layout(area: Rect) -> AppLayout
```

Layout computation uses ratatui's constraint system:
1. Vertical split: main content (Min 1) + status bar (Length 1)
2. Horizontal split of main: sidebar (Length 20) + content (Min 1)

The `Layout` struct provides static render methods:
- `Layout::render()` — full frame render (sidebar + status bar)
- `Layout::render_sidebar()` — sidebar with view list, highlight symbol `▸`, bold selected
- `Layout::render_status_bar()` — delegates to `StatusBar` widget

### View Enum (`layout.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Image,
    Audio,
    Config,
}
```

Methods:
- `View::all()` → `&'static [View]` in cycle order
- `View::name()` → display name for sidebar
- `View::next()` → cycles to next view (wraps around)

---

## Flow

### Rendering Pipeline

Each frame follows this pattern:

```
App::draw()
  ├── compute_layout(area) → AppLayout
  ├── Layout::render_sidebar()     // View list with selection indicator
  ├── match current_view {
  │     Chat  → ChatView::render()
  │     Image → ImageView::render() or ImageView::render_with_image()
  │     Audio → AudioView::render()
  │     Config → ConfigView::render()
  └── Layout::render_status_bar()  // Mode + position + help text
```

### View-Specific Flows

**ChatView** (`chat.rs`):
```
ChatView::new(messages, input_text, theme)
  .streaming(bool)       // Shows "Typing..." label
  .scroll_offset(u16)    // Scroll position in message list
  .render(frame, area)
    ├── Split: messages (Min 1) + divider (Length 1) + input (Length 5)
    ├── MessageList::new(messages).render()  // Bubble rendering with scrollbar
    └── InputField::new("Prompt", text).render()
```

**ImageView** (`image.rs`):
```
ImageView::new(prompt, theme)
  .generating(bool)      // Shows spinner
  .preview(text)         // Fallback text preview
  .render_with_image(frame, area, Option<&Protocol>)
    ├── Split: preview (Min 1) + divider + input (Length 5)
    ├── States:
    │   generating → Spinner widget in bordered box
    │   image_protocol → ratatui_image::Image widget
    │   preview_text → Paragraph with text
    │   empty → Placeholder text
    └── InputField::new("Prompt" or "Generating...")
```

**AudioView** (`audio.rs`):
```
AudioView::new(text, status_text, theme)
  .playing(bool)         // Shows "▶ Playing..."
  .generating(bool)      // Shows spinner
  .render(frame, area)
    ├── Split: text (Min 1) + divider + status (Length 4) + divider + hint (Length 1)
    ├── Text block: " Text " title with accent color
    ├── Status block:
    │   generating → Spinner
    │   playing → "▶ Playing..." in accent color
    │   idle → status_text in secondary color
    └── Hint: "◆ Press Enter to synthesize speech"
```

**ConfigView** (`config.rs`):
```
ConfigView::new(config, theme)
  .with_selected(index)    // Current field selection
  .with_editing(bool)      // Popup edit mode
  .with_edit_buffer(str)   // Edit field value
  .render(frame, area)
    ├── Build field list from ConfigField::build_fields()
    ├── Section headers: " Provider ", " StepFun ", " MiniMax "
    ├── Field rendering with ▎ prefix for selected
    ├── API key masking: show first 4 chars + "***"
    ├── Model selection with ◀ ▶ indicators
    ├── Help text at bottom
    └── If editing: centered popup with dim background, Clear widget, input display
```

### Widget Rendering

**MessageList** (`message.rs`) — the most complex widget:

1. **Empty state**: centered decorative message with diamond symbol
2. **Scroll calculation**: pre-compute height of each message, determine visible range
3. **Per-message rendering**:
   - Reserve 1 column for scrollbar (right edge)
   - Draw left accent bar (`▎` character) in role-specific color
   - Fill bubble background (role-specific)
   - Render bordered paragraph with role label, timestamp, content
   - Streaming messages show `▌` cursor suffix
4. **Scrollbar**: `Scrollbar` widget with `░` track and `▓` thumb

Message roles map to colors:
- `User` → accent color label, surface background
- `Assistant` → success (green) label, background color
- `System` → warning (yellow) label, red-tinted background

**InputField** (`input.rs`):
- Bordered block with bold accent title
- Focused state: `border_focused` color, blinking `▏` cursor
- Placeholder text in muted color when empty

**StatusBar** (`status_bar.rs`):
- Surface background fill
- Mode badge: bold text on surface_highlight background
- Position text: secondary color
- Help text: right-aligned, muted color

**Spinner** (`spinner.rs`):
- 10-frame Braille animation (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`)
- Frame counter divided by 3 for ~3fps at 60fps refresh
- Optional label text, warning color by default

---

## Integration

### Entry Point

The UI is consumed by `src/app.rs` (the main application struct):

```rust
// In app.rs
use crate::ui::{AppTheme, Layout, View, ChatView, ImageView, AudioView, ConfigView};

fn draw(&mut self, frame: &mut Frame) {
    let theme = AppTheme::from_config(self.config.theme.as_ref());
    let area = frame.area();
    let layout = compute_layout(area);

    // Sidebar + status bar
    Layout::render(frame, self.current_view, &self.mode, &self.position, &self.help, &theme);

    // Main content
    match self.current_view {
        View::Chat => ChatView::new(&self.messages, &self.input, &theme)
            .streaming(self.is_streaming)
            .scroll_offset(self.scroll_offset)
            .render(frame, layout.main),
        View::Image => ImageView::new(&self.image_prompt, &theme)
            .generating(self.image_generating)
            .render(frame, layout.main),
        View::Audio => AudioView::new(&self.audio_text, &self.audio_status, &theme)
            .playing(self.audio_playing)
            .generating(self.audio_generating)
            .render(frame, layout.main),
        View::Config => ConfigView::new(&self.config, &theme)
            .with_selected(self.config_selected)
            .with_editing(self.config_editing)
            .with_edit_buffer(&self.config_edit_buffer)
            .render(frame, layout.main),
    }
}
```

### Data Flow

```
Config (src/config/)
  └── AppTheme::from_config() ──► Theme colors

AppState (src/app.rs)
  ├── messages: Vec<ChatMessage>
  ├── current_view: View
  ├── input: String
  ├── config: Config
  └── ...state flags (streaming, generating, etc.)
        │
        └── View::new(...).builder_methods().render()
              └── Widget::render(area, buf)
                    └── ratatui buffer mutation
```

### Key Dependencies

- **ratatui**: Core TUI framework (Frame, Rect, Style, Color, widgets)
- **crossterm**: Terminal backend (via ratatui)
- **ratatui-image**: Image protocol support for ImageView
- **chrono**: Timestamp formatting in ChatMessage

### Testing

- `layout.rs`: Theme construction tests (default, custom accent, unknown color fallback), layout computation test
- `message.rs`: MessageList scroll offset tests
- Tests use `ratatui::layout::Rect::new()` for area construction

### Performance Notes

- MessageList pre-computes all message heights on each render (O(n) with message count)
- Sidebar background is filled cell-by-cell (could use `buf.set_style()` for area fill)
- ConfigView rebuilds field list on each render via `ConfigField::build_fields()`
- No explicit double-buffering beyond ratatui's internal buffer
