use clap::{Args, Parser, Subcommand};

/// Multi-provider AI multimedia CLI & TUI
#[derive(Parser, Debug)]
#[command(name = "vox", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalOpts {
    #[arg(long, env = "MMX_PROVIDER", default_value = "minimax")]
    pub provider: String,

    #[arg(long, env = "MMX_MODEL")]
    pub model: Option<String>,

    #[arg(long, env = "MMX_API_KEY")]
    pub api_key: Option<String>,

    #[arg(long, env = "MMX_GROUP_ID")]
    pub group_id: Option<String>,

    #[arg(long, env = "MMX_BASE_URL")]
    pub base_url: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Generate an image from a text prompt
    Image {
        /// The prompt describing the desired image
        prompt: String,

        /// Aspect ratio (1:1, 16:9, 4:3, 3:2, 2:3, 3:4, 9:16, 21:9)
        #[arg(long, default_value = "1:1")]
        aspect_ratio: String,

        /// Output file path
        #[arg(long, short)]
        output: Option<String>,

        /// Number of images to generate (1-9)
        #[arg(long, default_value_t = 1)]
        n: u8,
    },

    /// Synthesize speech from text
    Speech {
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

    /// Generate video from a text prompt
    Video {
        /// The prompt describing the desired video
        #[arg(long, short)]
        prompt: String,

        /// Duration in seconds (6 or 10)
        #[arg(long, default_value_t = 6)]
        duration: u8,

        /// Resolution (720P, 768P, 1080P)
        #[arg(long, default_value = "720P")]
        resolution: String,
    },

    /// Generate music from a prompt and optional lyrics
    Music {
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

    /// Search the web
    Search {
        /// Search query
        query: String,

        /// Number of results (1-10)
        #[arg(long, default_value_t = 5)]
        count: u8,
    },

    /// Analyze an image file
    Vision {
        /// Path to the image file
        file: String,

        /// Question to ask about the image
        #[arg(long, short)]
        prompt: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_no_args() {
        let cli = Cli::try_parse_from(["vox"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.global.provider, "minimax");
    }

    #[test]
    fn test_cli_image_command() {
        let cli = Cli::try_parse_from(["vox", "image", "a cat"]).unwrap();
        match cli.command {
            Some(Commands::Image {
                prompt,
                aspect_ratio,
                output,
                n,
            }) => {
                assert_eq!(prompt, "a cat");
                assert_eq!(aspect_ratio, "1:1");
                assert!(output.is_none());
                assert_eq!(n, 1);
            }
            _ => panic!("Expected Image command"),
        }
    }

    #[test]
    fn test_cli_speech_command() {
        let cli =
            Cli::try_parse_from(["vox", "speech", "--text", "Hello", "--out", "hello.mp3"])
                .unwrap();
        match cli.command {
            Some(Commands::Speech {
                text,
                out,
                voice,
                speed,
                format,
            }) => {
                assert_eq!(text, "Hello");
                assert_eq!(out, Some("hello.mp3".to_string()));
                assert_eq!(voice, "English_expressive_narrator");
                assert_eq!(speed, 1.0);
                assert_eq!(format, "mp3");
            }
            _ => panic!("Expected Speech command"),
        }
    }

    #[test]
    fn test_cli_video_command() {
        let cli = Cli::try_parse_from(["vox", "video", "--prompt", "Ocean waves"]).unwrap();
        match cli.command {
            Some(Commands::Video {
                prompt,
                duration,
                resolution,
            }) => {
                assert_eq!(prompt, "Ocean waves");
                assert_eq!(duration, 6);
                assert_eq!(resolution, "720P");
            }
            _ => panic!("Expected Video command"),
        }
    }

    #[test]
    fn test_cli_music_command() {
        let cli = Cli::try_parse_from([
            "vox",
            "music",
            "--prompt",
            "Upbeat pop",
            "--lyrics",
            "[verse] La da dee",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Music {
                prompt,
                lyrics,
                instrumental,
                out,
            }) => {
                assert_eq!(prompt, "Upbeat pop");
                assert_eq!(lyrics, Some("[verse] La da dee".to_string()));
                assert!(!instrumental);
                assert!(out.is_none());
            }
            _ => panic!("Expected Music command"),
        }
    }

    #[test]
    fn test_cli_search_command() {
        let cli = Cli::try_parse_from(["vox", "search", "MiniMax AI news"]).unwrap();
        match cli.command {
            Some(Commands::Search { query, count }) => {
                assert_eq!(query, "MiniMax AI news");
                assert_eq!(count, 5);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_cli_vision_command() {
        let cli = Cli::try_parse_from(["vox", "vision", "photo.jpg"]).unwrap();
        match cli.command {
            Some(Commands::Vision { file, prompt }) => {
                assert_eq!(file, "photo.jpg");
                assert!(prompt.is_none());
            }
            _ => panic!("Expected Vision command"),
        }
    }

    #[test]
    fn test_cli_global_opts_env() {
        let cli = Cli::try_parse_from(["vox"]).unwrap();
        assert_eq!(cli.global.provider, "minimax");
        assert!(cli.global.model.is_none());
        assert!(cli.global.api_key.is_none());
        assert!(cli.global.group_id.is_none());
        assert!(cli.global.base_url.is_none());
    }
}
