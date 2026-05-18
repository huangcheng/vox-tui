# Vox-TUI Design Specification

> Source of truth for the terminal UI. All implementation must match this document.
> **Version:** 2.0 (Ardot-refactored)

---

## Design Philosophy

- **Less boxes, more breathing room.** Content areas use background blocks; borders are reserved for input and the outer frame.
- **Semantic colors.** Every color is sourced from `AppTheme`. No hardcoded `Color::Cyan` in views.
- **Dark/Light parity.** Both modes are first-class. `theme.dark_mode` switches entire palettes.
- **Modern terminal aesthetics.** Inspired by `lazygit`, `btm`, `yazi`, `kimi-cli` — clean, minimal, information-dense.
- **v2 refinements:** Softer blue accent (`#58a6ff`), plain single-line borders, left-half accent indicators, badge-style mode labels.

---

## 1. Global Layout

```
┌─ Chat ───────────────────────────────────────────────────────┐
│                                                              │
│  ▸ Chat          │                                           │
│    Image         │  ┌─────────────────────────────────────┐  │
│    Audio         │  │ 14:33                          You  │  │
│    Config        │  │ Hello, can you help me with Rust?    │  │
│                  │  └─────────────────────────────────────┘  │
│                  │                                           │
│                  │  ┌─────────────────────────────────────┐  │
│                  │  │ Assistant                      14:33 │  │
│                  │  │ Of course! What would you like to    │  │
│                  │  │ know?▌                               │  │
│                  │  └─────────────────────────────────────┘  │
│                  │                                           │
│                  │  ▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░    │
│                  │                                           │
│                  │  ┌─────────────────────────────────────┐  │
│                  │  │ Prompt │ Hello, can you help ... |  │  │
│                  │  └─────────────────────────────────────┘  │
├──────────────────┴───────────────────────────────────────────┤
│  NORM  minimax · step-1-8k            Tab:switch q:quit     │
└──────────────────────────────────────────────────────────────┘
```

### Layout Computation (`compute_layout`)

```
┌──────────────────────────────────────────────┐
│ ┌────────┐ ┌──────────────────────────────┐  │
│ │Sidebar │ │         Main                 │  │
│ │  20    │ │        (flex)                │  │
│ │ cols   │ │                              │  │
│ └────────┘ └──────────────────────────────┘  │
├──────────────────────────────────────────────┤
│ Status Bar              1 row                │
└──────────────────────────────────────────────┘
```

| Area | Constraint | Description |
|------|-----------|-------------|
| Sidebar | `Length(20)` | Fixed 20 columns |
| Main | `Min(1)` | Fills remaining width |
| Status | `Length(1)` | Single row at bottom |

The entire terminal background is filled with `theme.background` before any widgets render.

---

## 2. Theme System (`AppTheme`)

### Dark Mode (Default)

| Token | Hex | Usage |
|-------|-----|-------|
| `background` | `#0d1117` | Terminal background fill |
| `surface` | `#161b22` | Sidebar bg, message bubble panel bg |
| `surface_highlight` | `#21262d` | Selected sidebar item, mode badge bg, hover |
| `border` | `#30363d` | Inactive borders, dividers |
| `border_focused` | `#58a6ff` | Focused input border |
| `text_primary` | `#e6edf3` | Main text, messages |
| `text_secondary` | `#8b949e` | Timestamps, hints, status info |
| `text_muted` | `#484f58` | Placeholders, disabled, empty states |
| `accent` | `#58a6ff` | Cursor, user role indicator, titles |
| `accent_dim` | `#388bfd` | Scrollbar, secondary accents |
| `success` | `#3fb950` | Assistant role indicator, OK status |
| `warning` | `#d29922` | System messages, generating spinner |
| `error` | `#f78166` | Error messages, failed requests |
| `user_msg_bg` | `#161b22` | User message bubble background (surface) |
| `assistant_msg_bg` | `#0d1117` | Assistant message bubble background (background) |
| `system_msg_bg` | `#331f1f` | System message bubble (soft red-tinted) |

### Light Mode

| Token | Hex | Usage |
|-------|-----|-------|
| `background` | `#ffffff` | Terminal background |
| `surface` | `#f6f8fa` | Panels, sidebar |
| `surface_highlight` | `#eaeef2` | Selected items, mode badge |
| `border` | `#d0d7de` | Inactive borders |
| `border_focused` | `#0969da` | Focused input |
| `text_primary` | `#1f2328` | Main text |
| `text_secondary` | `#656d76` | Timestamps, hints |
| `text_muted` | `#8c959f` | Placeholders |
| `accent` | `#0969da` | Cursor, highlights |
| `accent_dim` | `#54aeff` | Scrollbar |
| `success` | `#1a7f37` | Assistant role |
| `warning` | `#9a6700` | System messages |
| `error` | `#cf222e` | Errors |
| `user_msg_bg` | `#f6f8fa` | User bubble bg (surface) |
| `assistant_msg_bg` | `#ffffff` | Assistant bubble bg (background) |
| `system_msg_bg` | `#ffffff` | System bubble bg |

### Config Override

- `theme.dark_mode` switches between the two palettes.
- `theme.accent_color` (hex or named) overrides `accent`, `border_focused`, and `accent_dim`.
- Theme is cached in `main.rs` and only recreated when config changes.

---

## 3. Sidebar

```
▸ Chat
  Image
  Audio
  Config
```

- **Width:** 20 columns
- **Background:** Filled with `surface` color
- **Right border:** Single vertical line `│` in `border` color
- **Top padding:** 1 row (`Padding::vertical(1)`)
- **Items:** Plain text labels — no geometric icons
- **Selected indicator:** `▸ ` prefix + `surface_highlight` background + bold `text_primary`
- **Unselected:** `  ` prefix + regular `text_primary`
- **Navigation:** `Tab` / `Shift+Tab` cycles views; input clears on every switch

---

## 4. Chat View

### Layout

```
┌─ Main Area ──────────────────────────────────────────────┐
│                                                          │
│  Message List (scrollable)                               │
│                                                          │
│  ──────────────────────────────────────────────────────  │
│                                                          │
│  Input Field (5 rows)                                    │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

Vertical split: `Min(1)` for messages, `Length(1)` gap with top border divider, `Length(5)` for input.

### Message Bubbles (`MessageList`)

Each message renders as a bubble with:

- **Left accent indicator:** `▎` (left-quarter block) in role color
  - User: `accent` (`#58a6ff`)
  - Assistant: `success` (`#3fb950`)
  - System: `warning` (`#d29922`)
- **Border:** `BorderType::Plain` (single line `─│┌┐└┘`), `Borders::ALL`, `border` color
- **Background:** Role-specific (`user_msg_bg` / `assistant_msg_bg` / `system_msg_bg`)
- **Padding:** `Padding::uniform(1)` inside the bubble
- **Header line:** `Role` + `timestamp` (HH:MM format)
  - Styled with role color + `Modifier::BOLD`
- **Content:** `Wrap { trim: true }` for natural line breaking
- **Streaming indicator:** `▌` cursor appended to the last assistant chunk when `is_streaming`
- **Gap between bubbles:** 1 row

### Scrollbar

- Rendered on the right edge of the message area (reserves 1 column)
- Thumb: `▓`
- Track: `░`
- Only visible when `total_height > visible_height`

### Empty State

When no messages exist:
- Centered vertically and horizontally
- `◆` diamond icon in accent + bold
- "No messages yet" in `text_secondary` + bold
- "Start typing below to begin chatting." in `text_muted`
- "Press Tab to switch views · q to quit" in `text_muted` + `DIM`

### Input Field (`InputField`)

- **Block title:** `Prompt` (or `Typing...` during streaming) in accent + bold
- **Border:** `BorderType::Plain` single line
- **Border color:** `border_focused` when focused, `border` otherwise
- **Padding:** `Padding::uniform(1)`
- **Cursor:** `▏` (thin vertical bar) in accent + bold
- **Placeholder:** Shown in `text_muted` when empty

---

## 5. Image View

### Layout

Same vertical split as Chat: preview area + gap + input field.

### States

**Empty:**
```
┌────────────────────────────────────────┐
│                                        │
│     Image generation preview           │
│     will appear here.                  │
│                                        │
│     Enter a prompt below to generate.  │
│                                        │
└────────────────────────────────────────┘
```
- Centered text in `text_secondary`
- Plain border, `surface` background, uniform padding

**Generating:**
- Same block style
- Spinner widget with label "Generating image..."

**With Image:**
- Image rendered via `ratatui_image::Image` with `Protocol` (Halfblocks protocol forced)
- Timestamped save path shown below: `~/.config/vox/images/YYYY-MM-DD_HH-MM-SS.png`
- Images open in system viewer on `Enter`

### Input Field

- Label: `Prompt` (or `Generating...` during generation)
- Same styling as Chat input

---

## 6. Audio View

### Layout

```
┌─ Main Area ──────────────────────────────────────────────┐
│                                                          │
│  Text Block (scrollable)                                 │
│                                                          │
│  ──────────────────────────────────────────────────────  │
│                                                          │
│  Status Block (4 rows)                                   │
│                                                          │
│  ──────────────────────────────────────────────────────  │
│                                                          │
│  Hint Line (1 row, centered)                             │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Text Block

- Title: `Text` in accent + bold
- Content: The text to be spoken (from input history)
- Plain border, `surface` background, uniform padding

### Status Block

- Title: `Status` in accent + bold
- **Idle:** Shows provider, model, voice info in `text_secondary`
- **Playing:** `▶ Playing...` in accent color
- **Generating:** Spinner with "Generating speech..."

### Hint Line

- `◆ Press Enter to synthesize speech`
- Centered, `text_muted`

---

## 7. Config View

### Layout

Single scrollable list filling the main area.

### List Structure

```
 Configuration

 Provider
 ▎ default_provider: minimax ◀ ▶

 StepFun
   API Key: sk-***...
   Chat Model: step-1-8k ◀ ▶
   Image Model: step-image-edit-2 ◀ ▶
   Speech Model: step-tts-2 ◀ ▶

 MiniMax
   API Key: eyJ-***...
   Chat Model: abab6.5s ◀ ▶
   Image Model: image-01 ◀ ▶
   Speech Model: speech-01 ◀ ▶

 Theme
   Accent Color: #00bcd4
   Dark Mode: true

↑↓ Navigate  ←→: Cycle  Enter: Edit  q: Quit
```

### Field Rendering

- **Section headers:** Underlined accent + bold (e.g., " Provider ")
- **Selected field:** `▎` prefix, accent + bold text, `surface_highlight` background
- **Unselected field:** Two-space prefix, `text_primary`
- **Cyclable fields:** Show `◀ ▶` arrows when more than one option available
- **API keys:** Masked as `prefix***` (first 4 chars visible)

### Editing Popup

When a field is being edited:

1. **Background dimming:** Full area covered with `background` + `Modifier::DIM`
2. **Clear:** `Clear` widget wipes the popup rectangle
3. **Popup block:** Centered, `Borders::ALL`, `border_focused` color
   - Title: "Edit {Field Name}"
   - Padding: `Padding::horizontal(1)`
4. **Input:** Buffer content + `|` cursor
5. **Help line:** "Enter: Save  Esc: Cancel" in `text_muted`

---

## 8. Status Bar

### Layout

```
 NORM  minimax · step-1-8k            Tab:switch q:quit
```

Single line, **surface background**. Three segments separated by **spaces** (not `│`):

| Segment | Content | Style |
|---------|---------|-------|
| Mode | `NORM` / `INS` / `STRM` / `CFG` / `EDT` | Badge: `surface_highlight` bg + `text_primary` bold |
| Position | Provider + model (e.g., `minimax · step-1-8k`) | `text_secondary` |
| Help | View-aware hints | `text_muted`, right-aligned |

### Mode Labels

| Mode | Label | When Active |
|------|-------|-------------|
| Normal | `NORM` | Default navigation mode |
| Typing | `INS` | Input field focused |
| Streaming | `STRM` | Async work in progress |
| Config Nav | `CFG` | Config view, browsing fields |
| Config Edit | `EDT` | Config popup open |

### View-Aware Help Text

| View | Help |
|------|------|
| Chat | `Tab:switch Enter:send q:quit` |
| Image | `Tab:switch Enter:generate q:quit` |
| Audio | `Tab:switch Enter:speak q:quit` |
| Config | `Tab:switch Enter:edit q:quit` |

---

## 9. Widget Catalog

### `InputField`

```rust
InputField::new(label, value, theme)
    .focused(bool)
    .placeholder("Type a prompt...")
    .cursor_position(pos)
```

Renders a bordered input box with title, padding, and a pipe cursor. **Plain border** (not rounded).

### `MessageList`

```rust
MessageList::new(messages)
    .scroll_offset(offset)
    .render(f, area, theme)
```

Renders scrollable message bubbles with scrollbar and 1-row inter-bubble gaps.

### `StatusBar`

```rust
StatusBar::new(mode, position, help, theme)
```

Renders the single-line status bar with **badge-style mode** and right-aligned help.

### `Spinner`

Animated spinner widget for loading states (generating image/speech).

---

## 10. Architecture Notes

### View Isolation

The TUI enforces strict view isolation to prevent cross-view pollution:

- `send_message()` routes by `current_view` — Chat sends chat, Image sends image, Audio sends audio.
- `image_result` and `audio_result` are separate fields; async results never touch `messages`.
- `pending_view: Option<View>` tracks which view initiated async work so `WorkResult::Error` routes to the correct view's result field.
- Input text is cleared on every view switch (`next_view`, `prev_view`, `switch_view`).

### Async Work Flow

1. User submits input → `AppState` sets `input_mode = Streaming`, spawns tokio task
2. Task sends `WorkResult` via `mpsc` channel
3. `tick()` in event loop receives result:
   - `ChatCompletion` → append to messages
   - `ImageGenerated` → set `image_result`, save to `~/.config/vox/images/`
   - `SpeechGenerated` → set `audio_result`
   - `Error` → route to `pending_view` field or push system message

### Image Persistence

- Images saved as PNG with timestamp: `~/.config/vox/images/YYYY-MM-DD_HH-MM-SS.png`
- Full-resolution image also rendered inline via `ratatui-image` (Halfblocks protocol)
- `Enter` on image view opens saved file in system viewer via `open` crate

### Retry Logic

All provider calls are wrapped in `RetryProvider` with exponential backoff:
- 3 attempts max
- Delays: 500ms → 1s → 2s
- Only retries transient errors (5xx, timeouts, connection errors)

---

## 11. v1 → v2 Changelog

| Element | v1 | v2 |
|---------|-----|-----|
| Accent | `#67e8f9` (cyan) | `#58a6ff` (softer blue) |
| Muted | `#6b7280` | `#484f58` (better contrast) |
| Bubble accent | `█` full-height bar | `▎` left-quarter block |
| Bubble border | `BorderType::Rounded` | `BorderType::Plain` |
| Sidebar icons | `◆◈◉◊` geometric symbols | Plain text + `▸` selected indicator |
| Status bar mode | Bare accent text | Badge container (`surface_highlight` bg) |
| Status separator | `│` pipe | Space |
| Status bg | Terminal default | `surface` |
| Config selected | `┃` full bar | `▎` left-half indicator |
| Empty state hint | "Start typing to chat" | "Start typing below to begin chatting." |

---

## 12. File Map

| Component | File |
|-----------|------|
| Theme & layout | `src/ui/layout.rs` |
| Input widget | `src/ui/widget/input.rs` |
| Message bubbles & scrollbar | `src/ui/widget/message.rs` |
| Status bar | `src/ui/widget/status_bar.rs` |
| Spinner | `src/ui/widget/spinner.rs` |
| Chat view | `src/ui/view/chat.rs` |
| Image view | `src/ui/view/image.rs` |
| Audio view | `src/ui/view/audio.rs` |
| Config view | `src/ui/view/config.rs` |
| App state & event loop | `src/app.rs` |
| Entry point | `src/main.rs` |
