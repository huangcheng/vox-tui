---
name: vox-cli
description: >
  Use the vox CLI to generate text, images, speech, video, music, perform web search, and analyze images via multiple AI providers (StepFun, MiniMax).
  Trigger this skill whenever the user wants to generate AI content from the command line, synthesize speech, create images, produce video or music,
  search the web, analyze images, or manage AI provider configurations — even if they don't mention "vox" by name.
  Also use when the user asks to "generate an image", "make a TTS", "convert text to speech", "create a video", "compose music",
  "search the web", "analyze a photo", "chat with AI", or any similar AI generation task from a terminal.
---

# vox CLI — AI Agent Companion Guide

vox is a multi-provider AI multimedia CLI. It gives you a single interface to StepFun and MiniMax for 7 capabilities: text chat, image generation, speech synthesis, video generation, music generation, web search, and vision/image understanding.

## Installation

```bash
# From crates.io (recommended)
cargo install vox-ai

# Or build from source
git clone https://github.com/huangcheng/vox.git
cd vox
cargo build --release
# Binary at target/release/vox
```

The crate name is `vox-ai` but the installed binary is `vox`.

After installing, verify it works:

```bash
vox --help
```

## First-Time Setup

Before any AI task, the user needs at least one provider configured. Check with `vox doctor`:

```bash
vox doctor
```

If no providers are configured, add one:

```bash
vox provider add stepfun YOUR_API_KEY
vox provider add minimax YOUR_API_KEY
```

Config lives at `~/.config/vox/config.toml` (macOS/Linux) or `%APPDATA%\vox\config.toml` (Windows). The default provider is set via `provider` key in config, or auto-detected when only one has an API key.

## Capability Matrix

| Capability | MiniMax | StepFun | Command |
|-----------|---------|---------|---------|
| Text Chat | yes | yes | `vox text chat` |
| Image Generation | yes | yes | `vox image generate` |
| Speech Synthesis | yes | yes | `vox speech generate` |
| Video Generation | yes | **no** | `vox video generate` |
| Music Generation | yes | **no** | `vox music generate` |
| Web Search | yes | yes | `vox search query` |
| Vision (image understanding) | yes | yes | `vox vision analyze` |

StepFun lacks video and music. Use `--provider minimax` for those.

## Command Reference

### Text Chat

```bash
# Single message
vox text chat --message "Explain Rust ownership"

# With system prompt
vox text chat --message "Hello" --system "You are a pirate"

# Interactive REPL (multi-turn with history)
vox text repl
vox text repl --system "You are a helpful coding tutor"

# Text completion
vox text complete "The future of AI is"
```

### Image Generation

```bash
# Basic
vox image generate "A cat in space"

# With options
vox image generate "sunset over mountains" --aspect-ratio 16:9 -o sunset.png -n 2

# Aspect ratios: 1:1, 16:9, 4:3, 3:2, 2:3, 3:4, 9:16, 21:9
# -n: number of images (1-9)
# -o: output file path
```

### Speech Synthesis

```bash
# Basic
vox speech generate --text "Hello world"

# With options
vox speech generate --text "你好世界" --voice cixingnansheng --speed 1.2 --format mp3 -o hello.mp3

# Voices: cixingnansheng (default), and provider-specific voices
# Speed: 0.5-2.0 (default 1.0)
# Formats: mp3, wav, flac, pcm, opus
```

### Video Generation (MiniMax only)

```bash
vox video generate --prompt "Ocean waves crashing on rocks" --duration 10 --resolution 1080P -o waves.mp4

# Duration: 6 or 10 seconds (default 6)
# Resolution: 720P, 768P, 1080P (default 720P)
# Requires: --provider minimax (or minimax as default)
```

### Music Generation (MiniMax only)

```bash
# With lyrics
vox music generate --prompt "Upbeat pop song" --lyrics "[Verse] La da dee da" -o song.mp3

# Instrumental
vox music generate --prompt "Jazz piano" --instrumental -o jazz.mp3

# Requires: --provider minimax (or minimax as default)
```

### Web Search

```bash
vox search query "latest Rust news"
vox search query "Rust programming language" --count 10
```

### Vision / Image Understanding

```bash
# Analyze with default prompt
vox vision analyze photo.jpg

# With custom question
vox vision analyze photo.jpg --prompt "What breed is this dog?"
```

### Diagnostics

```bash
# Full health check
vox doctor

# Check specific item
vox doctor --check config
```

### Provider Management

```bash
# List configured providers
vox provider list

# Add provider
vox provider add stepfun YOUR_API_KEY
vox provider add minimax YOUR_API_KEY

# Test connectivity
vox provider status
vox provider status -p minimax

# Remove provider
vox provider remove stepfun
```

### Model Management

```bash
# List models per capability
vox models list
vox models list --capability chat

# Override default model for a capability
vox models set chat MiniMax-M2.7
vox models set speech speech-2.8-hd
```

### Configuration

```bash
# Show config (API keys are masked)
vox config show

# Get specific value
vox config get default_provider
vox config get stepfun.api_key   # returns masked

# Set value
vox config set default_provider minimax

# Edit in $EDITOR
vox config edit
```

### Global Options

All commands accept these flags:

| Flag | Description | Example |
|------|-------------|---------|
| `--provider <name>` | Override provider | `--provider minimax` |
| `--model <name>` | Override model | `--model step-1-8k` |
| `--format json` | JSON output for scripting | `--format json` |
| `--output-dir <dir>` | Default output directory | `--output-dir ./out` |
| `--config <path>` | Custom config file | `--config /tmp/test.toml` |
| `--quiet` | Suppress progress | `--quiet` |
| `--verbose` | Debug output | `--verbose` |
| `tui` | Launch terminal UI (requires tui feature) | `vox tui` |

## Provider Details

### StepFun (default)

- Base URL: `https://api.stepfun.com/v1`
- Chat models: `step-1-8k` (default), `step-1-32k`, `step-1-128k`, `step-1-flash`, `step-2-16k`, `step-2-32k`, `step-3.5-flash`
- Image models: `step-image-edit-2` (default), `step-2x-large`, `step-1x-medium`
- Speech models: `step-tts-2` (default), `step-tts-mini`, `stepaudio-2.5-tts`
- Vision models: `step-1v-8k`
- Search models: `step-search`

### MiniMax

- Base URL: `https://api.minimaxi.com/v1`
- Chat models: `MiniMax-M2.7` (default), `MiniMax-M2.5`, `MiniMax-M2.1`, `MiniMax-M2`
- Image models: `image-01` (default), `image-01-live`
- Speech models: `speech-01` (default), `speech-02-turbo`, `speech-2.6-hd`, `speech-2.8-hd`
- Video models: `MiniMax-Hailuo-2.3` (default), `MiniMax-Hailuo-02`
- Music models: `music-2.6`

## Common Agent Workflows

### Workflow 1: Generate and use an image

```bash
vox image generate "a diagram showing microservices architecture" -o diagram.png --format json
# The image is saved to diagram.png (or the URL is printed in JSON mode)
```

### Workflow 2: Research a topic

```bash
# Search the web
vox search query "Rust async runtime comparison 2025"

# Then chat about results
vox text chat --message "Compare tokio vs async-std based on these search results: [paste results]"
```

### Workflow 3: Text-to-speech for accessibility

```bash
vox speech generate --text "$(cat notes.txt)" --voice cixingnansheng -o notes_audio.mp3
```

### Workflow 4: Analyze a screenshot

```bash
vox vision analyze screenshot.png --prompt "What error is shown and how do I fix it?"
```

### Workflow 5: Script-friendly output

```bash
# Get structured JSON for scripting
vox text chat --message "List 5 colors" --format json
vox search query "Rust news" --format json
```

## Error Handling

- **"No providers configured"**: Run `vox provider add <name> <api_key>`
- **"does not support video_generate"**: Switch to MiniMax with `--provider minimax`
- **401 errors**: API key is invalid — update with `vox provider add <name> <new_key>`
- **5xx errors**: vox auto-retries 3 times with exponential backoff (500ms, 1s, 2s)
- **Config migration**: vox auto-upgrades old model names (`speech-01` -> newer) and old API URLs on load

## Config File Reference

```toml
provider = "stepfun"           # default provider

[stepfun]
api_key = "sk-your-key"
# base_url = "https://api.stepfun.com/v1"   # optional
# model = "step-1-8k"                        # optional chat model override

[minimax]
api_key = "your-key"
# base_url = "https://api.minimaxi.com/v1"   # optional
# model = "MiniMax-M2.7"                     # optional chat model override

[theme]
# accent_color = "#00bcd4"   # optional TUI accent
# dark_mode = true           # optional TUI mode
```

## Important Notes for Agents

1. **Always check setup first**: Run `vox doctor` before assuming vox works. If it fails, help the user configure a provider.
2. **Video and music are MiniMax-only**: Add `--provider minimax` when using `vox video` or `vox music`.
3. **API keys are positional**: `vox provider add stepfun KEY` — the key is the second argument.
4. **Config get masks keys**: `vox config get stepfun.api_key` returns `sk-yo***`, not the real key.
5. **JSON output for scripting**: Use `--format json` when you need to parse output programmatically.
6. **Output files**: Image and speech commands accept `-o` / `--out` for output path. Without it, files go to the current directory or `--output-dir`.
7. **`vox tui`** requires the `tui` cargo feature. Binary releases ship without it by default. Pass `--tab chat|image|audio|config` to open directly to a tab.
