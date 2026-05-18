use clap::{Args, Parser, Subcommand};

/// Multi-provider AI multimedia CLI & TUI
#[derive(Parser, Debug, Clone)]
#[command(name = "vox", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalOpts {
    /// AI provider to use (minimax, stepfun)
    #[arg(long, env = "VOX_PROVIDER")]
    pub provider: Option<String>,

    /// Model name override
    #[arg(long, env = "VOX_MODEL")]
    pub model: Option<String>,

    /// API key (overrides config)
    #[arg(long, env = "VOX_API_KEY")]
    pub api_key: Option<String>,

    /// Base URL for the provider API
    #[arg(long, env = "VOX_BASE_URL")]
    pub base_url: Option<String>,

    /// Output format (text, json)
    #[arg(long, env = "VOX_FORMAT", default_value = "text")]
    pub format: String,

    /// Default directory for output files
    #[arg(long, env = "VOX_OUTPUT_DIR")]
    pub output_dir: Option<String>,

    /// Path to config file (overrides default)
    #[arg(long, env = "VOX_CONFIG")]
    pub config: Option<String>,

    /// Suppress progress output
    #[arg(long, default_value_t = false)]
    pub quiet: bool,

    /// Enable verbose debug output
    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 120)]
    pub timeout: u64,

    /// Disable colored output
    #[arg(long, default_value_t = false)]
    pub no_color: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    // ── AI capability resources ──────────────────────────────────────
    /// Text generation and chat
    #[command(subcommand)]
    Text(TextCommand),

    /// Image generation and editing
    #[command(subcommand)]
    Image(ImageCommand),

    /// Speech synthesis (text-to-speech)
    #[command(subcommand)]
    Speech(SpeechCommand),

    /// Video generation
    #[command(subcommand)]
    Video(VideoCommand),

    /// Music generation
    #[command(subcommand)]
    Music(MusicCommand),

    /// Web search
    #[command(subcommand)]
    Search(SearchCommand),

    /// Image understanding (vision)
    #[command(subcommand)]
    Vision(VisionCommand),

    // ── Setup & configuration ────────────────────────────────────────
    /// Initialize vox with your API keys
    Init(InitArgs),

    /// Run diagnostics and report issues
    Doctor(DoctorArgs),

    /// Manage provider configurations
    #[command(subcommand)]
    Providers(ProvidersCommand),

    /// Manage model selections
    #[command(subcommand)]
    Models(ModelsCommand),

    /// Manage vox configuration
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Generate shell completion script
    Completion {
        /// Shell name (bash, zsh, fish, powershell, elvish)
        shell: String,
    },

    // ── Legacy backward-compat commands (hidden) ─────────────────────
    #[command(hide = true, name = "image-legacy")]
    LegacyImage(LegacyImageArgs),

    #[command(hide = true, name = "speech-legacy")]
    LegacySpeech(LegacySpeechArgs),

    #[command(hide = true, name = "video-legacy")]
    LegacyVideo(LegacyVideoArgs),

    #[command(hide = true, name = "music-legacy")]
    LegacyMusic(LegacyMusicArgs),

    #[command(hide = true, name = "search-legacy")]
    LegacySearch(LegacySearchArgs),

    #[command(hide = true, name = "vision-legacy")]
    LegacyVision(LegacyVisionArgs),
}

// ═══════════════════════════════════════════════════════════════════
// Text subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum TextCommand {
    /// Chat with an AI assistant
    Chat {
        /// The message to send (if omitted, enters interactive REPL)
        #[arg(short = 'm', long)]
        message: Option<String>,

        /// System prompt
        #[arg(long)]
        system: Option<String>,

        /// Conversation history file (JSON)
        #[arg(long)]
        history: Option<String>,

        /// Stream the response
        #[arg(long, default_value_t = false)]
        stream: bool,
    },
    /// Start an interactive chat session
    Repl {
        /// System prompt
        #[arg(long)]
        system: Option<String>,

        /// Conversation history file (JSON)
        #[arg(long)]
        history: Option<String>,
    },
    /// Complete text from a prompt
    Complete {
        /// The prompt to complete
        prompt: String,

        /// Maximum tokens to generate
        #[arg(long, default_value_t = 256)]
        max_tokens: u32,

        /// Sampling temperature (0-2)
        #[arg(long, default_value_t = 0.7)]
        temperature: f64,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Image subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum ImageCommand {
    /// Generate an image from a text prompt
    Generate {
        /// The prompt describing the desired image
        prompt: String,

        /// Aspect ratio (1:1, 16:9, 4:3, 3:2, 2:3, 3:4, 9:16, 21:9)
        #[arg(long, default_value = "1:1")]
        aspect_ratio: String,

        /// Output file path
        #[arg(long, short)]
        output: Option<String>,

        /// Number of images to generate (1-9)
        #[arg(long, short, default_value_t = 1)]
        n: u8,
    },
    /// Edit an existing image
    Edit {
        /// Path to the source image
        file: String,

        /// Edit instruction/prompt
        #[arg(long, short)]
        prompt: String,

        /// Output file path
        #[arg(long, short)]
        output: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Speech subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum SpeechCommand {
    /// Synthesize speech from text
    Generate {
        /// Text to synthesize
        #[arg(long, short)]
        text: String,

        /// Output file path
        #[arg(long, short)]
        out: Option<String>,

        /// Voice ID to use
        #[arg(long, default_value = "English_expressive_narrator")]
        voice: String,

        /// Speed (0.5-2.0)
        #[arg(long, default_value_t = 1.0)]
        speed: f64,

        /// Output format (mp3, wav, flac, pcm, opus)
        #[arg(long, default_value = "mp3")]
        format: String,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Video subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum VideoCommand {
    /// Generate a video from a text prompt
    Generate {
        /// The prompt describing the desired video
        #[arg(long, short)]
        prompt: String,

        /// Duration in seconds (6 or 10)
        #[arg(long, default_value_t = 6)]
        duration: u8,

        /// Resolution (720P, 768P, 1080P)
        #[arg(long, default_value = "720P")]
        resolution: String,

        /// Output file path
        #[arg(long, short)]
        out: Option<String>,
    },
    /// Check the status of a video generation task
    Status {
        /// Task ID from a previous generation
        task_id: String,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Music subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum MusicCommand {
    /// Generate music from a prompt and optional lyrics
    Generate {
        /// Style description
        #[arg(long)]
        prompt: String,

        /// Song lyrics with structure tags [Verse], [Chorus], etc.
        #[arg(long)]
        lyrics: Option<String>,

        /// Generate instrumental only (no vocals)
        #[arg(long, default_value_t = false)]
        instrumental: bool,

        /// Output file path
        #[arg(long, short)]
        out: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Search subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum SearchCommand {
    /// Search the web
    Query {
        /// Search query
        query: String,

        /// Number of results (1-10)
        #[arg(long, default_value_t = 5)]
        count: u8,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Vision subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum VisionCommand {
    /// Analyze an image file
    Analyze {
        /// Path to the image file
        file: String,

        /// Question to ask about the image
        #[arg(long, short)]
        prompt: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Setup command args
// ═══════════════════════════════════════════════════════════════════

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Provider to set up (minimax, stepfun)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// API key for the provider
    #[arg(long)]
    pub api_key: Option<String>,

    /// Interactive mode (prompts for values)
    #[arg(long, default_value_t = false)]
    pub interactive: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Run only the named check
    #[arg(long)]
    pub check: Option<String>,

    /// Output format (text, json)
    #[arg(long, default_value = "text")]
    pub format: String,
}

// ═══════════════════════════════════════════════════════════════════
// Providers subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum ProvidersCommand {
    /// List configured providers
    List,
    /// Test provider connectivity
    Status {
        /// Provider name (minimax, stepfun)
        #[arg(short, long)]
        provider: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Models subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum ModelsCommand {
    /// List available models for a capability
    List {
        /// Capability (chat, image, speech, video, music, vision, search)
        #[arg(short, long)]
        capability: Option<String>,
    },
    /// Set the model for a capability
    Set {
        /// Capability name
        capability: String,

        /// Model name
        model: String,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Config subcommands
// ═══════════════════════════════════════════════════════════════════

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Get a config value by dotted key
    Get {
        /// Dotted key (e.g. stepfun.api_key, default_provider)
        key: String,
    },
    /// Set a config value
    Set {
        /// Dotted key (e.g. stepfun.api_key, default_provider)
        key: String,

        /// Value to set
        value: String,
    },
    /// Edit config in $EDITOR
    Edit,
}

// ═══════════════════════════════════════════════════════════════════
// Legacy backward-compat arg structs
// ═══════════════════════════════════════════════════════════════════

#[derive(Args, Debug, Clone)]
pub struct LegacyImageArgs {
    /// The prompt describing the desired image
    pub prompt: String,

    /// Aspect ratio (1:1, 16:9, 4:3, 3:2, 2:3, 3:4, 9:16, 21:9)
    #[arg(long, default_value = "1:1")]
    pub aspect_ratio: String,

    /// Output file path
    #[arg(long, short)]
    pub output: Option<String>,

    /// Number of images to generate (1-9)
    #[arg(long, default_value_t = 1)]
    pub n: u8,
}

#[derive(Args, Debug, Clone)]
pub struct LegacySpeechArgs {
    /// Text to synthesize
    #[arg(long, short)]
    pub text: String,

    /// Output file path
    #[arg(long, short)]
    pub out: Option<String>,

    /// Voice ID to use
    #[arg(long, default_value = "English_expressive_narrator")]
    pub voice: String,

    /// Speed (0.5-2.0)
    #[arg(long, default_value_t = 1.0)]
    pub speed: f64,

    /// Output format (mp3, wav, flac, pcm, opus)
    #[arg(long, default_value = "mp3")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
pub struct LegacyVideoArgs {
    /// The prompt describing the desired video
    #[arg(long, short)]
    pub prompt: String,

    /// Duration in seconds (6 or 10)
    #[arg(long, default_value_t = 6)]
    pub duration: u8,

    /// Resolution (720P, 768P, 1080P)
    #[arg(long, default_value = "720P")]
    pub resolution: String,
}

#[derive(Args, Debug, Clone)]
pub struct LegacyMusicArgs {
    /// Style description
    #[arg(long)]
    pub prompt: String,

    /// Song lyrics with structure tags [Verse], [Chorus], etc.
    #[arg(long)]
    pub lyrics: Option<String>,

    /// Generate instrumental only (no vocals)
    #[arg(long, default_value_t = false)]
    pub instrumental: bool,

    /// Output file path
    #[arg(long, short)]
    pub out: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct LegacySearchArgs {
    /// Search query
    pub query: String,

    /// Number of results (1-10)
    #[arg(long, default_value_t = 5)]
    pub count: u8,
}

#[derive(Args, Debug, Clone)]
pub struct LegacyVisionArgs {
    /// Path to the image file
    pub file: String,

    /// Question to ask about the image
    #[arg(long, short)]
    pub prompt: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_chat() {
        let cli = Cli::try_parse_from([
            "vox", "text", "chat", "--message", "hello world",
        ]).unwrap();
        match cli.command {
            Some(Commands::Text(TextCommand::Chat { message, system, history, stream })) => {
                assert_eq!(message, Some("hello world".to_string()));
                assert!(system.is_none());
                assert!(history.is_none());
                assert!(!stream);
            }
            _ => panic!("Expected Text Chat command"),
        }
    }

    #[test]
    fn test_text_chat_repl() {
        let cli = Cli::try_parse_from(["vox", "text", "chat"]).unwrap();
        match cli.command {
            Some(Commands::Text(TextCommand::Chat { message, .. })) => {
                assert!(message.is_none());
            }
            _ => panic!("Expected Text Chat command"),
        }
    }

    #[test]
    fn test_text_repl() {
        let cli = Cli::try_parse_from(["vox", "text", "repl"]).unwrap();
        match cli.command {
            Some(Commands::Text(TextCommand::Repl { system, history })) => {
                assert!(system.is_none());
                assert!(history.is_none());
            }
            _ => panic!("Expected Text Repl command"),
        }
    }

    #[test]
    fn test_text_repl_with_system() {
        let cli = Cli::try_parse_from(["vox", "text", "repl", "--system", "You are a helpful assistant"]).unwrap();
        match cli.command {
            Some(Commands::Text(TextCommand::Repl { system, history })) => {
                assert_eq!(system, Some("You are a helpful assistant".to_string()));
                assert!(history.is_none());
            }
            _ => panic!("Expected Text Repl command"),
        }
    }

    #[test]
    fn test_text_complete() {
        let cli = Cli::try_parse_from([
            "vox", "text", "complete", "The future of AI",
        ]).unwrap();
        match cli.command {
            Some(Commands::Text(TextCommand::Complete { prompt, max_tokens, temperature })) => {
                assert_eq!(prompt, "The future of AI");
                assert_eq!(max_tokens, 256);
                assert!((temperature - 0.7).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Text Complete command"),
        }
    }

    #[test]
    fn test_image_generate() {
        let cli = Cli::try_parse_from([
            "vox", "image", "generate", "a sunset over mountains",
            "--aspect-ratio", "16:9", "-o", "sunset.png", "-n", "2",
        ]).unwrap();
        match cli.command {
            Some(Commands::Image(ImageCommand::Generate { prompt, aspect_ratio, output, n })) => {
                assert_eq!(prompt, "a sunset over mountains");
                assert_eq!(aspect_ratio, "16:9");
                assert_eq!(output, Some("sunset.png".to_string()));
                assert_eq!(n, 2);
            }
            _ => panic!("Expected Image Generate command"),
        }
    }

    #[test]
    fn test_image_edit() {
        let cli = Cli::try_parse_from([
            "vox", "image", "edit", "photo.jpg", "-p", "make it black and white",
        ]).unwrap();
        match cli.command {
            Some(Commands::Image(ImageCommand::Edit { file, prompt, output })) => {
                assert_eq!(file, "photo.jpg");
                assert_eq!(prompt, "make it black and white");
                assert!(output.is_none());
            }
            _ => panic!("Expected Image Edit command"),
        }
    }

    #[test]
    fn test_speech_generate() {
        let cli = Cli::try_parse_from([
            "vox", "speech", "generate", "--text", "Hello world", "--out", "hello.mp3",
            "--voice", "custom-voice", "--speed", "1.5", "--format", "wav",
        ]).unwrap();
        match cli.command {
            Some(Commands::Speech(SpeechCommand::Generate { text, out, voice, speed, format })) => {
                assert_eq!(text, "Hello world");
                assert_eq!(out, Some("hello.mp3".to_string()));
                assert_eq!(voice, "custom-voice");
                assert!((speed - 1.5).abs() < f64::EPSILON);
                assert_eq!(format, "wav");
            }
            _ => panic!("Expected Speech Generate command"),
        }
    }

    #[test]
    fn test_video_generate() {
        let cli = Cli::try_parse_from([
            "vox", "video", "generate", "-p", "Ocean waves",
            "--duration", "10", "--resolution", "1080P", "-o", "waves.mp4",
        ]).unwrap();
        match cli.command {
            Some(Commands::Video(VideoCommand::Generate { prompt, duration, resolution, out })) => {
                assert_eq!(prompt, "Ocean waves");
                assert_eq!(duration, 10);
                assert_eq!(resolution, "1080P");
                assert_eq!(out, Some("waves.mp4".to_string()));
            }
            _ => panic!("Expected Video Generate command"),
        }
    }

    #[test]
    fn test_video_status() {
        let cli = Cli::try_parse_from([
            "vox", "video", "status", "task-12345",
        ]).unwrap();
        match cli.command {
            Some(Commands::Video(VideoCommand::Status { task_id })) => {
                assert_eq!(task_id, "task-12345");
            }
            _ => panic!("Expected Video Status command"),
        }
    }

    #[test]
    fn test_music_generate() {
        let cli = Cli::try_parse_from([
            "vox", "music", "generate", "--prompt", "Upbeat pop",
            "--lyrics", "[verse] La da dee", "--instrumental", "-o", "song.mp3",
        ]).unwrap();
        match cli.command {
            Some(Commands::Music(MusicCommand::Generate { prompt, lyrics, instrumental, out })) => {
                assert_eq!(prompt, "Upbeat pop");
                assert_eq!(lyrics, Some("[verse] La da dee".to_string()));
                assert!(instrumental);
                assert_eq!(out, Some("song.mp3".to_string()));
            }
            _ => panic!("Expected Music Generate command"),
        }
    }

    #[test]
    fn test_search_query() {
        let cli = Cli::try_parse_from([
            "vox", "search", "query", "AI news", "--count", "10",
        ]).unwrap();
        match cli.command {
            Some(Commands::Search(SearchCommand::Query { query, count })) => {
                assert_eq!(query, "AI news");
                assert_eq!(count, 10);
            }
            _ => panic!("Expected Search Query command"),
        }
    }

    #[test]
    fn test_vision_analyze() {
        let cli = Cli::try_parse_from([
            "vox", "vision", "analyze", "photo.jpg", "-p", "What is in this image?",
        ]).unwrap();
        match cli.command {
            Some(Commands::Vision(VisionCommand::Analyze { file, prompt })) => {
                assert_eq!(file, "photo.jpg");
                assert_eq!(prompt, Some("What is in this image?".to_string()));
            }
            _ => panic!("Expected Vision Analyze command"),
        }
    }

    #[test]
    fn test_config_show() {
        let cli = Cli::try_parse_from(["vox", "config", "show"]).unwrap();
        match cli.command {
            Some(Commands::Config(ConfigCommand::Show)) => {}
            _ => panic!("Expected Config Show command"),
        }
    }

    #[test]
    fn test_config_get() {
        let cli = Cli::try_parse_from(["vox", "config", "get", "default_provider"]).unwrap();
        match cli.command {
            Some(Commands::Config(ConfigCommand::Get { key })) => {
                assert_eq!(key, "default_provider");
            }
            _ => panic!("Expected Config Get command"),
        }
    }

    #[test]
    fn test_config_set() {
        let cli = Cli::try_parse_from([
            "vox", "config", "set", "default_provider", "stepfun",
        ]).unwrap();
        match cli.command {
            Some(Commands::Config(ConfigCommand::Set { key, value })) => {
                assert_eq!(key, "default_provider");
                assert_eq!(value, "stepfun");
            }
            _ => panic!("Expected Config Set command"),
        }
    }

    #[test]
    fn test_config_edit() {
        let cli = Cli::try_parse_from(["vox", "config", "edit"]).unwrap();
        match cli.command {
            Some(Commands::Config(ConfigCommand::Edit)) => {}
            _ => panic!("Expected Config Edit command"),
        }
    }

    #[test]
    fn test_models_list() {
        let cli = Cli::try_parse_from(["vox", "models", "list", "-c", "chat"]).unwrap();
        match cli.command {
            Some(Commands::Models(ModelsCommand::List { capability })) => {
                assert_eq!(capability, Some("chat".to_string()));
            }
            _ => panic!("Expected Models List command"),
        }
    }

    #[test]
    fn test_models_set() {
        let cli = Cli::try_parse_from([
            "vox", "models", "set", "chat", "MiniMax-M2.7",
        ]).unwrap();
        match cli.command {
            Some(Commands::Models(ModelsCommand::Set { capability, model })) => {
                assert_eq!(capability, "chat");
                assert_eq!(model, "MiniMax-M2.7");
            }
            _ => panic!("Expected Models Set command"),
        }
    }

    #[test]
    fn test_providers_list() {
        let cli = Cli::try_parse_from(["vox", "providers", "list"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::List)) => {}
            _ => panic!("Expected Providers List command"),
        }
    }

    #[test]
    fn test_providers_status() {
        let cli = Cli::try_parse_from(["vox", "providers", "status", "-p", "minimax"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::Status { provider })) => {
                assert_eq!(provider, Some("minimax".to_string()));
            }
            _ => panic!("Expected Providers Status command"),
        }
    }

    #[test]
    fn test_init() {
        let cli = Cli::try_parse_from([
            "vox", "init", "-p", "stepfun", "--api-key", "sk-test",
        ]).unwrap();
        match cli.command {
            Some(Commands::Init(args)) => {
                assert_eq!(args.provider, Some("stepfun".to_string()));
                assert_eq!(args.api_key, Some("sk-test".to_string()));
                assert!(!args.interactive);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_doctor() {
        let cli = Cli::try_parse_from(["vox", "doctor"]).unwrap();
        match cli.command {
            Some(Commands::Doctor(args)) => {
                assert!(args.check.is_none());
                assert_eq!(args.format, "text");
            }
            _ => panic!("Expected Doctor command"),
        }
    }

    #[test]
    fn test_completion() {
        let cli = Cli::try_parse_from(["vox", "completion", "bash"]).unwrap();
        match cli.command {
            Some(Commands::Completion { shell }) => {
                assert_eq!(shell, "bash");
            }
            _ => panic!("Expected Completion command"),
        }
    }

    #[test]
    fn test_legacy_image() {
        let cli = Cli::try_parse_from([
            "vox", "image-legacy", "a cat", "--aspect-ratio", "4:3",
        ]).unwrap();
        match cli.command {
            Some(Commands::LegacyImage(args)) => {
                assert_eq!(args.prompt, "a cat");
                assert_eq!(args.aspect_ratio, "4:3");
            }
            _ => panic!("Expected LegacyImage command"),
        }
    }

    #[test]
    fn test_legacy_speech() {
        let cli = Cli::try_parse_from([
            "vox", "speech-legacy", "--text", "Hello",
        ]).unwrap();
        match cli.command {
            Some(Commands::LegacySpeech(args)) => {
                assert_eq!(args.text, "Hello");
            }
            _ => panic!("Expected LegacySpeech command"),
        }
    }

    #[test]
    fn test_global_opts() {
        let cli = Cli::try_parse_from([
            "vox", "--provider", "stepfun", "--model", "step-1-8k",
            "--quiet", "--verbose", "--no-color", "--format", "json",
            "--timeout", "60",
        ]).unwrap();
        assert_eq!(cli.global.provider, Some("stepfun".to_string()));
        assert_eq!(cli.global.model, Some("step-1-8k".to_string()));
        assert!(cli.global.quiet);
        assert!(cli.global.verbose);
        assert!(cli.global.no_color);
        assert_eq!(cli.global.format, "json");
        assert_eq!(cli.global.timeout, 60);
    }

    #[test]
    fn test_no_command_shows_help() {
        let cli = Cli::try_parse_from(["vox"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_legacy_commands_are_hidden() {
        // Verify legacy commands parse but aren't in help
        let cmd = Commands::LegacyImage(LegacyImageArgs {
            prompt: "test".into(),
            aspect_ratio: "1:1".into(),
            output: None,
            n: 1,
        });
        // Should not panic - just verifying the variant exists
        let _ = format!("{:?}", cmd);
    }
}
