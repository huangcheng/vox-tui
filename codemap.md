# Repository Atlas: vox

## Project Responsibility
A multi-provider AI multimedia CLI and TUI application written in Rust. Provides a unified interface to StepFun and MiniMax AI services for text chat, image generation, speech synthesis, video generation, music generation, web search, and image vision analysis. Supports both a traditional CLI mode and an optional terminal UI (TUI) mode.

## System Entry Points
- **`src/main.rs`**: Binary entry point. Dispatches to CLI subcommands or TUI mode based on arguments.
- **`src/lib.rs`**: Library root. Re-exports all public modules for use in tests and external integration.
- **`Cargo.toml`**: Package manifest with optional `tui` feature flag (ratatui + crossterm + ratatui-image + image).
- **`config.example.toml`**: Example configuration file for user setup (~/.config/vox/config.toml).

## Key Architecture Decisions
- **Dual-mode design**: CLI-first with optional TUI behind cargo feature gate
- **Provider abstraction**: `AIProvider` trait with 7 async capabilities, wrapped in `RetryProvider` for resilience
- **Static capability registry**: `ProviderCapabilities` gives instant feedback before API calls
- **Flat model config**: Per-capability model selection in code (`models.rs`), not in config
- **TUI async pattern**: `tokio::sync::mpsc` channels bridge spawned async work to synchronous render loop

## Directory Map (Aggregated)

| Directory | Responsibility Summary | Detailed Map |
|-----------|------------------------|--------------|
| `src/` | Core application: CLI parsing, provider abstraction, config system, API clients, TUI state machine | [View Map](src/codemap.md) |
| `src/ui/` | TUI rendering layer: layout engine, theme system, view components, reusable widgets | [View Map](src/ui/codemap.md) |

## Module Overview

### CLI & Entry (`src/main.rs`)
- `main()` → `run_cli()` or `run_tui()`
- 4-level provider resolution: CLI flag → config → auto-detect → default
- Command handlers for 7 AI capabilities + config/doctor/providers/models
- REPL mode with rustyline for interactive chat

### Configuration (`src/config.rs`)
- TOML-based config with default → user merge → migration → validation pipeline
- `ProviderModels`: flat per-capability model selection
- `ConfigEditor` + `ConfigField`: TUI navigable config editing state machine
- API key masking in Debug output

### Provider Layer (`src/provider.rs`)
- `AIProvider` async trait: chat, image, speech, video, music, search, vision
- `StepFunProvider` and `MiniMaxProvider` implementations
- `RetryProvider` decorator with exponential backoff (3 retries, transient 5xx detection)
- Factory pattern: `create_provider()` returns `Box<dyn AIProvider>`

### API Clients
- `src/minimax.rs`: MiniMax API client (chat, image, speech, video, music, search, vision)
- `src/stepfun.rs`: StepFun API client (chat, image, speech, search, vision; no video/music)

### TUI Layer (`src/app.rs`, `src/input.rs`, `src/ui/`)
- `AppState`: Central state with async channel (`tokio::sync::mpsc`)
- `InputMode` state machine: Normal → Typing → Streaming → ConfigNavigating → ConfigEditing
- 4 views: Chat, Image, Audio, Config
- `AppTheme`: Full semantic color palette with dark/light modes

### Support Modules
- `src/cli.rs`: Clap CLI definitions (12 subcommands)
- `src/models.rs`: Static known-model registry per provider/capability
- `src/capabilities.rs`: Static capability flags per provider
- `src/command.rs`: TUI slash command parser (/provider, /model, /help, /clear, /save, /status)
- `src/output.rs`: CLI output formatting (text/json, quiet/verbose, error tracking)

## External Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `ratatui` + `crossterm` | TUI framework and terminal control (optional, `tui` feature) |
| `ratatui-image` | Inline image rendering in terminal (optional, `tui` feature) |
| `tokio` | Async runtime |
| `reqwest` | HTTP client for API calls |
| `serde` + `serde_json` + `toml` | Serialization |
| `rustyline` | Interactive REPL line editing |
| `indicatif` | CLI progress spinners |
| `chrono` | Timestamps |
| `base64` | Image encoding for vision API |
| `dirs` | Cross-platform config directory resolution |

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `tui` | Off | Enables terminal UI mode (ratatui, crossterm, ratatui-image, image) |

## Capability Matrix

| Capability | MiniMax | StepFun |
|-----------|---------|---------|
| Chat | ✅ | ✅ |
| Image Generation | ✅ | ✅ |
| Speech Synthesis | ✅ | ✅ |
| Video Generation | ✅ | ❌ |
| Music Generation | ✅ | ❌ |
| Web Search | ✅ | ✅ |
| Vision | ✅ | ✅ |
