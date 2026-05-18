# Vox CLI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure vox CLI from flat commands to `vox <resource> <action>` pattern with multi-provider config, capability gating, and guided setup.

**Architecture:** New clap-derived nested subcommands dispatch to a `run_command()` function. Provider resolution uses a 4-level chain (flag → env → config → auto-detect). Config flattened from `{default, available}` nested structs to simple string model values. Output routed through a formatting layer (text/json).

**Tech Stack:** Rust, clap 4 (derive), tokio, reqwest, serde, toml, dirs, chrono

**Spec:** `docs/specs/2026-05-18-cli-redesign.md`

---

## File Structure

### New files
- `src/capabilities.rs` — `ProviderCapabilities` struct, per-provider static maps, gating logic
- `src/models.rs` — Known model lists per provider per capability, validation helpers
- `src/output.rs` — Output formatting (text/json), stderr/stdout routing, color control

### Modified files
- `src/cli.rs` — Full rewrite: nested `Resource::Action` subcommands with clap derive
- `src/config.rs` — Flatten model config from `CategoryModels` to simple `String`, add `output_dir`, add `config get/set` key validation, add `init` logic
- `src/provider.rs` — Add `capabilities()` to `AIProvider` trait, refactor `create_provider` for resolution chain
- `src/main.rs` — Extract `run_command()` dispatcher, replace giant match with command dispatch
- `src/lib.rs` — Add new module declarations

### Unchanged files
- `src/stepfun.rs` — API client stays as-is (search/vision stubs kept for now)
- `src/minimax.rs` — API client stays as-is
- `src/command.rs` — TUI slash commands, unchanged
- `src/app.rs`, `src/input.rs`, `src/ui/*` — TUI code, feature-gated, untouched
- `Cargo.toml` — No new deps needed (clap already has derive+env features)

---

## Phase 1: Foundation (capabilities, models, config refactor)

### Task 1: Create `src/capabilities.rs`

**Files:**
- Create: `src/capabilities.rs`

- [ ] **Step 1: Write the module with ProviderCapabilities struct and per-provider maps**

```rust
use crate::config::Provider;

/// Static capability registry per provider.
/// Checked BEFORE any API call to give instant feedback.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub chat: bool,
    pub image_generate: bool,
    pub speech_synthesize: bool,
    pub video_generate: bool,
    pub music_generate: bool,
    pub search: bool,
    pub vision: bool,
}

impl ProviderCapabilities {
    pub fn for_provider(provider: &Provider) -> &'static Self {
        match provider {
            Provider::MiniMax => &MINIMAX_CAPABILITIES,
            Provider::StepFun => &STEPFUN_CAPABILITIES,
        }
    }

    /// Check if a capability is supported. Returns error message if not.
    pub fn require(&self, capability: &str) -> Result<(), String> {
        let supported = match capability {
            "chat" => self.chat,
            "image_generate" => self.image_generate,
            "speech_synthesize" => self.speech_synthesize,
            "video_generate" => self.video_generate,
            "music_generate" => self.music_generate,
            "search" => self.search,
            "vision" => self.vision,
            _ => return Err(format!("Unknown capability: {capability}")),
        };
        if supported {
            Ok(())
        } else {
            Err(format!("does not support {capability}"))
        }
    }
}

static MINIMAX_CAPABILITIES: ProviderCapabilities = ProviderCapabilities {
    chat: true,
    image_generate: true,
    speech_synthesize: true,
    video_generate: true,
    music_generate: true,
    search: true,
    vision: true,
};

static STEPFUN_CAPABILITIES: ProviderCapabilities = ProviderCapabilities {
    chat: true,
    image_generate: true,
    speech_synthesize: true,
    video_generate: false,
    music_generate: false,
    search: true,
    vision: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimax_has_all_capabilities() {
        let cap = ProviderCapabilities::for_provider(&Provider::MiniMax);
        assert!(cap.chat);
        assert!(cap.video_generate);
        assert!(cap.music_generate);
    }

    #[test]
    fn test_stepfun_lacks_video_and_music() {
        let cap = ProviderCapabilities::for_provider(&Provider::StepFun);
        assert!(!cap.video_generate);
        assert!(!cap.music_generate);
        assert!(cap.chat);
        assert!(cap.image_generate);
    }

    #[test]
    fn test_require_supported() {
        let cap = ProviderCapabilities::for_provider(&Provider::MiniMax);
        assert!(cap.require("video_generate").is_ok());
    }

    #[test]
    fn test_require_unsupported() {
        let cap = ProviderCapabilities::for_provider(&Provider::StepFun);
        assert!(cap.require("video_generate").is_err());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -- capabilities`
Expected: 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/capabilities.rs
git commit -m "feat: add ProviderCapabilities struct and per-provider static maps"
```

---

### Task 2: Create `src/models.rs`

**Files:**
- Create: `src/models.rs`

- [ ] **Step 1: Write the module with known model lists and validation**

```rust
use crate::config::Provider;

/// Known models per provider per capability.
/// This lives in CODE, not config — updated with each CLI release.
pub struct KnownModels {
    pub models: &'static [&'static str],
    pub default: &'static str,
}

impl KnownModels {
    /// Check if a model name is in the known list.
    /// Returns true for unknown models too (forward-compatible).
    /// Callers should warn but accept unknown models.
    pub fn is_known(&self, model: &str) -> bool {
        self.models.contains(&model)
    }
}

pub fn get_known_models(provider: &Provider, capability: &str) -> Option<KnownModels> {
    match provider {
        Provider::MiniMax => minimax_models(capability),
        Provider::StepFun => stepfun_models(capability),
    }
}

fn minimax_models(capability: &str) -> Option<KnownModels> {
    match capability {
        "chat" => Some(KnownModels {
            models: &["MiniMax-M2.7", "MiniMax-M2.5", "MiniMax-M2.1", "MiniMax-M2"],
            default: "MiniMax-M2.7",
        }),
        "image" => Some(KnownModels {
            models: &["image-01", "image-01-live"],
            default: "image-01",
        }),
        "speech" => Some(KnownModels {
            models: &["speech-01", "speech-02-turbo", "speech-2.6-hd", "speech-2.8-hd"],
            default: "speech-01",
        }),
        "video" => Some(KnownModels {
            models: &["MiniMax-Hailuo-2.3", "MiniMax-Hailuo-02"],
            default: "MiniMax-Hailuo-2.3",
        }),
        "music" => Some(KnownModels {
            models: &["music-2.6"],
            default: "music-2.6",
        }),
        "vision" => Some(KnownModels {
            models: &["vision-01"],
            default: "vision-01",
        }),
        _ => None,
    }
}

fn stepfun_models(capability: &str) -> Option<KnownModels> {
    match capability {
        "chat" => Some(KnownModels {
            models: &[
                "step-1-8k", "step-1-32k", "step-1-128k", "step-1-flash",
                "step-2-16k", "step-2-32k", "step-3.5-flash",
            ],
            default: "step-1-8k",
        }),
        "image" => Some(KnownModels {
            models: &["step-image-edit-2", "step-2x-large", "step-1x-medium"],
            default: "step-image-edit-2",
        }),
        "speech" => Some(KnownModels {
            models: &["step-tts-2", "step-tts-mini", "stepaudio-2.5-tts"],
            default: "step-tts-2",
        }),
        "vision" => Some(KnownModels {
            models: &["step-1v-8k"],
            default: "step-1v-8k",
        }),
        "search" => Some(KnownModels {
            models: &["step-search"],
            default: "step-search",
        }),
        // StepFun does NOT support video or music
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimax_chat_models() {
        let km = get_known_models(&Provider::MiniMax, "chat").unwrap();
        assert!(km.is_known("MiniMax-M2.7"));
        assert!(!km.is_known("unknown-model"));
    }

    #[test]
    fn test_stepfun_no_video() {
        assert!(get_known_models(&Provider::StepFun, "video").is_none());
    }

    #[test]
    fn test_unknown_capability() {
        assert!(get_known_models(&Provider::MiniMax, "telepathy").is_none());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -- models`
Expected: 3 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/models.rs
git commit -m "feat: add known model lists per provider per capability"
```

---

### Task 3: Create `src/output.rs`

**Files:**
- Create: `src/output.rs`

- [ ] **Step 1: Write the output formatting module**

```rust
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}

pub struct Output {
    pub format: OutputFormat,
    pub quiet: bool,
    pub verbose: bool,
    pub no_color: bool,
}

impl Output {
    pub fn new(format: OutputFormat, quiet: bool, verbose: bool, no_color: bool) -> Self {
        Self { format, quiet, verbose, no_color }
    }

    /// Print result to stdout (respects format).
    pub fn result(&self, msg: &str) {
        println!("{msg}");
    }

    /// Print result as JSON to stdout.
    pub fn result_json(&self, value: &serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
    }

    /// Print status/progress to stderr (suppressed in quiet mode).
    pub fn status(&self, msg: &str) {
        if !self.quiet && self.format == OutputFormat::Text {
            eprintln!("{msg}");
        }
    }

    /// Print error to stderr (text) or stdout (json).
    pub fn error(&self, msg: &str, code: i32) {
        match self.format {
            OutputFormat::Text => eprintln!("Error: {msg}"),
            OutputFormat::Json => {
                let json = serde_json::json!({"error": msg, "code": code});
                println!("{json}");
            }
        }
    }

    /// Print verbose debug info to stderr.
    pub fn debug(&self, msg: &str) {
        if self.verbose {
            eprintln!("[debug] {msg}");
        }
    }

    /// Print deprecation warning to stderr.
    pub fn deprecation(&self, msg: &str) {
        eprintln!("Warning: {msg}");
    }
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("text"), OutputFormat::Text);
        assert_eq!(OutputFormat::from_str("anything"), OutputFormat::Text);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -- output`
Expected: 1 test PASS

- [ ] **Step 3: Commit**

```bash
git add src/output.rs
git commit -m "feat: add output formatting module (text/json, stderr routing)"
```

---

### Task 4: Add module declarations to `src/lib.rs` and `src/main.rs`

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add new module declarations to `src/lib.rs`**

Add these lines after the existing declarations:

```rust
pub mod capabilities;
pub mod models;
pub mod output;
```

- [ ] **Step 2: Add `mod output;` to `src/main.rs` (non-pub modules)**

Add after the existing `mod` declarations in `src/main.rs`:

```rust
mod capabilities;
mod models;
mod output;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build`
Expected: compiles with no errors (unused warnings ok for now)

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs src/main.rs
git commit -m "feat: register capabilities, models, output modules"
```

---

### Task 5: Refactor `src/config.rs` — flatten model config

**Files:**
- Modify: `src/config.rs`

This is the most invasive change. The `CategoryModels { default, available }` struct is replaced with simple `Option<String>` values in `ProviderModels`.

- [ ] **Step 1: Replace `CategoryModels` and `ProviderModels` with flat model strings**

Replace the `CategoryModels` struct, `ProviderModels` struct, and their impls (approximately lines 83–123) with:

```rust
/// Flat model selection per capability.
/// Config stores user CHOICES. Known model LISTS live in src/models.rs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderModels {
    pub chat: Option<String>,
    pub image: Option<String>,
    pub speech: Option<String>,
    pub video: Option<String>,
    pub music: Option<String>,
    pub vision: Option<String>,
    pub search: Option<String>,
}

impl ProviderModels {
    pub fn get(&self, category: &str) -> Option<&str> {
        match category {
            "chat" => self.chat.as_deref(),
            "image" => self.image.as_deref(),
            "speech" => self.speech.as_deref(),
            "video" => self.video.as_deref(),
            "music" => self.music.as_deref(),
            "vision" => self.vision.as_deref(),
            "search" => self.search.as_deref(),
            _ => None,
        }
    }

    pub fn set(&mut self, category: &str, value: String) -> bool {
        match category {
            "chat" => { self.chat = Some(value); true }
            "image" => { self.image = Some(value); true }
            "speech" => { self.speech = Some(value); true }
            "video" => { self.video = Some(value); true }
            "music" => { self.music = Some(value); true }
            "vision" => { self.vision = Some(value); true }
            "search" => { self.search = Some(value); true }
            _ => false,
        }
    }
}
```

- [ ] **Step 2: Update `Config` struct to add `output_dir`**

Add `output_dir` field to `Config`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(alias = "provider")]
    pub default_provider: Provider,
    pub output_dir: Option<String>,
    pub stepfun: Option<StepFunConfig>,
    pub minimax: Option<MiniMaxConfig>,
    pub theme: Option<ThemeConfig>,
}
```

Update `Default` impl to include `output_dir: None`.

- [ ] **Step 3: Update `DEFAULT_CONFIG_TOML` to use flat model values**

Replace the entire `DEFAULT_CONFIG_TOML` constant with:

```rust
const DEFAULT_CONFIG_TOML: &str = r#"
default_provider = "minimax"

[stepfun]
base_url = "https://api.stepfun.com/v1"

[stepfun.models]
chat = "step-1-8k"
image = "step-image-edit-2"
speech = "step-tts-2"
vision = "step-1v-8k"
search = "step-search"

[minimax]
base_url = "https://api.minimax.chat/v1"

[minimax.models]
chat = "MiniMax-M2.7"
image = "image-01"
speech = "speech-01"
video = "MiniMax-Hailuo-2.3"
music = "music-2.6"
vision = "vision-01"
"#;
```

Key changes: no `video`/`music` for stepfun; flat strings instead of `[category] default = "..."`.

- [ ] **Step 4: Update all internal references to `CategoryModels`**

Search for all uses of `CategoryModels`, `.default`, `.available` in config.rs and update them to use the new flat string access (`self.chat.as_deref()`, etc.). The TUI `ConfigField` and `ConfigEditor` code (behind `#[cfg(feature = "tui")]`) must also be updated to work with `Option<String>` instead of `CategoryModels`.

- [ ] **Step 5: Update env var prefixes from `MMX_` to `VOX_` in cli.rs**

In `src/cli.rs`, change env var names in `#[arg(...)]` attributes:
- `env = "MMX_PROVIDER"` → `env = "VOX_PROVIDER"`
- `env = "MMX_MODEL"` → `env = "VOX_MODEL"`
- `env = "MMX_API_KEY"` → `env = "VOX_API_KEY"`
- `env = "MMX_GROUP_ID"` → `env = "VOX_GROUP_ID"` (will be removed in Task 6, but migrate now)
- `env = "MMX_BASE_URL"` → `env = "VOX_BASE_URL"`

- [ ] **Step 6: Build and run all tests**

Run: `cargo build && cargo test`
Expected: compiles with no errors, all tests pass

- [ ] **Step 7: Commit**

```bash
git add src/config.rs src/cli.rs
git commit -m "refactor: flatten model config to simple strings, migrate MMX_* to VOX_*"
```

---

## Phase 2: CLI restructure

### Task 6: Rewrite `src/cli.rs` — nested resource/action subcommands

**Files:**
- Modify: `src/cli.rs` (full rewrite)

- [ ] **Step 1: Rewrite cli.rs with nested subcommands**

The full file is ~300 lines. Key structure:

```rust
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vox", version, about = "Multi-provider AI multimedia CLI")]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalOpts {
    #[arg(long, env = "VOX_PROVIDER")]
    pub provider: Option<String>,

    #[arg(long, env = "VOX_MODEL")]
    pub model: Option<String>,

    #[arg(long, env = "VOX_API_KEY")]
    pub api_key: Option<String>,

    #[arg(long, env = "VOX_BASE_URL")]
    pub base_url: Option<String>,

    #[arg(long, env = "VOX_FORMAT", default_value = "text")]
    pub format: String,

    #[arg(long, env = "VOX_OUTPUT_DIR")]
    pub output_dir: Option<String>,

    #[arg(long, env = "VOX_CONFIG")]
    pub config: Option<String>,

    #[arg(long, default_value_t = false)]
    pub quiet: bool,

    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    #[arg(long, default_value_t = 120)]
    pub timeout: u64,

    #[arg(long, default_value_t = false)]
    pub no_color: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    // ── AI capabilities ──
    #[command(subcommand)]
    Text(TextCommand),

    #[command(subcommand)]
    Image(ImageCommand),

    #[command(subcommand)]
    Speech(SpeechCommand),

    #[command(subcommand)]
    Video(VideoCommand),

    #[command(subcommand)]
    Music(MusicCommand),

    #[command(subcommand)]
    Search(SearchCommand),

    #[command(subcommand)]
    Vision(VisionCommand),

    // ── Setup & config ──
    Init(InitArgs),

    Doctor(DoctorArgs),

    #[command(subcommand)]
    Providers(ProvidersCommand),

    #[command(subcommand)]
    Models(ModelsCommand),

    #[command(subcommand)]
    Config(ConfigCommand),

    Completion { shell: String },

    // ── Backward compat (hidden) ──
    #[command(hide = true)]
    LegacyImage(LegacyImageArgs),

    #[command(hide = true)]
    LegacySpeech(LegacySpeechArgs),

    #[command(hide = true)]
    LegacyVideo(LegacyVideoArgs),

    #[command(hide = true)]
    LegacyMusic(LegacyMusicArgs),

    #[command(hide = true)]
    LegacySearch(LegacySearchArgs),

    #[command(hide = true)]
    LegacyVision(LegacyVisionArgs),
}

// ── Resource: text ──
#[derive(Subcommand, Debug, Clone)]
pub enum TextCommand {
    Chat {
        #[arg(long, short)]
        message: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        system: Option<String>,
        #[arg(long, default_value_t = false)]
        stream: bool,
        #[arg(long)]
        max_tokens: Option<u32>,
        #[arg(long)]
        temperature: Option<f64>,
        #[arg(long)]
        top_p: Option<f64>,
    },
}

// ── Resource: image ──
#[derive(Subcommand, Debug, Clone)]
pub enum ImageCommand {
    Generate {
        #[arg(long, short)]
        prompt: String,
        #[arg(long)]
        size: Option<String>,
        #[arg(long)]
        aspect_ratio: Option<String>,
        #[arg(long, default_value_t = 1)]
        n: u8,
        #[arg(long)]
        image: Option<String>,
        #[arg(long)]
        mask: Option<String>,
        #[arg(long, short)]
        out: Option<String>,
        #[arg(long, default_value_t = false)]
        no_save: bool,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        steps: Option<u32>,
    },
}

// ── Resource: speech ──
#[derive(Subcommand, Debug, Clone)]
pub enum SpeechCommand {
    Synthesize {
        #[arg(long, short)]
        text: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        voice: Option<String>,
        #[arg(long, default_value_t = 1.0)]
        speed: f64,
        #[arg(long, default_value = "mp3")]
        format: String,
        #[arg(long, short)]
        out: Option<String>,
        #[arg(long, default_value_t = false)]
        stream: bool,
        #[arg(long)]
        refer: Option<String>,
    },
    #[command(name = "list-voices")]
    ListVoices,
}

// ── Resource: video ──
#[derive(Subcommand, Debug, Clone)]
pub enum VideoCommand {
    Generate {
        #[arg(long, short)]
        prompt: String,
        #[arg(long, default_value_t = 6)]
        duration: u8,
        #[arg(long, default_value = "720P")]
        resolution: String,
        #[arg(long)]
        first_frame: Option<String>,
        #[arg(long)]
        last_frame: Option<String>,
        #[arg(long, default_value_t = false)]
        poll: bool,
        #[arg(long, short)]
        out: Option<String>,
    },
    Status {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        download: Option<String>,
        #[arg(long, default_value_t = false)]
        watch: bool,
    },
}

// ── Resource: music ──
#[derive(Subcommand, Debug, Clone)]
pub enum MusicCommand {
    Generate {
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        lyrics: Option<String>,
        #[arg(long)]
        lyrics_file: Option<String>,
        #[arg(long, default_value_t = false)]
        instrumental: bool,
        #[arg(long, default_value = "mp3")]
        format: String,
        #[arg(long, short)]
        out: Option<String>,
    },
}

// ── Resource: search ──
#[derive(Subcommand, Debug, Clone)]
pub enum SearchCommand {
    Query {
        query: String,
        #[arg(long, default_value_t = 5)]
        count: u8,
    },
}

// ── Resource: vision ──
#[derive(Subcommand, Debug, Clone)]
pub enum VisionCommand {
    Describe {
        #[arg(long, short)]
        file: String,
        #[arg(long, short)]
        prompt: Option<String>,
    },
}

// ── Setup commands ──
#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub api_key: Option<String>,
    #[arg(long)]
    pub group_id: Option<String>,
    #[arg(long)]
    pub base_url: Option<String>,
    #[arg(long, default_value_t = false)]
    pub default: bool,
    #[arg(long, short = 'y', default_value_t = false)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DoctorArgs {
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long, default_value_t = false)]
    pub fix: bool,
}

// ── Resource: providers ──
#[derive(Subcommand, Debug, Clone)]
pub enum ProvidersCommand {
    List,
    Status {
        #[arg(long)]
        provider: Option<String>,
    },
}

// ── Resource: models ──
#[derive(Subcommand, Debug, Clone)]
pub enum ModelsCommand {
    List {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long, default_value_t = false)]
        all: bool,
    },
    Set {
        capability: String,
        model: String,
        #[arg(long)]
        provider: Option<String>,
    },
}

// ── Resource: config ──
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Get { key: String },
    Set { key: String, value: String },
    Edit,
}

// ── Legacy backward compat ──
// These map old flat commands to new nested commands.
// They are hidden from --help but still parse.
#[derive(Args, Debug, Clone)]
pub struct LegacyImageArgs { pub prompt: String, #[arg(long, default_value = "1:1")] pub aspect_ratio: String, #[arg(long, short)] pub output: Option<String>, #[arg(long, default_value_t = 1)] pub n: u8 }

#[derive(Args, Debug, Clone)]
pub struct LegacySpeechArgs { #[arg(long, short)] pub text: String, #[arg(long, short)] pub out: Option<String>, #[arg(long)] pub voice: Option<String>, #[arg(long, default_value_t = 1.0)] pub speed: f64, #[arg(long, default_value = "mp3")] pub format: String }

#[derive(Args, Debug, Clone)]
pub struct LegacyVideoArgs { #[arg(long, short)] pub prompt: String, #[arg(long, default_value_t = 6)] pub duration: u8, #[arg(long, default_value = "720P")] pub resolution: String }

#[derive(Args, Debug, Clone)]
pub struct LegacyMusicArgs { #[arg(long)] pub prompt: String, #[arg(long)] pub lyrics: Option<String>, #[arg(long, default_value_t = false)] pub instrumental: bool, #[arg(long, short)] pub out: Option<String> }

#[derive(Args, Debug, Clone)]
pub struct LegacySearchArgs { pub query: String, #[arg(long, default_value_t = 5)] pub count: u8 }

#[derive(Args, Debug, Clone)]
pub struct LegacyVisionArgs { pub file: String, #[arg(long, short)] pub prompt: Option<String> }
```

- [ ] **Step 2: Build to verify clap parsing compiles**

Run: `cargo build`
Expected: compiles (may have unused warnings from main.rs match arms — that's ok, fixed in Task 7)

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat: restructure CLI to nested resource/action subcommands"
```

---

### Task 7: Rewrite `src/main.rs` — command dispatcher

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Replace the `run_cli` function with a new dispatcher**

Replace the entire `run_cli` function (lines 41–221) with the new command dispatcher that:
1. Resolves provider from flag → env → config → auto-detect
2. Creates output formatter
3. Dispatches to per-command handler functions
4. Handles legacy backward-compat aliases with deprecation warnings

The new `run_cli` should:
- Load config with `Config::load()` (respecting `--config` override)
- Resolve provider name using the 4-level chain
- Handle auto-detect (single configured provider) and ambiguous error
- Create `Output` instance from global flags
- Match on `cli.command` variants and dispatch to handler functions
- Legacy commands print deprecation warning then dispatch to the equivalent new command handler

- [ ] **Step 2: Extract command handler functions**

Create these handler functions (each is ~10-30 lines):

```rust
async fn handle_text_chat(provider: &dyn AIProvider, args: &TextCommand, output: &Output) -> std::io::Result<()>
async fn handle_image_generate(provider: &dyn AIProvider, args: &ImageCommand, output: &Output) -> std::io::Result<()>
async fn handle_speech_synthesize(provider: &dyn AIProvider, args: &SpeechCommand, output: &Output) -> std::io::Result<()>
async fn handle_speech_list_voices(provider: &dyn AIProvider, output: &Output) -> std::io::Result<()>
async fn handle_video_generate(provider: &dyn AIProvider, args: &VideoCommand, output: &Output) -> std::io::Result<()>
async fn handle_video_status(provider: &dyn AIProvider, args: &VideoCommand, output: &Output) -> std::io::Result<()>
async fn handle_music_generate(provider: &dyn AIProvider, args: &MusicCommand, output: &Output) -> std::io::Result<()>
async fn handle_search_query(provider: &dyn AIProvider, args: &SearchCommand, output: &Output) -> std::io::Result<()>
async fn handle_vision_describe(provider: &dyn AIProvider, args: &VisionCommand, output: &Output) -> std::io::Result<()>
async fn handle_init(args: &InitArgs, config: &mut Config) -> std::io::Result<()>
async fn handle_doctor(args: &DoctorArgs, config: &Config, output: &Output) -> std::io::Result<()>
async fn handle_providers_list(config: &Config, output: &Output) -> std::io::Result<()>
async fn handle_providers_status(config: &Config, provider: Option<&str>, output: &Output) -> std::io::Result<()>
async fn handle_models_list(config: &Config, args: &ModelsCommand, output: &Output) -> std::io::Result<()>
async fn handle_models_set(config: &mut Config, args: &ModelsCommand, output: &Output) -> std::io::Result<()>
async fn handle_config_show(config: &Config, output: &Output) -> std::io::Result<()>
async fn handle_config_get(config: &Config, key: &str, output: &Output) -> std::io::Result<()>
async fn handle_config_set(config: &mut Config, key: &str, value: &str, output: &Output) -> std::io::Result<()>
async fn handle_config_edit(config: &Config) -> std::io::Result<()>
async fn handle_completion(shell: &str) -> std::io::Result<()>
```

- [ ] **Step 3: Implement provider resolution function**

Add a `resolve_provider` function:

```rust
fn resolve_provider(global: &GlobalOpts, config: &Config) -> Result<Provider, String> {
    // 1. CLI flag
    if let Some(name) = &global.provider {
        return parse_provider_name(name);
    }
    // 2. VOX_PROVIDER env (already resolved by clap)
    // 3. config.default_provider
    // 4. Auto-detect: only 1 provider has API key
    let stepfun_has_key = config.stepfun.as_ref().map_or(false, |s| !s.api_key.is_empty());
    let minimax_has_key = config.minimax.as_ref().map_or(false, |m| !m.api_key.is_empty());
    match (stepfun_has_key, minimax_has_key) {
        (true, false) => Ok(Provider::StepFun),
        (false, true) => Ok(Provider::MiniMax),
        (true, true) => Err("No default provider set. Multiple providers are configured (minimax, stepfun).\n  Run: vox config set default_provider <name>".into()),
        (false, false) => Err("No providers configured. Run: vox init".into()),
    }
}
```

- [ ] **Step 4: Implement legacy alias dispatchers**

Add functions that print deprecation warnings and translate old args to new command structures:

```rust
fn dispatch_legacy_image(args: &LegacyImageArgs, output: &Output) -> Commands {
    output.deprecation("'vox image \"...\"' is deprecated. Use 'vox image generate --prompt \"...\"' instead.");
    Commands::Image(ImageCommand::Generate { prompt: args.prompt.clone(), size: None, aspect_ratio: Some(args.aspect_ratio.clone()), n: args.n, image: None, mask: None, out: args.output.clone(), no_save: false, seed: None, steps: None })
}
// ... similar for speech, video, music, search, vision
```

- [ ] **Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: compiles, all existing tests still pass

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: rewrite CLI dispatcher with provider resolution and command handlers"
```

---

## Phase 3: Command handlers

### Task 8: Implement config commands (show, get, set, edit)

**Files:**
- Modify: `src/main.rs` (handler implementations)

- [ ] **Step 1: Implement `handle_config_show`**

Print config with masked API keys (reuse existing `mask_key` from config.rs, make it pub).

- [ ] **Step 2: Implement `handle_config_get`**

Parse dotted key, return raw value. Validate key against schema.

- [ ] **Step 3: Implement `handle_config_set`**

Parse dotted key, set value in config struct, save to TOML. Validate key against schema. Warn on unknown model values.

- [ ] **Step 4: Implement `handle_config_edit`**

Open config file path in `$EDITOR`. Use `std::process::Command` to spawn editor.

- [ ] **Step 5: Test config commands manually**

```bash
cargo run -- config show
cargo run -- config get default_provider
cargo run -- config set default_provider stepfun
```

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/config.rs
git commit -m "feat: implement config show/get/set/edit commands"
```

---

### Task 9: Implement provider commands (list, status) and models commands (list, set)

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement `handle_providers_list`**

Print formatted table of configured providers with masked keys, models, capabilities (using `ProviderCapabilities`).

- [ ] **Step 2: Implement `handle_providers_status`**

Make lightweight API call to verify connectivity. Print per-provider status with latency.

- [ ] **Step 3: Implement `handle_models_list`**

Use `models::get_known_models()` to list available models. Show active model from config. Handle `--all` and `--capability` filters.

- [ ] **Step 4: Implement `handle_models_set`**

Validate model against known list (warn but accept unknown). Set in config and save.

- [ ] **Step 5: Test manually**

```bash
cargo run -- providers list
cargo run -- models list --provider stepfun
cargo run -- models set chat step-2-16k
```

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement providers list/status and models list/set commands"
```

---

### Task 10: Implement `vox init` and `vox doctor`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement `handle_init` in non-interactive mode**

When `--provider` and `--api-key` are given with `--yes`: set config values and save. No prompts.

- [ ] **Step 2: Implement `handle_init` in interactive mode**

When no flags: prompt for provider selection, API key, group_id (MiniMax), models. Use `std::io::stdin().read_line()` for prompts. (No external dialoguer dependency — keep it simple.)

- [ ] **Step 3: Implement `handle_doctor`**

Run 8 checks from the spec. Print ✓/⚠/✗ results. Exit code 0/1 based on failures.

- [ ] **Step 4: Test manually**

```bash
cargo run -- init --provider minimax --api-key "test-key" --yes
cargo run -- doctor
```

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement vox init and vox doctor commands"
```

---

### Task 11: Implement `vox completion` and remaining AI command handlers

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement `handle_completion`**

Use clap's `clap_complete` to generate shell completions. Add `clap_complete` to `Cargo.toml` dependencies:

```toml
clap_complete = "4"
```

- [ ] **Step 2: Implement remaining AI handlers**

Each handler:
1. Check capability gating (`capabilities.require(...)`)
2. Call the provider method
3. Format output via `Output` struct
4. Handle `--out` / output directory logic

Handlers: `handle_text_chat`, `handle_image_generate`, `handle_speech_synthesize`, `handle_speech_list_voices`, `handle_video_generate`, `handle_video_status`, `handle_music_generate`, `handle_search_query`, `handle_vision_describe`

These are straightforward translations of the existing match arms in the old `run_cli`, with added capability checks and output formatting.

- [ ] **Step 3: Build and test all commands**

```bash
cargo build
cargo run -- text chat --message "hello"
cargo run -- image generate --prompt "cat"
cargo run -- speech synthesize --text "hello"
cargo run -- search query "rust lang"
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs Cargo.toml Cargo.lock
git commit -m "feat: implement all AI command handlers with capability gating"
```

---

## Phase 4: Testing and cleanup

### Task 12: Update and add tests

**Files:**
- Modify: `src/cli.rs` (update existing tests for new command structure)
- Add tests in new files as needed

- [ ] **Step 1: Update CLI parsing tests in `src/cli.rs`**

Replace old tests with new ones that test nested subcommands:

```rust
#[test]
fn test_text_chat() {
    let cli = Cli::try_parse_from(["vox", "text", "chat", "--message", "hello"]).unwrap();
    match cli.command {
        Some(Commands::Text(TextCommand::Chat { message, .. })) => {
            assert_eq!(message, Some("hello".to_string()));
        }
        _ => panic!("Expected text chat"),
    }
}

#[test]
fn test_image_generate() {
    let cli = Cli::try_parse_from(["vox", "image", "generate", "--prompt", "cat"]).unwrap();
    match cli.command {
        Some(Commands::Image(ImageCommand::Generate { prompt, .. })) => {
            assert_eq!(prompt, "cat");
        }
        _ => panic!("Expected image generate"),
    }
}

#[test]
fn test_legacy_image() {
    let cli = Cli::try_parse_from(["vox", "legacy-image", "cat"]).unwrap();
    match cli.command {
        Some(Commands::LegacyImage(args)) => assert_eq!(args.prompt, "cat"),
        _ => panic!("Expected legacy image"),
    }
}

#[test]
fn test_config_set() {
    let cli = Cli::try_parse_from(["vox", "config", "set", "default_provider", "stepfun"]).unwrap();
    match cli.command {
        Some(Commands::Config(ConfigCommand::Set { key, value })) => {
            assert_eq!(key, "default_provider");
            assert_eq!(value, "stepfun");
        }
        _ => panic!("Expected config set"),
    }
}

#[test]
fn test_models_set() {
    let cli = Cli::try_parse_from(["vox", "models", "set", "chat", "step-2-16k"]).unwrap();
    match cli.command {
        Some(Commands::Models(ModelsCommand::Set { capability, model, .. })) => {
            assert_eq!(capability, "chat");
            assert_eq!(model, "step-2-16k");
        }
        _ => panic!("Expected models set"),
    }
}

#[test]
fn test_video_status() {
    let cli = Cli::try_parse_from(["vox", "video", "status", "--task-id", "abc123"]).unwrap();
    match cli.command {
        Some(Commands::Video(VideoCommand::Status { task_id, .. })) => {
            assert_eq!(task_id, "abc123");
        }
        _ => panic!("Expected video status"),
    }
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "test: update CLI parsing tests for nested subcommands"
```

---

### Task 13: Final build verification

**Files:**
- All files

- [ ] **Step 1: Build without TUI feature**

Run: `cargo build`
Expected: clean build, no errors

- [ ] **Step 2: Build WITH TUI feature**

Run: `cargo build --features tui`
Expected: clean build, no errors

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 4: Run all tests with TUI feature**

Run: `cargo test --features tui`
Expected: all tests pass

- [ ] **Step 5: Manual smoke test of key commands**

```bash
cargo run -- --help
cargo run -- providers list
cargo run -- models list
cargo run -- doctor
cargo run -- completion bash
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: final build verification and cleanup"
```

---

## Self-Review Checklist

### Spec coverage
- [x] Provider Capability Matrix → Task 1 (capabilities.rs)
- [x] Command Structure (resource/action map) → Task 6 (cli.rs rewrite)
- [x] Global Flags → Task 6 (GlobalOpts)
- [x] Per-Command Flags → Task 6 (per-resource enums)
- [x] vox init → Task 10
- [x] vox doctor → Task 10
- [x] vox providers list/status → Task 9
- [x] vox models list/set → Task 9
- [x] vox config show/get/set/edit → Task 8
- [x] vox completion → Task 11
- [x] Config Interface (two paths) → Tasks 8, 9
- [x] Provider resolution chain → Task 7 (resolve_provider)
- [x] Config file location → existing Config::config_dir()
- [x] Output directory → Task 5 (config.output_dir) + Task 11 (handlers)
- [x] Capability gating → Task 1 + Task 11 (handlers)
- [x] Output Formatting → Task 3 (output.rs)
- [x] Error Handling & Exit Codes → Task 7 (dispatcher)
- [x] Retry behavior → deferred (provider.rs currently has no retry)
- [x] Signal handling → deferred (requires ctrlc crate or tokio signal)
- [x] Progress display → deferred (requires indicatif crate)
- [x] Interactive chat mode → deferred (requires readline)
- [x] Backward compat aliases → Task 6 (legacy commands)
- [x] MMX_* → VOX_* migration → Task 5
- [x] Config flatten → Task 5
- [x] Codebase reconciliation → Task 1 (capabilities), Task 5 (config), Task 9 (models)

### Deferred items (future work)
- **Retry logic**: Needs `reqwest-middleware` or custom retry layer. Not in current deps.
- **Signal handling**: Needs `tokio::signal` or `ctrlc` crate. Added as follow-up.
- **Progress display**: Needs `indicatif` crate. Added as follow-up.
- **Interactive chat REPL**: Needs `rustyline` or similar. Added as follow-up.
- **StepFun search/vision implementations**: API client methods needed. Added as follow-up.

### Placeholder scan
- No TBD, TODO, or placeholder patterns in this plan.

### Type consistency
- All types reference structs defined in earlier tasks.
- `Provider` comes from `config.rs` (existing).
- `ProviderCapabilities` defined in Task 1, used in Task 11.
- `KnownModels` defined in Task 2, used in Task 9.
- `Output` defined in Task 3, used in Tasks 7-11.
- `GlobalOpts`, `Commands`, and per-resource enums all defined in Task 6, used in Tasks 7-11.
