# vox — Multi-Provider AI Multimedia CLI

A Rust CLI that provides a unified interface to multiple AI providers (StepFun, MiniMax) for text, image, speech, video, music, search, and vision capabilities.

## Features

- **Multi-provider**: StepFun and MiniMax with shared OpenAI-compatible base adapter
- **7 capabilities**: text chat/completion, image generation, speech synthesis, video generation, music generation, web search, vision/image understanding
- **Interactive REPL**: `vox text repl` for multi-turn chat with history
- **Provider management**: `vox provider add/remove/list/status`
- **Model management**: `vox models list/set` per capability per provider
- **Diagnostics**: `vox doctor` to check config, connectivity, and auth
- **Auto-retry**: exponential backoff (3 retries) on transient failures
- **Config migration**: auto-upgrades old model names and API URLs
- **JSON output**: `--format json` for scripting
- **Shell completion**: `vox completion bash|zsh|fish|elvish`

## Install

```bash
git clone https://github.com/huangcheng/vox.git
cd vox
cargo build --release
# Binary at target/release/vox
```

## Quick Start

```bash
# Add your API key
vox provider add stepfun YOUR_API_KEY
vox provider add minimax YOUR_API_KEY

# Chat
vox text chat --message "Explain Rust ownership"
vox --provider minimax text chat --message "Hello"

# Generate image
vox image generate "A cat in space" --output cat.png

# Speech synthesis
vox speech generate --text "Hello world" --voice cixingnansheng --out hello.mp3

# Web search
vox search query "Rust programming language"

# Vision
vox vision analyze photo.jpg --prompt "What's in this image?"

# Launch TUI mode
vox --tui
```

## Configuration

Config file location:
- macOS/Linux: `~/.config/vox/config.toml`
- Windows: `%APPDATA%\vox\config.toml`

Example (`config.example.toml`):

```toml
provider = "stepfun"

[stepfun]
api_key = "sk-your-api-key-here"

[minimax]
api_key = "your-minimax-api-key-here"
```

### Provider Details

| Provider | Base URL | Chat Model | Speech Model |
|----------|----------|------------|--------------|
| StepFun | `https://api.stepfun.com/v1` | step-1-8k | stepaudio-2.5-tts |
| MiniMax | `https://api.minimaxi.com/v1` | MiniMax-M2.7 | speech-2.8-hd |

### Model Override

```toml
[minimax]
api_key = "..."
model = "MiniMax-M2.7-highspeed"  # Override default chat model
```

Or per-capability:

```bash
vox models set speech speech-2.8-hd
vox models list --provider stepfun
```

## CLI Reference

```
vox [OPTIONS] [COMMAND]

Commands:
  text        Text generation and chat
  image       Image generation
  speech      Speech synthesis (TTS)
  video       Video generation
  music       Music generation
  search      Web search
  vision      Image understanding
  doctor      Run diagnostics
  provider    Manage providers
  models      Manage models
  config      Manage configuration
  completion  Shell completion script

Options:
  --provider <PROVIDER>      Provider (minimax, stepfun)
  --model <MODEL>            Model name override
  --format <FORMAT>          Output format (text, json) [default: text]
  --output-dir <DIR>         Default output directory
  --config <PATH>            Config file path
  --quiet                    Suppress progress output
  --verbose                  Debug output
```

## Architecture

```
src/
  providers/
    mod.rs       AIProvider trait, RetryProvider, factory
    openai.rs    Shared OpenAI-compatible HTTP client
    stepfun.rs   StepFun adapter (~200 LOC)
    minimax.rs   MiniMax adapter (~230 LOC)
  config.rs      Config, migration, provider/model management
  cli.rs         Clap CLI definitions
  app.rs         Command dispatch
  capabilities.rs  Per-provider capability flags
  models.rs      Static model registry
```

The `AIProvider` trait defines capabilities (`chat`, `image_generate`, `speech_synthesize`, etc.). The shared `OpenAIClient` provides default implementations for OpenAI-compatible endpoints — providers only override unique APIs.

## License

MIT
