mod capabilities;
mod config;
pub mod minimax;
mod models;
mod output;
pub mod provider;
pub mod cli;
pub mod command;
mod stepfun;

#[cfg(feature = "tui")]
mod app;
#[cfg(feature = "tui")]
mod input;
#[cfg(feature = "tui")]
pub mod ui;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::{Cli, Commands, GlobalOpts, TextCommand, ImageCommand, SpeechCommand, VideoCommand, MusicCommand, SearchCommand, VisionCommand, ProvidersCommand, ModelsCommand, ConfigCommand};
use crate::config::{Config, Provider as ConfigProvider};
use crate::output::{Output, OutputFormat};
use crate::provider::create_provider;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    if cli.command.is_some() {
        run_cli(cli).await
    } else {
        #[cfg(feature = "tui")]
        {
            run_tui().await
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("vox: no subcommand given. TUI is not enabled in this build.");
            eprintln!("Run `vox --help` for available commands.");
            std::process::exit(1);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Provider resolution
// ═══════════════════════════════════════════════════════════════════

fn resolve_provider(global: &GlobalOpts, config: &Config) -> Result<ConfigProvider, String> {
    // 1. CLI flag takes highest priority
    if let Some(name) = &global.provider {
        return match name.to_lowercase().as_str() {
            "minimax" => Ok(ConfigProvider::MiniMax),
            "stepfun" => Ok(ConfigProvider::StepFun),
            other => Err(format!("Unknown provider: {other}")),
        };
    }

    // 2. config.default_provider
    let default = &config.default_provider;

    // 3. Auto-detect: only 1 provider has API key
    let stepfun_has_key = config.stepfun.as_ref().map_or(false, |s| !s.api_key.is_empty());
    let minimax_has_key = config.minimax.as_ref().map_or(false, |m| !m.api_key.is_empty());

    match (stepfun_has_key, minimax_has_key) {
        (true, false) => return Ok(ConfigProvider::StepFun),
        (false, true) => return Ok(ConfigProvider::MiniMax),
        (true, true) => {} // fall through to default_provider
        (false, false) => return Err("No providers configured. Run: vox init".into()),
    }

    // Use default_provider from config
    Ok(default.clone())
}

// ═══════════════════════════════════════════════════════════════════
// Main CLI dispatcher
// ═══════════════════════════════════════════════════════════════════

async fn run_cli(cli: Cli) -> std::io::Result<()> {
    // Load config (respecting --config override via VOX_CONFIG env)
    let mut config = if let Some(ref config_path) = cli.global.config {
        Config::load_from(std::path::Path::new(config_path)).unwrap_or_default()
    } else {
        Config::load().unwrap_or_default()
    };

    // Create Output formatter
    let output = Output::new(
        OutputFormat::from_str(&cli.global.format),
        cli.global.quiet,
        cli.global.verbose,
        cli.global.no_color,
    );

    // Handle commands that don't need a provider first
    match cli.command {
        Some(Commands::Init(args)) => {
            handle_init(args, &mut config, &output);
            return Ok(());
        }
        Some(Commands::Doctor(args)) => {
            handle_doctor(args, &config, &output);
            return Ok(());
        }
        Some(Commands::Config(cmd)) => {
            handle_config(cmd, &mut config, &output);
            return Ok(());
        }
        Some(Commands::Completion { shell }) => {
            handle_completion(&shell);
            return Ok(());
        }
        _ => {}
    }

    // Resolve provider using 4-level chain
    let provider_name = match resolve_provider(&cli.global, &config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Error: {e}"), 1);
            std::process::exit(1);
        }
    };

    // Override config provider based on resolution
    config.default_provider = provider_name.clone();

    // Apply global flag overrides to config
    apply_global_overrides(&cli.global, &mut config);

    // Dispatch to command handler
    match cli.command {
        // ── AI capability resources ──────────────────────────────────
        Some(Commands::Text(cmd)) => handle_text(cmd, &config, &output).await,
        Some(Commands::Image(cmd)) => handle_image(cmd, &config, &output).await,
        Some(Commands::Speech(cmd)) => handle_speech(cmd, &config, &output).await,
        Some(Commands::Video(cmd)) => handle_video(cmd, &config, &output).await,
        Some(Commands::Music(cmd)) => handle_music(cmd, &config, &output).await,
        Some(Commands::Search(cmd)) => handle_search(cmd, &config, &output).await,
        Some(Commands::Vision(cmd)) => handle_vision(cmd, &config, &output).await,

        // ── Setup & configuration ────────────────────────────────────
        Some(Commands::Init(_)) => unreachable!("handled above"),
        Some(Commands::Doctor(_)) => unreachable!("handled above"),
        Some(Commands::Providers(cmd)) => handle_providers(cmd, &config, &output),
        Some(Commands::Models(cmd)) => handle_models(cmd, &mut config, &output),
        Some(Commands::Config(_)) => unreachable!("handled above"),
        Some(Commands::Completion { .. }) => unreachable!("handled above"),

        // ── Legacy backward-compat ───────────────────────────────────
        Some(Commands::LegacyImage(args)) => {
            output.deprecation("The 'image' command is deprecated. Use 'vox image generate <prompt>' instead.");
            handle_image(ImageCommand::Generate {
                prompt: args.prompt,
                aspect_ratio: args.aspect_ratio,
                output: args.output,
                n: args.n,
            }, &config, &output).await;
        }
        Some(Commands::LegacySpeech(args)) => {
            output.deprecation("The 'speech' command is deprecated. Use 'vox speech generate --text <text>' instead.");
            handle_speech(SpeechCommand::Generate {
                text: args.text,
                out: args.out,
                voice: args.voice,
                speed: args.speed,
                format: args.format,
            }, &config, &output).await;
        }
        Some(Commands::LegacyVideo(args)) => {
            output.deprecation("The 'video' command is deprecated. Use 'vox video generate --prompt <prompt>' instead.");
            handle_video(VideoCommand::Generate {
                prompt: args.prompt,
                duration: args.duration,
                resolution: args.resolution,
                out: None,
            }, &config, &output).await;
        }
        Some(Commands::LegacyMusic(args)) => {
            output.deprecation("The 'music' command is deprecated. Use 'vox music generate --prompt <prompt>' instead.");
            handle_music(MusicCommand::Generate {
                prompt: args.prompt,
                lyrics: args.lyrics,
                instrumental: args.instrumental,
                out: args.out,
            }, &config, &output).await;
        }
        Some(Commands::LegacySearch(args)) => {
            output.deprecation("The 'search' command is deprecated. Use 'vox search query <query>' instead.");
            handle_search(SearchCommand::Query {
                query: args.query,
                count: args.count,
            }, &config, &output).await;
        }
        Some(Commands::LegacyVision(args)) => {
            output.deprecation("The 'vision' command is deprecated. Use 'vox vision analyze <file>' instead.");
            handle_vision(VisionCommand::Analyze {
                file: args.file,
                prompt: args.prompt,
            }, &config, &output).await;
        }

        None => {}
    }

    Ok(())
}

fn apply_global_overrides(global: &GlobalOpts, config: &mut Config) {
    if let Some(model) = &global.model {
        match config.default_provider {
            ConfigProvider::StepFun => {
                if let Some(ref mut stepfun) = config.stepfun {
                    stepfun.model = Some(model.clone());
                }
            }
            ConfigProvider::MiniMax => {
                if let Some(ref mut minimax) = config.minimax {
                    minimax.model = Some(model.clone());
                }
            }
        }
    }

    if let Some(api_key) = &global.api_key {
        match config.default_provider {
            ConfigProvider::StepFun => {
                if let Some(ref mut stepfun) = config.stepfun {
                    stepfun.api_key.clone_from(api_key);
                }
            }
            ConfigProvider::MiniMax => {
                if let Some(ref mut minimax) = config.minimax {
                    minimax.api_key.clone_from(api_key);
                }
            }
        }
    }

    if let Some(base_url) = &global.base_url {
        match config.default_provider {
            ConfigProvider::StepFun => {
                if let Some(ref mut stepfun) = config.stepfun {
                    stepfun.base_url = Some(base_url.clone());
                }
            }
            ConfigProvider::MiniMax => {
                if let Some(ref mut minimax) = config.minimax {
                    minimax.base_url = Some(base_url.clone());
                }
            }
        }
    }

    if let Some(ref group_id) = global.api_key {
        // group_id was removed from GlobalOpts; this is a no-op for backward compat
        let _ = group_id;
    }

    if let Some(ref output_dir) = global.output_dir {
        config.output_dir = Some(output_dir.clone());
    }
}

// ═══════════════════════════════════════════════════════════════════
// Command handlers
// ═══════════════════════════════════════════════════════════════════

async fn handle_text(cmd: TextCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        TextCommand::Chat { message, system, history: _, stream } => {
            // Check capability
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("chat") {
                output.error(&e, 1);
                return;
            }

            let messages = if let Some(sys) = system {
                vec![
                    provider::Message::system(sys),
                    provider::Message::user(message),
                ]
            } else {
                vec![provider::Message::user(message)]
            };

            if stream {
                output.status("Streaming response...");
                // Streaming not fully implemented in providers yet
                match provider.chat(&messages).await {
                    Ok(resp) => output.result(&resp.content),
                    Err(e) => output.error(&format!("{e}"), 1),
                }
            } else {
                match provider.chat(&messages).await {
                    Ok(resp) => output.result(&resp.content),
                    Err(e) => output.error(&format!("{e}"), 1),
                }
            }
        }
        TextCommand::Complete { prompt, max_tokens: _, temperature: _ } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("chat") {
                output.error(&e, 1);
                return;
            }

            let messages = vec![provider::Message::user(prompt)];
            match provider.chat(&messages).await {
                Ok(resp) => output.result(&resp.content),
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

async fn handle_image(cmd: ImageCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        ImageCommand::Generate { prompt, aspect_ratio, output: out_path, n } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("image_generate") {
                output.error(&e, 1);
                return;
            }

            match provider.image_generate(&prompt, n, &aspect_ratio).await {
                Ok(resp) => {
                    if let Some(path) = out_path {
                        for (i, url) in resp.urls.iter().enumerate() {
                            let file_path = if resp.urls.len() == 1 {
                                path.clone()
                            } else {
                                let p = std::path::Path::new(&path);
                                let parent = p.parent();
                                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
                                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("png");
                                let filename = format!("{stem}_{i}.{ext}");
                                match parent {
                                    Some(dir) if !dir.as_os_str().is_empty() => {
                                        dir.join(filename).to_string_lossy().into_owned()
                                    }
                                    _ => filename,
                                }
                            };
                            if let Err(e) = download_file(url, &file_path) {
                                output.error(&format!("Failed to download {url}: {e}"), 1);
                            } else {
                                output.status(&format!("Saved to {file_path}"));
                            }
                        }
                    } else {
                        for url in resp.urls {
                            output.result(&url);
                        }
                    }
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
        ImageCommand::Edit { file, prompt, output: out_path } => {
            // Image editing not yet supported by providers
            output.error("Image editing is not yet implemented", 1);
            let _ = (file, prompt, out_path);
        }
    }
}

async fn handle_speech(cmd: SpeechCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        SpeechCommand::Generate { text, out, voice, speed, format } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("speech_synthesize") {
                output.error(&e, 1);
                return;
            }

            let output_path = out.unwrap_or_else(|| "output.mp3".to_string());
            match provider.speech_synthesize(&text, &voice, speed, &format).await {
                Ok(resp) => {
                    if let Err(e) = std::fs::write(&output_path, &resp.audio_data) {
                        output.error(&format!("Failed to write audio file: {e}"), 1);
                    } else {
                        output.status(&format!("Saved to {output_path}"));
                    }
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

async fn handle_video(cmd: VideoCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        VideoCommand::Generate { prompt, duration, resolution, out: _ } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("video_generate") {
                output.error(&e, 1);
                return;
            }

            match provider.video_generate(&prompt, duration, &resolution).await {
                Ok(resp) => {
                    output.result(&format!("Task ID: {}", resp.task_id));
                    output.result(&format!("Status: {}", resp.status));
                    if let Some(url) = resp.video_url {
                        output.result(&format!("Video URL: {url}"));
                    }
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
        VideoCommand::Status { task_id } => {
            // Video status polling not yet implemented in providers
            output.result(&format!("Checking status of task: {task_id}"));
            output.result("Video status polling is not yet implemented");
        }
    }
}

async fn handle_music(cmd: MusicCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        MusicCommand::Generate { prompt, lyrics, instrumental, out } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("music_generate") {
                output.error(&e, 1);
                return;
            }

            let output_path = out.unwrap_or_else(|| "output.mp3".to_string());
            match provider.music_generate(&prompt, lyrics.as_deref(), instrumental).await {
                Ok(resp) => {
                    if let Err(e) = std::fs::write(&output_path, &resp.audio_data) {
                        output.error(&format!("Failed to write audio file: {e}"), 1);
                    } else {
                        output.status(&format!("Saved to {output_path}"));
                    }
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

async fn handle_search(cmd: SearchCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        SearchCommand::Query { query, count } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("search") {
                output.error(&e, 1);
                return;
            }

            match provider.search(&query, count).await {
                Ok(resp) => {
                    for result in resp.results {
                        output.result(&format!("{}\n  {}\n  {}\n", result.title, result.url, result.snippet));
                    }
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

async fn handle_vision(cmd: VisionCommand, config: &Config, output: &Output) {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return;
        }
    };

    match cmd {
        VisionCommand::Analyze { file, prompt } => {
            if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider).require("vision") {
                output.error(&e, 1);
                return;
            }

            match provider.vision(&file, prompt.as_deref()).await {
                Ok(resp) => {
                    output.result(&resp.description);
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Setup & configuration handlers
// ═══════════════════════════════════════════════════════════════════

fn handle_init(args: cli::InitArgs, config: &mut Config, output: &Output) {
    if args.interactive {
        println!("Interactive setup not yet implemented. Please run: vox init -p <provider> --api-key <key>");
        return;
    }

    let provider = match args.provider.as_deref() {
        Some("minimax") => ConfigProvider::MiniMax,
        Some("stepfun") => ConfigProvider::StepFun,
        Some(other) => {
            output.error(&format!("Unknown provider: {other}"), 1);
            return;
        }
        None => {
            output.error("Provider is required. Use -p minimax or -p stepfun", 1);
            return;
        }
    };

    let api_key = match args.api_key {
        Some(key) => key,
        None => {
            output.error("API key is required. Use --api-key <key>", 1);
            return;
        }
    };

    config.default_provider = provider.clone();

    match provider {
        ConfigProvider::StepFun => {
            config.stepfun = Some(config::StepFunConfig {
                api_key,
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
        }
        ConfigProvider::MiniMax => {
            config.minimax = Some(config::MiniMaxConfig {
                api_key,
                group_id: None,
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
        }
    }

    if let Err(e) = config.save() {
        output.error(&format!("Failed to save config: {e}"), 1);
    } else {
        output.status("Configuration saved successfully.");
    }
}

fn handle_doctor(args: cli::DoctorArgs, config: &Config, output: &Output) {
    output.result("vox doctor — diagnostics");
    output.result("========================");

    // Check config exists
    match Config::config_path() {
        Some(path) => output.result(&format!("Config path: {}", path.display())),
        None => output.result("Config path: (none)"),
    }

    // Check providers
    let providers = config.configured_providers();
    if providers.is_empty() {
        output.result("Providers: (none configured)");
    } else {
        for p in &providers {
            let name = match p {
                ConfigProvider::StepFun => "StepFun",
                ConfigProvider::MiniMax => "MiniMax",
            };
            let has_key = match p {
                ConfigProvider::StepFun => config.stepfun.as_ref().map_or(false, |s| !s.api_key.is_empty()),
                ConfigProvider::MiniMax => config.minimax.as_ref().map_or(false, |m| !m.api_key.is_empty()),
            };
            output.result(&format!("  {name}: API key {}", if has_key { "configured" } else { "missing" }));
        }
    }

    // Check default provider
    output.result(&format!("Default provider: {}", config.default_provider));

    // API connectivity check (only if specific check not requested)
    if args.check.is_none() {
        output.result("");
        output.result("Checking API connectivity...");

        match create_provider(config) {
            Ok(provider) => {
                output.result(&format!("Provider {} is reachable", provider.name()));
            }
            Err(e) => {
                output.error(&format!("Provider connectivity check failed: {e}"), 0);
            }
        }
    }

    let _ = args;
}

fn handle_providers(cmd: ProvidersCommand, config: &Config, output: &Output) {
    match cmd {
        ProvidersCommand::List => {
            output.result("Configured providers:");
            let providers = config.configured_providers();
            if providers.is_empty() {
                output.result("  (none)");
            } else {
                for p in &providers {
                    let (name, api_key_masked) = match p {
                        ConfigProvider::StepFun => {
                            let masked = config.stepfun.as_ref()
                                .map(|s| {
                                    if s.api_key.is_empty() { "(not set)".to_string() }
                                    else { format!("{}***", &s.api_key[..4.min(s.api_key.len())]) }
                                })
                                .unwrap_or_else(|| "(not configured)".to_string());
                            ("StepFun", masked)
                        }
                        ConfigProvider::MiniMax => {
                            let masked = config.minimax.as_ref()
                                .map(|m| {
                                    if m.api_key.is_empty() { "(not set)".to_string() }
                                    else { format!("{}***", &m.api_key[..4.min(m.api_key.len())]) }
                                })
                                .unwrap_or_else(|| "(not configured)".to_string());
                            ("MiniMax", masked)
                        }
                    };
                    let is_default = if p == &config.default_provider { " (default)" } else { "" };
                    output.result(&format!("  {name}: {api_key_masked}{is_default}"));
                }
            }
        }
        ProvidersCommand::Status { provider } => {
            let target = match provider {
                Some(name) => match name.to_lowercase().as_str() {
                    "minimax" => ConfigProvider::MiniMax,
                    "stepfun" => ConfigProvider::StepFun,
                    other => {
                        output.error(&format!("Unknown provider: {other}"), 1);
                        return;
                    }
                },
                None => config.default_provider.clone(),
            };

            output.result(&format!("Testing connectivity to {}...", target));

            // Make a lightweight test call
            let test_config = Config {
                default_provider: target.clone(),
                stepfun: config.stepfun.clone(),
                minimax: config.minimax.clone(),
                theme: None,
                output_dir: None,
            };

            match create_provider(&test_config) {
                Ok(_) => output.result(&format!("{} is reachable", target)),
                Err(e) => output.error(&format!("{} connectivity failed: {e}", target), 1),
            }
        }
    }
}

fn handle_models(cmd: ModelsCommand, config: &mut Config, output: &Output) {
    match cmd {
        ModelsCommand::List { capability } => {
            let capabilities = match capability.as_deref() {
                Some(c) => vec![c.to_string()],
                None => vec!["chat".into(), "image".into(), "speech".into(), "video".into(), "music".into(), "vision".into(), "search".into()],
            };

            for cap in &capabilities {
                let models = models::get_available_models(&config.default_provider, cap);
                if models.is_empty() {
                    output.result(&format!("{cap}: (no known models)"));
                } else {
                    output.result(&format!("{cap}:"));
                    for m in &models {
                        let is_selected = config.get_model_for(cap).map_or(false, |cm| cm == *m);
                        let marker = if is_selected { " (current)" } else { "" };
                        output.result(&format!("  {m}{marker}"));
                    }
                }
            }
        }
        ModelsCommand::Set { capability, model } => {
            // Validate the model is known
            let known = models::get_available_models(&config.default_provider, &capability);
            if !known.is_empty() && !known.contains(&model) {
                output.error(&format!("Unknown model '{model}' for capability '{capability}'. Known models: {}", known.join(", ")), 1);
                return;
            }

            // Set in config
            match config.default_provider {
                ConfigProvider::StepFun => {
                    let sf = config.stepfun.get_or_insert_with(|| config::StepFunConfig {
                        api_key: String::new(),
                        base_url: None,
                        model: None,
                        models: config::ProviderModels::default(),
                    });
                    sf.models.set(&capability, model.clone());
                }
                ConfigProvider::MiniMax => {
                    let mm = config.minimax.get_or_insert_with(|| config::MiniMaxConfig {
                        api_key: String::new(),
                        group_id: None,
                        base_url: None,
                        model: None,
                        models: config::ProviderModels::default(),
                    });
                    mm.models.set(&capability, model.clone());
                }
            }

            if let Err(e) = config.save() {
                output.error(&format!("Failed to save config: {e}"), 1);
            } else {
                output.status(&format!("Set {capability} model to {model}"));
            }
        }
    }
}

fn handle_config(cmd: ConfigCommand, config: &mut Config, output: &Output) {
    match cmd {
        ConfigCommand::Show => {
            // Print config with masked API keys
            output.result("Current configuration:");
            output.result(&format!("  default_provider: {}", config.default_provider));

            if let Some(ref sf) = config.stepfun {
                let masked = if sf.api_key.is_empty() {
                    "(not set)".to_string()
                } else {
                    format!("{}***", &sf.api_key[..4.min(sf.api_key.len())])
                };
                output.result("  stepfun:");
                output.result(&format!("    api_key: {masked}"));
                if let Some(ref url) = sf.base_url {
                    output.result(&format!("    base_url: {url}"));
                }
            }

            if let Some(ref mm) = config.minimax {
                let masked = if mm.api_key.is_empty() {
                    "(not set)".to_string()
                } else {
                    format!("{}***", &mm.api_key[..4.min(mm.api_key.len())])
                };
                output.result("  minimax:");
                output.result(&format!("    api_key: {masked}"));
                if let Some(ref gid) = mm.group_id {
                    output.result(&format!("    group_id: {gid}"));
                }
                if let Some(ref url) = mm.base_url {
                    output.result(&format!("    base_url: {url}"));
                }
            }

            if let Some(ref dir) = config.output_dir {
                output.result(&format!("  output_dir: {dir}"));
            }
        }
        ConfigCommand::Get { key } => {
            let value = parse_config_key(config, &key);
            match value {
                Some(v) => output.result(&v),
                None => output.error(&format!("Unknown config key: {key}"), 1),
            }
        }
        ConfigCommand::Set { key, value } => {
            if let Err(e) = set_config_key(config, &key, &value) {
                output.error(&e, 1);
            } else {
                output.status(&format!("Set {key} = {value}"));
            }
        }
        ConfigCommand::Edit => {
            let config_path = Config::config_path();
            match config_path {
                Some(path) => {
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
                        if cfg!(windows) { "notepad" } else { "vi" }.to_string()
                    });
                    output.status(&format!("Opening {} in {}", path.display(), editor));
                    if let Err(e) = std::process::Command::new(&editor).arg(&path).status() {
                        output.error(&format!("Failed to open editor: {e}"), 1);
                    }
                }
                None => {
                    output.error("No config path available", 1);
                }
            }
        }
    }
}

fn handle_completion(shell: &str) {
    let shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        other => {
            eprintln!("Unsupported shell: {other}. Supported: bash, zsh, fish, powershell, elvish");
            std::process::exit(1);
        }
    };

    let mut app = Cli::command();
    generate(shell, &mut app, "vox", &mut io::stdout());
}

// ═══════════════════════════════════════════════════════════════════
// Config key helpers
// ═══════════════════════════════════════════════════════════════════

fn parse_config_key(config: &Config, key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["default_provider"] => Some(config.default_provider.to_string()),
        ["stepfun", "api_key"] => config.stepfun.as_ref().map(|s| s.api_key.clone()),
        ["stepfun", "base_url"] => config.stepfun.as_ref().and_then(|s| s.base_url.clone()),
        ["stepfun", "model"] => config.stepfun.as_ref().and_then(|s| s.model.clone()),
        ["minimax", "api_key"] => config.minimax.as_ref().map(|m| m.api_key.clone()),
        ["minimax", "group_id"] => config.minimax.as_ref().and_then(|m| m.group_id.clone()),
        ["minimax", "base_url"] => config.minimax.as_ref().and_then(|m| m.base_url.clone()),
        ["minimax", "model"] => config.minimax.as_ref().and_then(|m| m.model.clone()),
        ["output_dir"] => config.output_dir.clone(),
        _ => None,
    }
}

fn set_config_key(config: &mut Config, key: &str, value: &str) -> Result<(), String> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["default_provider"] => {
            config.default_provider = match value.to_lowercase().as_str() {
                "minimax" => ConfigProvider::MiniMax,
                "stepfun" => ConfigProvider::StepFun,
                other => return Err(format!("Unknown provider: {other}")),
            };
        }
        ["stepfun", "api_key"] => {
            let sf = config.stepfun.get_or_insert_with(|| config::StepFunConfig {
                api_key: String::new(),
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
            sf.api_key = value.to_string();
        }
        ["stepfun", "base_url"] => {
            let sf = config.stepfun.get_or_insert_with(|| config::StepFunConfig {
                api_key: String::new(),
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
            sf.base_url = Some(value.to_string());
        }
        ["minimax", "api_key"] => {
            let mm = config.minimax.get_or_insert_with(|| config::MiniMaxConfig {
                api_key: String::new(),
                group_id: None,
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
            mm.api_key = value.to_string();
        }
        ["minimax", "group_id"] => {
            let mm = config.minimax.get_or_insert_with(|| config::MiniMaxConfig {
                api_key: String::new(),
                group_id: None,
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
            mm.group_id = Some(value.to_string());
        }
        ["minimax", "base_url"] => {
            let mm = config.minimax.get_or_insert_with(|| config::MiniMaxConfig {
                api_key: String::new(),
                group_id: None,
                base_url: None,
                model: None,
                models: config::ProviderModels::default(),
            });
            mm.base_url = Some(value.to_string());
        }
        ["output_dir"] => {
            config.output_dir = Some(value.to_string());
        }
        _ => return Err(format!("Unknown config key: {key}")),
    }
    config.save().map_err(|e| format!("Failed to save config: {e}"))
}

// ═══════════════════════════════════════════════════════════════════
// Utility functions
// ═══════════════════════════════════════════════════════════════════

fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;
    std::fs::write(path, bytes)?;
    Ok(())
}

// ── TUI mode (only compiled with `--features tui`) ──────────────────────

#[cfg(feature = "tui")]
use std::{
    io,
    time::{Duration, Instant},
};

#[cfg(feature = "tui")]
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[cfg(feature = "tui")]
use ratatui::backend::CrosstermBackend;

#[cfg(feature = "tui")]
use ratatui::Terminal;

#[cfg(feature = "tui")]
use crate::app::AppState;

#[cfg(feature = "tui")]
use crate::input::InputMode;

#[cfg(feature = "tui")]
use crate::ui::{AudioView, ChatView, ConfigView, ImageView, Layout, View, AppTheme, AppLayout, compute_layout};

#[cfg(feature = "tui")]
async fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new_for_tui();
    app.init_picker();
    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(feature = "tui")]
async fn run_app(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut AppState,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);
    let mut theme = AppTheme::from_config(app.config.theme.as_ref());

    while app.running {
        terminal.draw(|f| {
            let area = f.area();
            // Recreate theme only if config changed (simple check: compare accent)
            let new_theme = AppTheme::from_config(app.config.theme.as_ref());
            if new_theme.accent != theme.accent || new_theme.is_dark != theme.is_dark {
                theme = new_theme;
            }

            // Global background fill
            for y in area.y..(area.y + area.height) {
                for x in area.x..(area.x + area.width) {
                    if let Some(cell) = f.buffer_mut().cell_mut((x, y)) {
                        cell.set_bg(theme.background);
                    }
                }
            }

            let AppLayout { sidebar, main, status } = compute_layout(area);

            Layout::render_sidebar(f, sidebar, app.current_view, &theme);

            match app.current_view {
                View::Chat => {
                    let chat_view = ChatView::new(&app.messages, &app.input.content, &theme)
                        .streaming(app.input_mode == InputMode::Streaming)
                        .scroll_offset(app.scroll_offset);
                    chat_view.render(f, main);
                }
                View::Image => {
                    let mut image_view = ImageView::new(&app.input.content, &theme)
                        .generating(app.input_mode == InputMode::Streaming);
                    if let Some(ref result) = app.image_result {
                        image_view = image_view.preview(result);
                    }
                    let proto = app.image_protocol.as_ref();
                    image_view.render_with_image(f, main, proto);
                }
                View::Audio => {
                    let display_text = app.audio_result.as_deref().unwrap_or(&app.input.content);
                    let audio_view = AudioView::new(display_text, &app.status, &theme)
                        .generating(app.input_mode == InputMode::Streaming);
                    audio_view.render(f, main);
                }
                View::Config => {
                    let config_view = ConfigView::new(&app.config, &theme)
                        .with_selected(app.config_editor.selected)
                        .with_editing(app.config_editor.editing)
                        .with_edit_buffer(&app.config_editor.edit_buffer);
                    config_view.render(f, main);
                }
            }

            let mode_label = match app.input_mode {
                InputMode::Normal => "NORM",
                InputMode::Typing => "INS",
                InputMode::Streaming => "STRM",
                InputMode::ConfigNavigating => "CFG",
                InputMode::ConfigEditing => "EDT",
            };
            let provider_name = match app.config.default_provider {
                ConfigProvider::StepFun => "StepFun",
                ConfigProvider::MiniMax => "MiniMax",
            };
            let model_name = match app.config.default_provider {
                ConfigProvider::StepFun => app.config.stepfun.as_ref().and_then(|s| s.model.as_deref()).unwrap_or("default"),
                ConfigProvider::MiniMax => app.config.minimax.as_ref().and_then(|m| m.model.as_deref()).unwrap_or("default"),
            };
            let position_label = format!("{provider_name} • {model_name}");
            let help_label = match app.current_view {
                View::Chat => "Tab: switch  Enter: chat  q: quit",
                View::Image => "Tab: switch  Enter: generate  q: quit",
                View::Audio => "Tab: switch  Enter: speak  q: quit",
                View::Config => "Tab: switch  Enter: edit  q: quit",
            };

            Layout::render_status_bar(f, status, mode_label, &position_label, help_label, &theme);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    let action = input::handle_key_event(key);
                    app.handle_input(action);
                }
                Event::Paste(text) => {
                    if app.input_mode == InputMode::Normal && app.current_view != View::Config {
                        app.input_mode = InputMode::Typing;
                    }
                    if app.input_mode == InputMode::Typing {
                        for c in text.chars() {
                            app.input.insert_char(c);
                        }
                    }
                }
                _other => {}
            }
        }

        app.tick();

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}
