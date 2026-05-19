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
    /// Run diagnostics and report issues
    Doctor(DoctorArgs),

    /// Manage provider configurations
    #[command(subcommand, name = "provider")]
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
    },
    /// Start an interactive chat session
    Repl {
        /// System prompt
        #[arg(long)]
        system: Option<String>,
    },
    /// Complete text from a prompt
    Complete {
        /// The prompt to complete
        prompt: String,
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
    /// Add or update a provider API key
    Add {
        /// Provider name (minimax, stepfun)
        provider: String,

        /// API key for the provider
        api_key: String,

        /// Group ID (required for MiniMax)
        #[arg(long)]
        group_id: Option<String>,
    },
    /// Remove a provider configuration
    Remove {
        /// Provider name (minimax, stepfun)
        provider: String,
    },
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
            Some(Commands::Text(TextCommand::Chat { message, system })) => {
                assert_eq!(message, Some("hello world".to_string()));
                assert!(system.is_none());
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
            Some(Commands::Text(TextCommand::Repl { system })) => {
                assert!(system.is_none());
            }
            _ => panic!("Expected Text Repl command"),
        }
    }

    #[test]
    fn test_text_repl_with_system() {
        let cli = Cli::try_parse_from(["vox", "text", "repl", "--system", "You are a helpful assistant"]).unwrap();
        match cli.command {
            Some(Commands::Text(TextCommand::Repl { system })) => {
                assert_eq!(system, Some("You are a helpful assistant".to_string()));
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
            Some(Commands::Text(TextCommand::Complete { prompt })) => {
                assert_eq!(prompt, "The future of AI");
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
        let cli = Cli::try_parse_from(["vox", "provider", "list"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::List)) => {}
            _ => panic!("Expected Providers List command"),
        }
    }

    #[test]
    fn test_providers_add() {
        let cli = Cli::try_parse_from(["vox", "provider", "add", "minimax", "sk-test123"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::Add { provider, api_key, group_id })) => {
                assert_eq!(provider, "minimax");
                assert_eq!(api_key, "sk-test123");
                assert!(group_id.is_none());
            }
            _ => panic!("Expected Providers Add command"),
        }
    }

    #[test]
    fn test_providers_add_with_group_id() {
        let cli = Cli::try_parse_from(["vox", "provider", "add", "minimax", "sk-test", "--group-id", "grp123"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::Add { provider, api_key, group_id })) => {
                assert_eq!(provider, "minimax");
                assert_eq!(api_key, "sk-test");
                assert_eq!(group_id, Some("grp123".to_string()));
            }
            _ => panic!("Expected Providers Add command with group_id"),
        }
    }

    #[test]
    fn test_providers_remove() {
        let cli = Cli::try_parse_from(["vox", "provider", "remove", "stepfun"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::Remove { provider })) => {
                assert_eq!(provider, "stepfun");
            }
            _ => panic!("Expected Providers Remove command"),
        }
    }

    #[test]
    fn test_providers_status() {
        let cli = Cli::try_parse_from(["vox", "provider", "status", "-p", "minimax"]).unwrap();
        match cli.command {
            Some(Commands::Providers(ProvidersCommand::Status { provider })) => {
                assert_eq!(provider, Some("minimax".to_string()));
            }
            _ => panic!("Expected Providers Status command"),
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
    fn test_global_opts() {
        let cli = Cli::try_parse_from([
            "vox", "--provider", "stepfun", "--model", "step-1-8k",
            "--quiet", "--verbose", "--no-color", "--format", "json",
        ]).unwrap();
        assert_eq!(cli.global.provider, Some("stepfun".to_string()));
        assert_eq!(cli.global.model, Some("step-1-8k".to_string()));
        assert!(cli.global.quiet);
        assert!(cli.global.verbose);
        assert!(cli.global.no_color);
        assert_eq!(cli.global.format, "json");
    }

    #[test]
    fn test_no_command_shows_help() {
        let cli = Cli::try_parse_from(["vox"]).unwrap();
        assert!(cli.command.is_none());
    }
}
