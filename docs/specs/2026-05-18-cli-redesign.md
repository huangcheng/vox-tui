# Vox CLI Redesign Spec

> **Date:** 2026-05-18
> **Status:** Reviewed — final
> **Pattern reference:** [MiniMax-AI/cli](https://github.com/MiniMax-AI/cli) — `vox <resource> <action> [flags]`

---

## 1. Provider Capability Matrix

Not all providers support all features. The CLI must know what's available and give clear errors for unsupported combinations.

| Capability | StepFun | MiniMax |
|---|---|---|
| **Chat (text completion)** | `step-1-8k`, `step-1-32k`, `step-1-128k`, `step-1-flash`, `step-2-16k`, `step-2-32k`, `step-3.5-flash` | `MiniMax-M2.7`, `MiniMax-M2.5`, `MiniMax-M2.1`, `MiniMax-M2` |
| **Vision (image understanding)** | `step-1v-8k` (via chat multimodal) | `vision-01` |
| **Image generation** | `step-image-edit-2`, `step-2x-large`, `step-1x-medium` | `image-01`, `image-01-live` |
| **Speech synthesis (TTS)** | `step-tts-2`, `step-tts-mini`, `stepaudio-2.5-tts` | `speech-01`, `speech-02-turbo`, `speech-2.6-hd`, `speech-2.8-hd` |
| **Video generation** | **Not supported** | `MiniMax-Hailuo-2.3`, `MiniMax-Hailuo-02` |
| **Music generation** | **Not supported** | `music-2.6` |
| **Web search** | `step-search` | MiniMax search API |
| **Voice cloning** | Supported (via voices API) | Supported (via voice cloning API) |
| **Audio input (speech-to-text)** | `stepaudio-2.5-chat` (realtime) | Not supported (standalone) |
| **Streaming** | SSE (chat), SSE/WS (speech) | SSE (chat, speech) |

### Provider-specific auth

| Field | StepFun | MiniMax |
|---|---|---|
| **API key** | Required | Required |
| **Group ID** | Not needed | Required |
| **Base URL** | `https://api.stepfun.com/v1` | `https://api.minimax.chat/v1` (global) or `https://api.minimaxi.com/v1` (CN) |

---

## 2. Command Structure

Adopt the MiniMax CLI's two-level `vox <resource> <action>` pattern. Provider is a global flag, defaults from config.

```
vox <resource> <action> [flags]
```

### Resource/Action Map

**AI capabilities:**
```
vox text chat              # Text generation / chat
vox image generate         # Image generation
vox speech synthesize      # Text-to-speech
vox speech list-voices     # List available voice IDs
vox video generate         # Video generation (async)
vox video status           # Query video task status
vox music generate         # Music generation
vox search query           # Web search
vox vision describe        # Image understanding
```

**Setup & configuration:**
```
vox init                   # First-time interactive setup wizard
vox doctor                 # Diagnose setup issues and connectivity
vox providers list         # List configured providers + status
vox providers status       # Check auth/connectivity
vox models list            # List available models (per provider)
vox models set             # Set model for a capability
vox config show            # Display current config
vox config get             # Get a single config value
vox config set             # Set any config value (power-user)
vox config edit            # Open config in $EDITOR
vox completion <shell>     # Generate shell completions
```

> **Design principle:** `init` → `models list` → `models set` is the guided path. `config set` with dotted keys is the power-user escape hatch.

### Backward Compatibility (hidden aliases)

Old flat commands still parse, hidden from `--help`:

| Old (still works) | Maps to |
|---|---|
| `vox image "cat"` | `vox image generate --prompt "cat"` |
| `vox speech --text "hi"` | `vox speech synthesize --text "hi"` |
| `vox video --prompt "x"` | `vox video generate --prompt "x"` |
| `vox music --prompt "x"` | `vox music generate --prompt "x"` |
| `vox search "query"` | `vox search query "query"` |
| `vox vision file.jpg` | `vox vision describe --file file.jpg` |

> **Deprecation warnings:** Old aliases print a one-time warning to stderr: `Warning: 'vox image "cat"' is deprecated. Use 'vox image generate --prompt "cat"' instead.` Hidden from `--help` but functional.

---

## 3. Global Flags

Available on every command. Provider-aware defaults come from config file.

```
  --provider <name>       Provider to use: minimax, stepfun (default: from config)
  --model <model>         Override model for this action
  --api-key <key>         Override API key
  --base-url <url>        Override API base URL
  --format <format>       Output format: text (default), json
  --output-dir <path>     Directory for saved files (default: ./vox-output)
  --config <path>         Override config file path
  --quiet                 Suppress non-essential output
  --verbose               Print request/response details
  --timeout <seconds>     Request timeout (default: 120)
  --no-color              Disable ANSI colors
  --help                  Show help
  --version               Print version
```

# (no --group-id flag — provider-specific; set via 'vox config set minimax.group_id' or 'vox init')

### Environment Variables

All global flags also read from env vars (lower priority than CLI flags):

| Flag | Env var |
|---|---|
| `--provider` | `VOX_PROVIDER` |
| `--model` | `VOX_MODEL` |
| `--api-key` | `VOX_API_KEY` |
| `--base-url` | `VOX_BASE_URL` |
| `--format` | `VOX_FORMAT` |
| `--output-dir` | `VOX_OUTPUT_DIR` |
| `--config` | `VOX_CONFIG` |
| `--timeout` | `VOX_TIMEOUT` |

Priority: **CLI flag > env var > config file > built-in default**

### Migration from MMX_* env vars

The previous version used `MMX_*` prefixed env vars. These are deprecated but still accepted during a transition period (with a deprecation warning to stderr):

| Old env var | New env var | Removed in |
|---|---|---|
| `MMX_PROVIDER` | `VOX_PROVIDER` | v1.0 |
| `MMX_MODEL` | `VOX_MODEL` | v1.0 |
| `MMX_API_KEY` | `VOX_API_KEY` | v1.0 |
| `MMX_GROUP_ID` | (removed — use config) | v1.0 |
| `MMX_BASE_URL` | `VOX_BASE_URL` | v1.0 |

Priority with migration: **CLI flag > VOX_* env > MMX_* env (deprecated) > config file > built-in default**

---

## 4. Per-Command Flags

### `vox text chat`
```
Options:
  --message <text>         Chat message (required, or --file, or interactive)
  --file <path>            Read message from file (- for stdin)
  --system <text>          System prompt
  --stream                 Stream response tokens (default: off)
  --max-tokens <n>         Max tokens to generate
  --temperature <float>    Sampling temperature 0.0–2.0
  --top-p <float>          Top-p sampling
```

When no `--message` and no `--file` and stdin is a terminal, enters interactive multi-turn chat mode. Use Ctrl+C or type `/exit` to quit.

**Edge cases:**
- `--message` and `--file` both provided → error: "Specify either --message or --file, not both"
- `--message ""` (empty string) → treated as "provided but empty", enters interactive mode with empty initial message
- `--file -` reads from stdin; if stdin is not a terminal, reads all stdin as the message

`--verbose` masks `Authorization` header values in HTTP logs to prevent credential leaks.

### `vox image generate`
```
Options:
  --prompt <text>           Image description (required)
  --size <WxH>              Image size (e.g. 1024x1024, 768x1360) (provider-specific)
  --aspect-ratio <ratio>    Aspect ratio shortcut: 1:1, 16:9, 4:3 etc. (MiniMax)
  --n <count>               Number of images (MiniMax, default: 1)
  --image <path>            Source image for image2image or editing (StepFun)
  --mask <path>             Mask image for inpainting (StepFun, requires --image)
  --out <path>              Save to file/directory (prints URLs if omitted)
  --no-save                 Print URLs only, don't save to disk
  --seed <int>              Random seed (StepFun)
  --steps <int>             Generation steps (StepFun)
```

Model override uses the global `--model` flag.

### `vox speech synthesize`
```
Options:
  --text <text>             Text to synthesize (required, or --file)
  --file <path>             Read text from file (- for stdin)
  --voice <id>              Voice ID (default: provider-specific)
  --speed <float>           Speed 0.5–2.0 (default: 1.0)
  --format <fmt>            Output format: mp3, wav, flac, opus, pcm (default: mp3)
  --out <path>              Output file path (default: output.<format>)
  --stream                  Stream audio as it generates
  --refer <path>            Reference audio file for voice cloning
```

### `vox speech list-voices`
```
Options:
  --provider <name>         List voices for specific provider (default: active provider)
```

Lists available voice IDs for TTS. Output shows voice ID, name, language, and description.

```
$ vox speech list-voices --provider minimax

MiniMax voices:

  ID                  Name              Gender  Language
  male-qn-qingse      青涩青年          Male    Chinese
  male-qn-jingying    精英青年          Male    Chinese
  male-qn-badao       霸道青年          Male    Chinese
  female-shaonv       少女              Female  Chinese
  female-yujie        御姐              Female  Chinese
  female-chengshu     成熟女性          Female  Chinese
  presenter_male      主持人            Male    Chinese
  presenter_female    主持人            Female  Chinese
  ...

Use with: vox speech synthesize --voice male-qn-qingse --text "你好"
```

With `--format json` for scripting: `vox speech list-voices --format json | jq '.[0].id'`.

### `vox video generate`
```
Options:
  --prompt <text>           Video description (required)
  --duration <seconds>      Duration: 6 or 10 (default: 6)
  --resolution <res>        Resolution: 720P, 768P, 1080P (default: 720P)
  --first-frame <path>      First frame image for img2video
  --last-frame <path>       Last frame image for img2video
  --poll                    Poll until completion (default: just return task_id)
  --out <path>              Download video to file (implies --poll)
```

### `vox video status`
```
Options:
  --task-id <id>            Task ID to query (required)
  --download <path>         Download completed video to file (only if status is complete)
  --watch                   Poll continuously until complete, then print result
```

### `vox music generate`
```
Options:
  --prompt <text>           Style description (required)
  --lyrics <text>           Song lyrics with [Verse], [Chorus] tags
  --lyrics-file <path>      Read lyrics from file (- for stdin)
  --instrumental            Instrumental only (no vocals)
  --format <fmt>            Output format (default: mp3)
  --out <path>              Output file path (default: music_<timestamp>.mp3)
```

### `vox search query`
```
Options:
  --query <query>           Search query (required, or positional)
  --count <n>               Number of results 1–10 (default: 5)
```

Also accepts positional query: `vox search "cats"` is equivalent to `vox search query --query "cats"`.

### `vox vision describe`
```
Options:
  --file <path>             Image file path (required)
  --prompt <text>           Question about the image
```

### `vox init`
```
Options (interactive mode):
  (no options — walks through setup with prompts)

Options (non-interactive mode):
  --provider <name>       Provider to configure (required for non-interactive)
  --api-key <key>         API key for the provider
  --group-id <id>         Group ID (MiniMax only)
  --base-url <url>        Override base URL
  --default               Set as default provider
  --yes                   Accept all defaults, skip prompts (for CI)
```

First-time setup wizard. Walks the user through:
1. **Select provider** — choose from `minimax`, `stepfun` (or both)
2. **Enter credentials** — API key, group_id (MiniMax only), base URL (with defaults)
3. **Set default provider** — which one to use when `--provider` is omitted
4. **Choose models** — for each capability, pick from known models or accept defaults

If config already exists, asks whether to update or reconfigure from scratch.

```
$ vox init

Welcome to vox! Let's set up your providers.

? Which providers do you want to configure? (multi-select)
  ◉ minimax
  ◯ stepfun

--- Configuring minimax ---
? API key: ****
? Group ID: 10086
? Base URL [https://api.minimax.chat/v1]:

--- Select models ---
? Chat model [MiniMax-M2.7]: MiniMax-M2.7
? Image model [image-01]: image-01
? Speech model [speech-2.8-hd]: speech-2.8-hd
? Video model [MiniMax-Hailuo-2.3]: MiniMax-Hailuo-2.3
? Music model [music-2.6]: music-2.6

? Set minimax as default provider? (Y/n): Y

✓ Config saved to ~/.config/vox/config.toml
  Provider: minimax (default)
  Run 'vox models list' to see all available models.
```

Non-interactive mode (for scripting):
```bash
vox init --provider minimax --api-key "sk-..." --group-id "10086" --yes
```

### `vox doctor`
```
Options:
  --provider <name>         Check specific provider (default: all configured)
  --fix                     Attempt to auto-fix issues where possible
```

Runs a diagnostic checklist and reports issues. Inspired by `brew doctor` / `npm doctor`.

**Checks performed:**

| # | Check | Pass | Warn | Fail |
|---|-------|------|------|------|
| 1 | Config file exists & is valid TOML | ✓ | — | ✗ (missing / parse error) |
| 2 | Default provider is set | ✓ | — | ✗ (no default, no providers configured) |
| 3 | Provider API key is set | ✓ | — | ✗ (missing api_key) |
| 4 | Provider-specific required fields are set | ✓ (all set) | — | ✗ (e.g., MiniMax missing group_id) |
| 5 | API key connectivity test | ✓ (200 OK) | — | ✗ (401/403/timeout) |
| 6 | Configured models are valid | ✓ | ⚠ (unknown model, may be new) | ✗ (empty where required) |
| 7 | Base URL is reachable | ✓ | ⚠ (slow >2s) | ✗ (DNS/connect error) |
| 8 | Output directory writable | ✓ | — | ✗ (permission denied) |

**Output example:**

```
$ vox doctor

Running diagnostics...

  ✓ Config file: ~/.config/vox/config.toml (valid)
  ✓ Default provider: minimax
  ✓ minimax API key: set (mmx-a***)
  ✓ minimax required fields: group_id set (10086)
  ✓ minimax connectivity: OK (238ms)
  ✓ minimax models: all valid
  ⚠ stepfun base URL: slow response (2.4s) — https://api.stepfun.com/v1
  ✗ stepfun API key: not set

  7 passed, 1 warning, 1 failed

  Issues:
    1. [WARN] stepfun base URL is slow (2.4s). Consider using a closer endpoint.
    2. [FAIL] stepfun has no API key configured.
       Fix: vox config set stepfun.api_key "sk-..."
       Or:  vox init --provider stepfun
```

With `--fix`:
- Creates config file if missing (then runs `vox init`)
- Prompts to re-enter missing API keys
- Resets unknown model values to provider defaults

Exit code: 0 if all pass, 1 if any fail, 2 if usage error.

### `vox models list`
```
Options:
  --provider <name>         Show models for specific provider (default: active provider)
  --capability <cap>        Filter to one capability: chat, image, speech, video, music, vision
  --all                     Show all providers side-by-side
```

Lists known models per capability for the given provider.

```
$ vox models list --provider stepfun

StepFun models:

  chat:
    step-1-8k, step-1-32k, step-1-128k, step-1-flash, step-2-16k, step-2-32k, step-3.5-flash
    Active: step-1-8k

  image:
    step-image-edit-2, step-2x-large, step-1x-medium
    Active: step-image-edit-2

  speech:
    step-tts-2, step-tts-mini, stepaudio-2.5-tts
    Active: step-tts-2

  vision:
    step-1v-8k (via chat multimodal)
    Active: step-1v-8k

  search:
    step-search
    Active: step-search

  Not supported: video, music
```

With `--all`:
```
$ vox models list --all --capability chat

Chat models:

  minimax:  MiniMax-M2.7 (active), MiniMax-M2.5, MiniMax-M2.1, MiniMax-M2
  stepfun:  step-1-8k (active), step-1-32k, step-1-128k, step-1-flash, step-2-16k, step-2-32k, step-3.5-flash
```

### `vox models set`
```
vox models set <capability> <model>    # uses default provider
vox models set <capability> <model> --provider <name>   # specific provider
```

Sets the active model for a capability on a provider. Validates the model against known list (warns but accepts if unknown).

```bash
# Set chat model for default provider
vox models set chat step-2-16k

# Set image model for specific provider
vox models set image image-01 --provider minimax

# Set speech model
vox models set speech speech-2.8-hd --provider minimax
```

Output:
```
$ vox models set chat step-2-16k
✓ stepfun.models.chat = step-2-16k
```

Capabilities: `chat`, `image`, `speech`, `video`, `music`, `vision`, `search`.

### `vox config show`
```
(no options — displays current config with masked API keys)
```

### `vox providers list`
```
(no options — lists all configured providers with masked keys and supported capabilities)
```

### `vox providers status`
```
Options:
  --provider <name>         Check specific provider (default: all configured)
```

Sends a lightweight API call (e.g. model list) to verify connectivity and auth.

### `vox completion`
```
vox completion bash       # Print bash completions to stdout
vox completion zsh        # Print zsh completions to stdout
vox completion fish       # Print fish completions to stdout
vox completion powershell # Print PowerShell completions to stdout
```

Generates shell completion scripts using clap's built-in completion support. Users source or install as appropriate for their shell.

```bash
# Bash — add to ~/.bashrc
echo 'eval "$(vox completion bash)"' >> ~/.bashrc

# Zsh — add to ~/.zshrc
echo 'eval "$(vox completion zsh)"' >> ~/.zshrc

# Fish
vox completion fish > ~/.config/fish/completions/vox.fish

# PowerShell — add to $PROFILE
vox completion powershell | Out-String | Invoke-Expression
```

---

## 5. Config Interface

### Two paths to configure

**Guided path** (new users, first-time setup):
```
vox init                    → interactive wizard: pick providers, enter creds, choose models
vox models list             → see what's available
vox models set chat step-2  → pick a model by name
```

**Power-user path** (scripting, CI, quick edits):
```
vox config set stepfun.api_key "sk-..."
vox config set minimax.models.chat MiniMax-M2.7
vox config set default_provider stepfun
```

Both write to the same TOML file. `models set` is a convenience wrapper around `config set <provider>.models.<capability>`.

### Config commands

### `vox config edit`
```
(no options — opens config file in $EDITOR)
```

Opens the config file in the user's default editor (`$EDITOR` or `$VISUAL`). If config doesn't exist, creates it with defaults first.

### `vox config get`
```
vox config get <key>    # print a single config value
```

Retrieves a single value by dotted key. Useful for scripting. Always prints the raw value (no masking) to stdout.

```bash
vox config get minimax.api_key         # → sk-... (full key, unmasked)
vox config get default_provider        # → minimax
vox config get stepfun.models.chat     # → step-1-8k
```

Exit code: 0 if key exists, 1 if key not found (with error message to stderr).

### `vox config set`
```
vox config set <key> <value>    # positional form (preferred)
vox config set --key <key> --value <value>   # explicit flag form
```

Power-user escape hatch for setting any config value directly via dotted key.

#### Full key schema

Every config key is a dotted path that maps 1:1 to the TOML structure:

```
# Global
default_provider                     "minimax" | "stepfun"
output_dir                           string (default: "./vox-output")

# Per-provider auth & endpoint
stepfun.api_key                      string
stepfun.base_url                     string
minimax.api_key                      string
minimax.group_id                     string
minimax.base_url                     string

# Per-provider, per-capability model selection
stepfun.models.chat                  e.g. "step-2-16k"
stepfun.models.image                 e.g. "step-image-edit-2"
stepfun.models.speech                e.g. "stepaudio-2.5-tts"
stepfun.models.vision                e.g. "step-1v-8k"
stepfun.models.search                e.g. "step-search"

minimax.models.chat                  e.g. "MiniMax-M2.7"
minimax.models.image                 e.g. "image-01"
minimax.models.speech                e.g. "speech-2.8-hd"
minimax.models.video                 e.g. "MiniMax-Hailuo-2.3"
minimax.models.music                 e.g. "music-2.6"
minimax.models.vision                e.g. "vision-01"

# Theme (TUI only, ignored in CLI mode)
theme.accent_color                   e.g. "#00bcd4"
theme.dark_mode                      true | false
```

#### Examples

```bash
# Switch default provider
vox config set default_provider stepfun

# Set API keys
vox config set stepfun.api_key "sk-new-key"
vox config set minimax.api_key "mmx-new-key"

# Change models per capability
vox config set stepfun.models.image step-image-edit-2
vox config set stepfun.models.speech stepaudio-2.5-tts
vox config set minimax.models.chat MiniMax-M2.7-highspeed
vox config set minimax.models.speech speech-2.8-turbo
vox config set minimax.models.video MiniMax-Hailuo-2.3-Fast

# Override base URL
vox config set minimax.base_url "https://api.minimaxi.com/v1"
```

The key is validated against the schema. Unknown keys produce an error with the list of valid keys. Model values are validated against the provider's known model list (warned but not rejected if unknown, to support new models without CLI updates).

### Config file

```toml
# Default provider when --provider is not given
default_provider = "minimax"

# Directory for saved files (images, videos, audio)
# Default: ./vox-output (created on first save)
output_dir = "./vox-output"

[stepfun]
api_key = "sk-..."
base_url = "https://api.stepfun.com/v1"    # optional override

[stepfun.models]
chat = "step-1-8k"
image = "step-image-edit-2"
speech = "step-tts-2"

[minimax]
api_key = "..."
group_id = "..."                            # required for MiniMax
base_url = "https://api.minimax.chat/v1"    # optional override

[minimax.models]
chat = "MiniMax-M2.7"
image = "image-01"
speech = "speech-2.8-hd"
video = "MiniMax-Hailuo-2.3"
music = "music-2.6"
vision = "vision-01"
```

### Config design: flat model values

Model selection in config uses simple string values, NOT nested `{default, available}` structs:

```toml
# ✅ Correct (spec design)
[minimax.models]
chat = "MiniMax-M2.7"
speech = "speech-2.8-hd"

# ❌ Not used (old code pattern)
[minimax.models.chat]
default = "MiniMax-M2.7"
available = ["MiniMax-M2.7", "MiniMax-M2.5", ...]
```

**Why:** Config stores *user preferences* (choices). Known model lists (options) live in code (`src/models.rs`), updated with each CLI release. Embedding available lists in config creates stale data as providers release new models.

### Config resolution for a command

When `vox image generate --prompt "cat"` runs:

1. **Provider**: `--provider` flag → `VOX_PROVIDER` env → `config.default_provider` → auto-detect (see below) → error
2. **Model**: `--model` flag → `VOX_MODEL` env → `config.<provider>.models.image` → provider's default
3. **API key**: `--api-key` flag → `VOX_API_KEY` env → `config.<provider>.api_key` → error
4. **Base URL**: `--base-url` flag → `VOX_BASE_URL` env → `config.<provider>.base_url` → built-in default

### Provider resolution (when `--provider` is not given)

```
1. VOX_PROVIDER env var       → use it
2. config.default_provider    → use it
3. Only 1 provider has API key configured → use it (implicit default)
4. Multiple configured, no default → error with guidance
```

**No hard-coded built-in default.** If we hard-coded "minimax", a StepFun-only user would hit errors until they configure the default. The "only 1 configured → use it" rule handles the single-provider case gracefully.

**Error when ambiguous:**

```
$ vox text chat --message "hello"
Error: No default provider set. Multiple providers are configured (minimax, stepfun).

  Run one of:
    vox config set default_provider minimax
    vox config set default_provider stepfun
  Or use the flag: vox text chat --provider minimax --message "hello"
```

**Capability-aware suggestion (not auto-switch):**

When the resolved provider doesn't support the requested capability, but another configured provider does:

```
$ vox video generate --prompt "ocean"
Error: default provider (stepfun) does not support video generation.

  minimax supports this capability.
  Run with: vox video generate --prompt "ocean" --provider minimax
  Or set:   vox config set default_provider minimax
```

The CLI does NOT auto-switch providers — that would be surprising behavior.

### Config file location

| Platform | Path |
|---|---|
| Linux/macOS | `~/.config/vox/config.toml` (respects `$XDG_CONFIG_HOME` if set) |
| Windows | `%APPDATA%\vox\config.toml` |
| Override | `VOX_CONFIG=/path/to/config.toml` env var |

### Output directory

Generated files (images, videos, audio) are saved to a configurable output directory.

**Resolution order:**
```
--output-dir flag → VOX_OUTPUT_DIR env → config.output_dir → "./vox-output"
```

**Directory structure:**
```
./vox-output/
├── images/
│   ├── image_20260518_143022.png
│   └── image_20260518_143105.png
├── videos/
│   └── video_20260518_143200.mp4
├── speech/
│   └── speech_20260518_143300.mp3
└── music/
    └── music_20260518_143400.mp3
```

**Naming convention:** `<type>_<YYYYMMDD>_<HHMMSS>.<ext>`

**Behavior:**
- Directory created on first file save (not at init)
- Subdirectories (`images/`, `videos/`, etc.) created per media type
- If `--out <path>` is specified on a command, it overrides the directory and naming — file saved exactly where specified
- If `--out` is omitted, file is saved to `output_dir/<type>/` with auto-generated name, and the path is printed to stdout
- No overwriting — if a file with the same name exists, append `_2`, `_3`, etc.
- **Atomic writes:** All file writes (config and media) use atomic write: write to temp file, then rename. This prevents partial files on crash.

### Capability gating

When a user runs a command the provider doesn't support:

```
$ vox --provider stepfun video generate --prompt "ocean waves"
Error: StepFun does not support video generation.

Supported capabilities for StepFun:
  text chat, image generate, speech synthesize, vision describe, search query

Use --provider minimax for video generation.
```

This is handled by a **capability registry** — a static map per provider:

```rust
struct ProviderCapabilities {
    chat: bool,
    image_generate: bool,
    speech_synthesize: bool,
    video_generate: bool,
    music_generate: bool,
    search: bool,
    vision: bool,
}
```

The CLI checks this **before** making any API call, giving instant feedback.

### `vox providers list` output

```
Configured providers:

  minimax (default)
    API Key:  mmx-a***
    Group ID: 10086
    Base URL: https://api.minimax.chat/v1
    Models:
      chat:    MiniMax-M2.7
      image:   image-01
      speech:  speech-2.8-hd
      video:   MiniMax-Hailuo-2.3
      music:   music-2.6
      vision:  vision-01
    Capabilities: chat, image, speech, video, music, search, vision

  stepfun
    API Key:  sk-s***
    Base URL: https://api.stepfun.com/v1
    Models:
      chat:    step-1-8k
      image:   step-image-edit-2
      speech:  step-tts-2
      vision:  (via chat multimodal — step-1v-8k)
    Capabilities: chat, image, speech, vision, search
    Not supported: video, music
```

### Codebase reconciliation notes

During implementation, the following gaps between this spec and current code must be resolved:

1. **StepFun search & vision**: Current `StepFunProvider` returns `Unsupported` for search and vision, but StepFun's API supports both. Implementation must add these methods to the StepFun client.

2. **Placeholder models**: Current default config has `stepfun.models.video = "step-video"` and `stepfun.models.music = "step-music"`. These must be removed — StepFun does not support video or music.

3. **Config structure migration**: Current code uses `CategoryModels { default, available }` nested struct. Must be flattened to simple string values matching this spec.

4. **Env var prefix**: Current code uses `MMX_*` prefix. Must be migrated to `VOX_*` with backward compat (see Migration section).

5. **AIProvider trait**: Current trait signature uses flat params. May need extending for image2image (`--image`), voice cloning (`--refer`), and provider-specific params added in this spec.

---

## 6. Output Formatting

Two output modes controlled by `--format`:

### `text` (default) — human-readable
- Progress/status to stderr
- Results to stdout
- Color when terminal supports it (disabled with `--no-color`)

### `json` — machine-readable
- All output as structured JSON to stdout
- Errors also JSON to stdout with `{"error": "...", "code": 2}`
- Useful for piping: `vox search query --query "cats" --format json | jq '.results[0].url'`

### `--quiet`
- Only essential output (result URLs, file paths)
- No progress indicators, no model info line

### `--verbose`
- Print HTTP request/response details to stderr
- Useful for debugging

---

## 7. Error Handling & Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General / API error |
| 2 | Usage error (missing flag, invalid value, unsupported capability) |

Errors always to stderr (in text mode). In JSON mode, errors as JSON to stdout.

---

## 8. Cross-Cutting Concerns

### Retry behavior

Transient HTTP errors (429 rate limit, 500/502/503 server errors) are retried automatically:

- **Max retries:** 3 (configurable via `VOX_MAX_RETRIES` env)
- **Backoff:** exponential with jitter — 1s, 2s, 4s (base)
- **429 specific:** respect `Retry-After` header if present
- **Not retried:** 400 (bad request), 401/403 (auth), 404, user cancellation

```
$ vox image generate --prompt "cat"
Error: API rate limited (429). Retrying in 2s... (attempt 2/3)
```

Retries are silent in `--quiet` mode, shown in text mode, and logged in `--verbose` mode.

### Signal handling

- **Ctrl+C during streaming chat:** Stop the stream immediately, print what was received so far. Exit 0 if any content was received, exit 130 (SIGINT) otherwise.
- **Ctrl+C during video polling:** Cancel the poll, print the task_id so user can resume later with `vox video status --task-id <id>`.
- **Ctrl+C during file upload:** Cancel and clean up any partial output file.

### Progress display

Long-running operations show progress to stderr:

| Operation | Progress indicator |
|---|---|
| Image generation | Spinner: `⠋ Generating image... (3.2s)` |
| Speech synthesis | Spinner + byte counter if `--stream` |
| Video generation + `--poll` | Status bar: `⠋ Polling task abc123... Queued → Processing (12s)` |
| Video download | Progress bar: `Downloading [████████░░] 80% 4.2MB/5.3MB` |
| File upload | Progress bar for files >1MB |

Progress is suppressed in `--quiet` mode and in `--format json` mode.

### Interactive chat mode

When `vox text chat` is run without `--message` and stdin is a TTY:

```
$ vox text chat
Entering interactive chat with MiniMax-M2.7 (minimax).
Type your message and press Enter. /exit or Ctrl+C to quit.

> What is Rust?
Rust is a systems programming language focused on safety, speed, and concurrency...

> How is it different from Go?
Unlike Go, Rust uses a borrow checker...
```

- Multi-turn: conversation history maintained in-memory for the session
- System prompt: `--system` flag if provided, otherwise no system prompt
- Chat commands: `/exit` or Ctrl+C to quit, `/clear` to reset conversation history, `/system <text>` to change system prompt.
- Not a full REPL — no tool use, no file access, just multi-turn chat

---

## 9. Implementation Scope

### Files to modify

| File | Change |
|------|--------|
| `src/cli.rs` | Restructure to nested `Resource::Action` subcommands with clap derive; add `Init`, `Models`, `Config`, `Provider` resources |
| `src/main.rs` | Extract `run_command()` dispatcher; add output formatting layer; add backward-compat aliases |
| `src/config.rs` | Add capability registry; add `config set` key validation; add `ProviderCapabilities` struct; add `init` wizard logic |
| `src/provider.rs` | Add `capabilities()` method to `AIProvider` trait; update `WorkResult` for CLI output |

### New files

| File | Purpose |
|------|---------|
| `src/output.rs` | Output formatting (text/json), stderr/stdout routing, color control |
| `src/capabilities.rs` | `ProviderCapabilities` struct and per-provider static maps |
| `src/models.rs` | Known model lists per provider per capability, validation helpers |

### No changes

- `src/stepfun.rs`, `src/minimax.rs` — API clients stay as-is
- `src/ui/*`, `src/app.rs`, `src/input.rs` — TUI code untouched (feature-gated)
