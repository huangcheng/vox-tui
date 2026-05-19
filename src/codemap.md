# vox-tui — Source Code Codemap

> Multi-provider AI multimedia CLI & TUI application supporting StepFun and MiniMax providers.

## Responsibility

The `src/` directory is organized into **four layers**:

### Layer 1: Entry & CLI (`main.rs`, `cli.rs`, `lib.rs`)

| File | Responsibility |
|------|---------------|
| `main.rs` | Binary entry point. Dispatches to CLI (`run_cli`) or TUI (`run_tui`) mode. Contains provider resolution (CLI flag > config > auto-detect > default), CLI command handlers (`handle_text`, `handle_image`, `handle_speech`, `handle_video`, `handle_music`, `handle_search`, `handle_vision`), REPL loop (`handle_text_repl`), diagnostics (`handle_doctor`), config management, shell completion, and utility functions (`create_spinner`, `download_file`). ~1450 lines. |
| `cli.rs` | Clap-based CLI definitions. `Cli` struct with `GlobalOpts` (provider, model, API key, format, output dir, config path, quiet/verbose/no-color flags) and `Commands` enum. Subcommand structs for each capability: `TextCommand`, `ImageCommand`, `SpeechCommand`, `VideoCommand`, `MusicCommand`, `SearchCommand`, `VisionCommand`. Config commands: `ConfigCommand`, `ModelsCommand`, `ProvidersCommand`. `DoctorArgs` for diagnostics. ~650 lines with extensive unit tests. |
| `lib.rs` | Library root. Re-exports all public modules (`capabilities`, `cli`, `command`, `config`, `minimax`, `models`, `output`, `provider`, `stepfun`). Enables use as a library crate. |

### Layer 2: Configuration (`config.rs`, `models.rs`, `capabilities.rs`)

| File | Responsibility |
|------|---------------|
| `config.rs` | Configuration system. `Config` struct with provider configs, theme, output_dir. `StepFunConfig`, `MiniMaxConfig`, `ProviderModels` (flat per-capability model selection). `ConfigField` enum for TUI-navigable fields. `ConfigEditor` state machine for TUI editing. Config loading pipeline: defaults → user merge → migrate → validate. TOML serialization with API key masking in Debug. `ConfigError` enum. ~1350 lines. |
| `models.rs` | Known model registry. `KnownModels` struct with static model lists and defaults per provider/capability. `get_known_models()` and `get_available_models()` functions. MiniMax: chat, image, speech, video, music, vision. StepFun: chat, image, speech, vision, search. ~110 lines. |
| `capabilities.rs` | Static capability flags per provider. `ProviderCapabilities` struct with 7 boolean fields. `for_provider()` returns the static instance. `require()` checks capability support with descriptive error messages. MiniMax: all enabled. StepFun: no video or music. ~100 lines. |

### Layer 3: Provider Abstraction (`provider.rs`, `stepfun.rs`, `minimax.rs`)

| File | Responsibility |
|------|---------------|
| `provider.rs` | Provider abstraction layer. `AIProvider` trait (async_trait) with 7 capabilities: `chat`, `image_generate`, `speech_synthesize`, `video_generate`, `music_generate`, `search`, `vision`. `StepFunProvider` and `MiniMaxProvider` implementing `AIProvider`. `RetryProvider` decorator with exponential backoff (3 retries, transient detection). Factory: `create_provider()` / `create_provider_with_client()`. Data types: `Message`, `CompletionResponse`, `ImageResponse`, `SpeechResponse`, `VideoResponse`, `MusicResponse`, `SearchResponse`, `VisionResponse`. `WorkResult` enum for TUI async communication. `ProviderError` unified error type. ~700 lines. |
| `stepfun.rs` | StepFun API client. `StepFunClient` with `chat`, `image_generate`, `speech_synthesize`, `search`, `vision`. OpenAI-compatible chat completions API (`/v1/chat/completions`). `file_to_data_uri` helper for local image base64 encoding. `StepFunError` error type. ~610 lines. |
| `minimax.rs` | MiniMax API client. `MiniMaxClient` with `chat`, `image_generate`, `speech_synthesize`, `video_generate`, `music_generate`, `search`, `vision`. Uses hex-encoded audio responses, `group_id` parameter. `MiniMaxError` error type. ~490 lines. |

### Layer 4: TUI (`app.rs`, `input.rs`, `command.rs`, `output.rs`, `ui/`)

| File | Responsibility |
|------|---------------|
| `app.rs` | TUI application state (behind `#[cfg(feature = "tui")]`). `AppState` struct with message history, input state, config, async channel. View switching (Chat/Image/Audio/Config). Input mode state machine (Normal/Typing/Streaming/ConfigNavigating/ConfigEditing). Async work spawning: `send_chat`, `send_image`, `send_audio`. `tick()` processes `WorkResult` from async tasks. Slash command handling in TUI context. Conversation save to markdown. ~955 lines. |
| `input.rs` | TUI input handling (behind `#[cfg(feature = "tui")]`). `InputMode` enum. `InputAction` enum. `handle_key_event()`: crossterm key → action mapping. `TextInputState`: UTF-8-safe cursor, insert, delete, navigation. ~450 lines. |
| `command.rs` | Slash command parser for TUI. `SlashCommand` enum: Provider, Model, Help, Clear, Save, Status, Unknown. `parse_slash_command()` and `complete_command()`. ~170 lines. |
| `output.rs` | Output formatting. `Output` struct with Text/JSON formats. `result()`, `status()`, `error()`, `debug()` methods. Exit code tracking via `last_error_code`. ~110 lines. |

### UI Subdirectory (`ui/`)

| File | Responsibility |
|------|---------------|
| `ui/mod.rs` | UI module root. Re-exports `AppLayout`, `AppTheme`, `Layout`, `View` from layout; `AudioView`, `ChatView`, `ConfigField`, `ConfigView`, `ImageView` from view. |
| `ui/layout.rs` | Layout computation and theming. `compute_layout()` returns `AppLayout` (sidebar 20 cols, main area, status bar 1 row). `AppTheme` struct with full semantic color palette (dark/light modes, customizable accent). `Layout::render()` orchestrates sidebar + status bar rendering. ~290 lines. |
| `ui/view/mod.rs` | View module root. Re-exports `AudioView`, `ChatView`, `ConfigView`, `ImageView`, `ConfigField`. |
| `ui/view/chat.rs` | Chat view rendering. Message list display with user/assistant/system styling. Streaming indicator. |
| `ui/view/image.rs` | Image view rendering. Generation prompt input, result preview with `ratatui-image` protocol support. |
| `ui/view/audio.rs` | Audio view rendering. Speech synthesis UI, audio result display. |
| `ui/view/config.rs` | Config view rendering. Field list navigation, value editing with `ConfigEditor` state machine. |
| `ui/widget/mod.rs` | Widget module root. Re-exports `InputField`, `ChatMessage`, `MessageList`, `MessageRole`, `Spinner`, `StatusBar`. |
| `ui/widget/input.rs` | Text input widget. Focused input field with cursor, placeholder text, theme-aware styling. |
| `ui/widget/message.rs` | Chat message widget. `ChatMessage` struct with role/content, `MessageList` for scrollable rendering, `MessageRole` enum. |
| `ui/widget/spinner.rs` | Animated spinner widget for async operations. |
| `ui/widget/status_bar.rs` | Bottom status bar widget. Displays mode, position, help text with theme colors. |

---

## Design

### Provider Resolution (4-level cascade)

```
CLI flag (--provider) → config.default_provider → auto-detect (only 1 key set) → error
```

Implemented in `resolve_provider()` in `main.rs`. Auto-detect checks if exactly one provider has a non-empty API key.

### Provider Abstraction

```
AIProvider (trait)
├── StepFunProvider  →  StepFunClient  →  reqwest HTTP
└── MiniMaxProvider  →  MiniMaxClient  →  reqwest HTTP
    └── RetryProvider (decorator, 3 retries, exponential backoff)
```

The `AIProvider` trait defines 7 async methods. Each provider client handles its own API protocol. `RetryProvider` wraps any `AIProvider` with retry logic.

### Configuration Architecture

```
Config (loaded from ~/.config/vox/config.toml)
├── default_provider: Provider
├── stepfun: Option<StepFunConfig>
│   ├── api_key, base_url
│   └── models: ProviderModels (chat, image, speech, vision, search)
├── minimax: Option<MiniMaxConfig>
│   ├── api_key, base_url, group_id
│   └── models: ProviderModels (chat, image, speech, video, music, vision)
└── theme: ThemeConfig (dark_mode, accent_color)
```

**Separation of concerns**: Config stores user *choices*. Known model *lists* live in `models.rs` (code, not config).

### TUI State Machine

```
InputMode
├── Normal       → 1-4: switch view, Tab: focus input, q/Ctrl+C: quit
├── Typing       → char: input, Enter: submit, Esc: cancel
├── Streaming    → Esc: cancel (awaiting async response)
├── ConfigNavigating → Tab/↑↓: navigate fields, Enter: edit
└── ConfigEditing    → char: edit value, Enter: save, Esc: cancel
```

### Async Work Pattern (TUI)

```
AppState.send_chat() → spawn tokio task → tx.send(WorkResult::ChatResponse)
                                                    ↓
AppState.tick() ← work_rx.try_recv() ← update messages, reset mode
```

`WorkResult` enum carries typed results back to the event loop. `pending_view` tracks which view initiated the operation for routing results/errors.

### Theme System

`AppTheme` provides a full semantic color palette derived from config:
- Dark/light modes with GitHub-inspired colors
- Customizable accent color (hex or named)
- All widgets source colors from `AppTheme` — no hardcoded colors in views

---

## Flow

### CLI Request Flow

```
main()
├── Cli::parse()                          (clap)
├── run_cli(cli)
│   ├── Config::load()                    (merge defaults + user)
│   ├── resolve_provider()                (4-level cascade)
│   ├── create_provider()                 (factory)
│   └── match command {
│       Text(cmd)     → handle_text()     → provider.chat()
│       Image(cmd)    → handle_image()    → provider.image_generate()
│       Speech(cmd)   → handle_speech()   → provider.speech_synthesize()
│       Video(cmd)    → handle_video()    → provider.video_generate()
│       Music(cmd)    → handle_music()    → provider.music_generate()
│       Search(cmd)   → handle_search()   → provider.search()
│       Vision(cmd)   → handle_vision()   → provider.vision()
│       Doctor        → handle_doctor()   (diagnostics)
│       Config        → handle_config()   (TUI editor)
│       Models        → handle_models()   (list/select)
│       Providers     → handle_providers() (list)
│       Completion    → handle_completion() (shell)
│   }
└── Output::result/error()                (format + exit code)
```

### TUI Event Loop

```
run_tui()
├── terminal setup (crossterm)
├── AppState::new_for_tui()
├── loop {
│   ├── terminal.draw(|f| {
│   │   ├── compute_layout()
│   │   ├── render_sidebar()
│   │   ├── match current_view {
│   │   │   Chat  → ChatView::render()
│   │   │   Image → ImageView::render()
│   │   │   Audio → AudioView::render()
│   │   │   Config→ ConfigView::render()
│   │   }
│   │   └── render_status_bar()
│   })
│   ├── input::handle_key_event()         (crossterm → InputAction)
│   ├── match input_mode {
│   │   Normal     → view switch, quit, focus input
│   │   Typing     → text input, submit
│   │   Streaming  → escape only
│   │   Config*    → config navigation/editing
│   }
│   ├── if Submit → spawn async work (send_chat, send_image, send_audio)
│   ├── if slash command → parse_slash_command() → execute
│   └── state.tick()                       (process WorkResult from channel)
│ }
└── terminal restore
```

### Provider Call Flow

```
handle_text(prompt)
├── capabilities.require("chat", provider)   (fail fast if unsupported)
├── create_spinner()
├── provider.chat(&messages)
│   ├── [RetryProvider] retry loop
│   ├── StepFunProvider::chat()
│   │   └── StepFunClient::chat()
│   │       ├── POST /v1/chat/completions
│   │       └── parse ChatCompletionResponse
│   └── or MiniMaxProvider::chat()
│       └── MiniMaxClient::chat()
│           ├── POST /v1/text/chat
│           └── parse ChatResponse
├── stop_spinner()
└── output.result()
```

### Configuration Load Flow

```
Config::load()
├── Determine config path: --config flag > VOX_CONFIG env > ~/.config/vox/config.toml
├── If not found → write DEFAULT_CONFIG_TOML
├── Parse TOML
├── Merge with defaults (user values override)
├── Apply env var overrides (VOX_API_KEY → active provider's api_key)
├── Migrate (if schema version changed)
├── Validate (API keys non-empty for active provider)
└── Return Config
```

---

## Integration

### Module Dependency Graph

```
main.rs
├── cli.rs          (CLI definitions)
├── config.rs       (Config loading, ConfigEditor)
├── provider.rs     (AIProvider trait, factory, WorkResult)
│   ├── stepfun.rs  (StepFunClient)
│   └── minimax.rs  (MiniMaxClient)
├── capabilities.rs (ProviderCapabilities)
├── models.rs       (KnownModels registry)
├── output.rs       (Output formatter)
├── command.rs      (Slash command parser)
├── app.rs          (TUI AppState) [cfg tui]
│   ├── input.rs    (InputMode, TextInputState) [cfg tui]
│   ├── config.rs   (ConfigEditor)
│   ├── command.rs  (SlashCommand)
│   ├── provider.rs (WorkResult, create_provider)
│   └── ui/
│       ├── layout.rs    (View, AppTheme, compute_layout)
│       ├── view/        (ChatView, ImageView, AudioView, ConfigView)
│       └── widget/      (InputField, ChatMessage, Spinner, StatusBar)
└── lib.rs          (re-exports)
```

### External Crates

| Crate | Used By | Purpose |
|-------|---------|---------|
| `clap` | `cli.rs`, `main.rs` | CLI argument parsing |
| `clap_complete` | `main.rs` | Shell completion generation |
| `tokio` | `main.rs`, `app.rs`, `provider.rs`, `stepfun.rs`, `minimax.rs` | Async runtime |
| `reqwest` | `stepfun.rs`, `minimax.rs` | HTTP client |
| `serde`/`serde_json` | `config.rs`, `stepfun.rs`, `minimax.rs`, `output.rs` | Serialization |
| `ratatui` | `ui/`, `app.rs` | TUI framework |
| `crossterm` | `input.rs`, `app.rs` | Terminal I/O |
| `ratatui-image` | `app.rs`, `ui/view/image.rs` | Image rendering in TUI |
| `rustyline` | `main.rs` | REPL line editing |
| `async_trait` | `provider.rs` | Trait methods with async |
| `chrono` | `app.rs` | Timestamps for file naming |
| `base64` | `stepfun.rs` | Base64 encoding for image data URIs |

### Feature Flags

- `tui` (optional): Gates `app.rs`, `input.rs`, `ui/` modules. When disabled, TUI mode is unavailable and the binary only supports CLI commands.

### Data Flow Boundaries

1. **CLI → Provider**: `main.rs` command handlers call `provider.rs` trait methods, which dispatch to `stepfun.rs` or `minimax.rs` clients.
2. **TUI → Provider**: `app.rs` spawns tokio tasks that call the same `provider.rs` trait methods, sending `WorkResult` back via mpsc channel.
3. **Config → All**: `config.rs` is loaded by both CLI and TUI. Provider resolution uses config values. Theme config feeds `AppTheme`.
4. **Models/Capabilities → Validation**: `models.rs` and `capabilities.rs` are consulted before API calls to provide instant feedback on unsupported operations.
