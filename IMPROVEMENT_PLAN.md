# Vox-TUI Improvement Plan

> **Status:** Draft — based on full codebase audit (2026-05-18)
> **Goal:** Fix compilation, modernize the TUI, harden the architecture, and ship a polished v0.2.0

---

## Phase 0: Fix the Build (P0 — Blocker)

The project **does not compile** due to 5 API mismatches with `ratatui-image 10.0.8`.

| File | Fix |
|------|-----|
| `src/main.rs:45` | `ratatui_image::Protocol` → `ratatui_image::protocol::Protocol` |
| `src/main.rs:154-155` | `font_size.width` / `font_size.height` → `font_size.0` / `font_size.1` |
| `src/main.rs:160` | `picker.new_protocol(..., size)` → `picker.new_protocol(..., size.into())` |
| `src/ui/view/image.rs:123` | Remove `.allow_clipping(true)` (method removed in v10) |

**Acceptance:** `cargo check`, `cargo clippy`, `cargo test` all pass cleanly.

---

## Phase 1: TUI Visual Overhaul (P1 — "Ugly" Fix)

### 1.1 Design System — `AppTheme` That Actually Works

**Current state:** `AppTheme` parses `accent_color` and `dark_mode` from config, but only `accent_color` is partially used. `dark_mode` is a no-op.

**Plan:**
- Replace the flat `AppTheme` with a full semantic palette:
  ```
  background, surface, surface_highlight
  border, border_focused, border_active
  text_primary, text_secondary, text_muted
  accent, accent_dim, success, warning, error
  user_message_bg, assistant_message_bg, system_message_bg
  ```
- Implement **dark/light palette switching** based on `theme.dark_mode`.
- Derive all widget styles from the theme. Zero hardcoded colors in views/widgets.

### 1.2 Kill the "Prison Grid" — Border & Spacing Refinement

**Current state:** Every widget uses `Borders::ALL` with sharp corners. Zero padding. Content touches borders.

**Plan:**
- Remove borders from **message content areas** (chat log, image preview). Use subtle background color blocks instead.
- Keep **light borders** (`BorderType::Rounded` where supported, or thin lines) only on:
  - The outer app frame (1px)
  - The input field (1px, accent when focused)
  - The config popup (1px)
- Add `Padding::uniform(1)` (or `(1, 2)`) to **every** block that contains text.
- Use `Block::default().padding(Padding::new(1, 1, 0, 0))` on sidebar list, message lists, config lists.

### 1.3 Message Bubbles for Chat

**Current state:** Messages render as raw `[You] hello` text with a single blank line. No wrapping, no containers, no visual hierarchy.

**Plan:**
- Render each message as a **bubble** — a `Paragraph` inside a `Block` with:
  - Subtle background color (`user_message_bg` / `assistant_message_bg` / `system_message_bg`)
  - `Padding::horizontal(1)` and `Padding::vertical(0)`
  - No border (or a 1-char left-side accent bar for modern look)
  - `Wrap { trim: true }` for long content
- Right-align user bubbles, left-align assistant bubbles (or indent differently).
- Add a **timestamp** (HH:MM) in `text_muted` next to the role label.
- Style role labels with accent color + bold, not raw hardcoded colors.

### 1.4 Fix the Cursor

**Current state:** `█` (full block) in bright cyan. Looks like a rendering bug.

**Plan:**
- Use `|` (pipe) as the cursor symbol.
- Blink or dim the cursor using `Modifier::DIM` when not actively typing.
- Keep accent color but reduce intensity.

### 1.5 Redesign the Status Bar

**Current state:** A crude "sticker" — black-on-cyan bold badge on a dark gray bar.

**Plan:**
- Use a **single-line bar** with no heavy background blocks.
- Mode indicator: subtle accent-colored text (e.g., `[NORMAL]`, `[INSERT]`) instead of a filled badge.
- Provider/model info: right-aligned in `text_secondary`.
- Help hint: center or right, muted.
- Remove the hard `Black` on `Cyan` contrast.

### 1.6 Sidebar Redesign

**Current state:** Boxed list with emoji causing misalignment. Selected item is just bold + accent text.

**Plan:**
- Remove the outer box border. Use a subtle vertical separator line (`│`) or no separator.
- Replace emoji with **single-width unicode symbols** or just text labels:
  - `Chat`, `Image`, `Audio`, `Config`
- Highlight the selected item with:
  - A background color (`surface_highlight`)
  - An accent-colored left bar (`▎` or `┃`)
  - Bold text
- Use ratatui's native `List::highlight_style` and `List::highlight_symbol` instead of manual item styling.

### 1.7 Empty States

**Current state:** "No messages yet. Start a conversation!" in `DarkGray`. Looks like disabled text.

**Plan:**
- Center the text both horizontally and vertically.
- Use a subtle icon/symbol (e.g., `💬` or a custom ASCII art) + friendly message in `text_secondary`.
- Same treatment for Image, Audio, and Config empty/preview states.

### 1.8 Config Editor Polish

**Current state:** Popup is transparent (no dimming), cursor is `█`, help text overlaps.

**Plan:**
- **Dim the background** when popup is open: render a full-screen `Block` with `bg: Black` and `Modifier::DIM` behind the popup.
- Use rounded/popup-style border (`BorderType::Thick` or `BorderType::Double`).
- Add padding inside the popup.
- Fix the cursor.
- Move help text to the status bar or a single bottom line, not duplicated.

### 1.9 Scrollbar for Message List

**Current state:** `scroll_offset` exists but no visual indicator.

**Plan:**
- Add a `Scrollbar` widget to `MessageList` on the right edge.
- Style with `accent_dim` color.

### 1.10 Audio View — Remove the Fake Input

**Current state:** A static paragraph that looks like an input field but does nothing.

**Plan:**
- Remove the fake input block.
- Replace with a simple inline hint: "Press `Enter` to synthesize" near the status block.
- Or make it a real input field if audio prompts need to be editable independently.

---

## Phase 2: Architecture & Code Quality (P2)

### 2.1 Fix `Config::validate()`

**Current state:** Returns `Ok(())` unconditionally.

**Plan:**
- Validate that at least one provider has a non-empty API key.
- Validate that `default_provider` is a known provider.
- Validate hex color strings in `theme.accent_color`.
- Return structured errors (not just strings).

### 2.2 Remove Code Duplication

**Current state:**
- Provider config override logic duplicated between `run_cli` and slash command handler.
- `ImageView::render()` and `render_with_image()` are ~90% identical.

**Plan:**
- Extract a `ProviderConfigOverlay` helper that applies CLI flags → config.
- Collapse `ImageView` into a single `render()` with conditional image branch.

### 2.3 Wire Up Streaming (StepFun `chat_stream`)

**Current state:** `chat_stream` exists but is `#[allow(dead_code)]`. Not wired to TUI.

**Plan:**
- Add `WorkResult::ChatChunk(String)` for incremental updates.
- In the main loop, append chunks to the last assistant message instead of replacing it.
- Add a `Streaming` input mode that shows a cancel hint.

### 2.4 Implement `/save` Slash Command

**Current state:** `/save` is parsed but does nothing.

**Plan:**
- Save current chat history to a markdown file (`~/.config/vox/history/YYYY-MM-DD_HH-MM.md`).
- Confirm in status bar: "Saved to ~/.config/vox/history/..."

### 2.5 Cross-Platform `open` for Images

**Current state:** Hardcoded `open` (macOS only).

**Plan:**
- Use `open::that(path)` from the `open` crate, or shell out to `xdg-open` / `start`.

### 2.6 Clean Up Dead Code

- Remove `render_main_placeholder` from `layout.rs`.
- Remove or expose `chat_stream` properly.
- Remove debug logging to `/tmp/vox-debug.log` or gate it behind a feature flag / env var.

### 2.7 Error Handling Improvements

**Current state:** `WorkResult::Error(String)` is generic; setting both `image_result` and `audio_result` on error causes UI confusion.

**Plan:**
- Add `WorkResult::ChatError`, `WorkResult::ImageError`, `WorkResult::AudioError` (or a tagged enum).
- Render errors inline in the relevant view, not globally.

### 2.8 Reduce `main.rs` Bloat

**Current state:** `main.rs` is 1315 lines — holds `AppState`, event loop, rendering dispatch, CLI handlers, slash commands.

**Plan:**
- Extract `AppState` and the event loop into `src/app.rs`.
- Extract slash command handlers into `src/command.rs` (currently only parsing).
- Keep `main.rs` as thin entry point only.

---

## Phase 3: Provider Expansion (P3)

### 3.1 StepFun — Implement Missing Capabilities

**Current state:** Only `chat` and `image_generate` are implemented. 5/7 return `Unsupported`.

**Plan:**
- Add speech, video, music, search, vision support if StepFun APIs support them.
- Otherwise, improve error messages to explain *why* (e.g., "StepFun does not expose a TTS endpoint").

### 3.2 Add Retry Logic

**Plan:**
- Wrap provider calls in a retry with exponential backoff (3 attempts, 500ms base).
- Only retry on transient errors (5xx, timeout, connection reset).

---

## Phase 4: Testing & Hardening (P4)

### 4.1 Integration Tests

**Current state:** Only `--help` smoke test.

**Plan:**
- Add mocked provider tests using `mockito` (already in dev-deps).
- Test TUI rendering by driving `AppState` directly (no need for a real terminal).
- Test config load/merge/save cycles with temp directories.

### 4.2 Clippy Clean

**Plan:**
- Run `cargo clippy -- -W clippy::pedantic` after Phase 0.
- Fix all warnings.
- Add a CI check (GitHub Actions) for `cargo check`, `clippy`, `test`.

### 4.3 Doc Comments

**Plan:**
- Add rustdoc to all public types and functions.
- Run `cargo doc` and ensure no warnings.

---

## Appendix A: File Touch Map

| Phase | Files Modified |
|-------|---------------|
| 0 | `src/main.rs`, `src/ui/view/image.rs` |
| 1.1 | `src/ui/layout.rs`, `src/config.rs` |
| 1.2–1.10 | `src/ui/layout.rs`, `src/ui/view/*.rs`, `src/ui/widget/*.rs`, `src/main.rs` |
| 2.1 | `src/config.rs` |
| 2.2 | `src/main.rs`, `src/ui/view/image.rs`, `src/cli.rs` |
| 2.3 | `src/provider.rs`, `src/stepfun.rs`, `src/main.rs` |
| 2.4 | `src/command.rs`, `src/main.rs` |
| 2.5 | `src/main.rs` (or add `open` crate) |
| 2.6 | `src/ui/layout.rs`, `src/stepfun.rs`, `src/main.rs` |
| 2.7 | `src/provider.rs`, `src/main.rs`, `src/ui/view/*.rs` |
| 2.8 | **NEW** `src/app.rs`, `src/main.rs`, `src/command.rs` |
| 3 | `src/stepfun.rs`, `src/provider.rs` |
| 4 | `tests/integration.rs`, **NEW** `.github/workflows/ci.yml` |

---

## Appendix B: Recommended New Dependencies

| Crate | Purpose | Phase |
|-------|---------|-------|
| `open` | Cross-platform `open::that()` for images | 2.5 |
| `chrono` | Timestamps in chat bubbles | 1.3 |
| `tempfile` | Integration test temp dirs | 4.1 |

---

## Appendix C: Success Criteria for v0.2.0

- [ ] `cargo build`, `cargo clippy`, `cargo test` all green
- [ ] Dark/light mode fully functional and toggleable at runtime (`/dark`, `/light` slash commands)
- [ ] Chat has visually distinct message bubbles with wrap, timestamps, and scrollbar
- [ ] No hardcoded colors in view/widget code — all from `AppTheme`
- [ ] Sidebar uses background highlight + left bar indicator, no emoji misalignment
- [ ] Status bar is a clean single-line bar, not a "sticker"
- [ ] Config popup dims background and has padding + proper cursor
- [ ] `/save` writes chat history to markdown
- [ ] Streaming chat works for StepFun
- [ ] `main.rs` is under 200 lines
