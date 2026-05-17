# vox-tui

A multi-provider AI multimedia TUI (Text User Interface) built in Rust with Ratatui.

## Features

- **Multi-Provider Support**: Switch between StepFun and MiniMax AI providers
- **Chat Interface**: Full-duplex chat with streaming responses
- **Image Generation**: Text-to-image capabilities
- **Audio/TTS**: Text-to-speech synthesis
- **Configuration Management**: TOML-based config with validation
- **Cross-Platform**: Works on macOS, Linux, and Windows

## Installation

```bash
# From source
git clone https://github.com/yourusername/vox-tui.git
cd vox-tui
cargo install --path .

# Or from crates.io (when published)
cargo install vox-tui
```

## Configuration

Create a configuration file at `~/.config/vox/config.toml` (macOS/Linux) or `%APPDATA%/vox/config.toml` (Windows):

```toml
provider = "stepfun"

[stepfun]
api_key = "sk-your-api-key-here"

[minimax]
api_key = "your-minimax-api-key-here"
```

See `config.example.toml` for all available options.

## Usage

```bash
vox
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1-4` | Switch to view (Chat/Image/Audio/Config) |
| `Tab` / `Shift+Tab` | Next/previous view |
| `Enter` | Focus input / Send message |
| `Escape` | Clear input / Cancel streaming |
| `q` / `Ctrl+C` | Quit |
| `↑` / `↓` | Scroll messages |

## Architecture

```
src/
├── main.rs          # App state, event loop, rendering
├── config.rs        # TOML config with validation
├── input.rs         # Keyboard handling, TextInputState
├── stepfun.rs       # StepFun HTTP client + SSE streaming
├── minimax.rs       # MiniMax HTTP client
├── provider.rs      # Provider trait + factory
└── ui/
    ├── mod.rs       # UI module exports
    ├── layout.rs    # Sidebar + main + status bar layout
    ├── view/        # View components
    │   ├── chat.rs  # Chat view with message list
    │   ├── config.rs # Config display view
    │   ├── image.rs # Image generation view
    │   └── audio.rs # TTS/Audio view
    └── widget/      # Reusable widgets
        ├── button.rs
        ├── input.rs
        ├── message.rs
        ├── spinner.rs
        └── status_bar.rs
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run in debug mode
cargo run

# Release build
cargo build --release
```

## License

MIT
