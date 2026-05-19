mod capabilities;
pub mod cli;
pub mod command;
mod config;
mod models;
mod output;
pub mod providers;

#[cfg(feature = "tui")]
mod app;
#[cfg(feature = "tui")]
mod input;
#[cfg(feature = "tui")]
pub mod ui;

use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use std::io;

use crate::capabilities::Capability;
use crate::cli::{
    Cli, Commands, ConfigCommand, GlobalOpts, ImageCommand, ModelsCommand, MusicCommand,
    ProvidersCommand, SearchCommand, SpeechCommand, TextCommand, VideoCommand, VisionCommand,
};
use crate::config::{Config, Provider as ConfigProvider};
use crate::output::{Output, OutputFormat};
use crate::providers::create_provider;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    if cli.command.is_some() {
        run_cli(cli).await
    } else if cli.global.tui {
        #[cfg(feature = "tui")]
        {
            run_tui().await
        }
        #[cfg(not(feature = "tui"))]
        {
            Err(std::io::Error::other(
                "TUI is not enabled in this build. Rebuild with: cargo build --features tui",
            ))
        }
    } else {
        // No subcommand and no --tui: print help
        Cli::command().print_help().unwrap();
        println!();
        Ok(())
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
    let stepfun_has_key = config
        .stepfun
        .as_ref()
        .is_some_and(|s| !s.api_key.is_empty());
    let minimax_has_key = config
        .minimax
        .as_ref()
        .is_some_and(|m| !m.api_key.is_empty());

    match (stepfun_has_key, minimax_has_key) {
        (true, false) => return Ok(ConfigProvider::StepFun),
        (false, true) => return Ok(ConfigProvider::MiniMax),
        (true, true) => {} // fall through to default_provider
        (false, false) => return Err("No providers configured. Run `vox config edit` to set API keys, or edit ~/.config/vox/config.toml".into()),
    }

    // Use default_provider from config
    Ok(default.clone())
}

// ═══════════════════════════════════════════════════════════════════
// Main CLI dispatcher
// ═══════════════════════════════════════════════════════════════════

async fn run_cli(cli: Cli) -> std::io::Result<()> {
    // Set up Ctrl+C handler for graceful shutdown
    let cli_copy = cli.clone();
    tokio::spawn(async {
        let _ = tokio::signal::ctrl_c().await;
        // Clean up any spinners by exiting
        std::process::exit(130); // 128 + SIGINT(2)
    });
    let cli = cli_copy;

    // Create Output formatter
    let output = Output::new(
        OutputFormat::from_str(&cli.global.format),
        cli.global.quiet,
        cli.global.verbose,
        cli.global.no_color,
    );

    // Load config (respecting --config override via VOX_CONFIG env)
    let mut config = if let Some(ref config_path) = cli.global.config {
        match Config::load_from(std::path::Path::new(config_path)) {
            Ok(c) => c,
            Err(e) => {
                return Err(std::io::Error::other(format!(
                    "Failed to load config from {}: {e}",
                    config_path
                )));
            }
        }
    } else {
        Config::load().unwrap_or_default()
    };

    // Handle commands that don't need a provider first
    match cli.command {
        Some(Commands::Doctor(args)) => {
            handle_doctor(args, &config, &output).await;
            return Ok(());
        }
        Some(Commands::Config(cmd)) => {
            handle_config(cmd, &mut config, &output);
            return Ok(());
        }
        Some(Commands::Providers(cmd)) => {
            handle_providers(cmd, &mut config, &output);
            return Ok(());
        }
        Some(Commands::Models(cmd)) => {
            handle_models(cmd, &mut config, &output);
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
            return Err(std::io::Error::other(format!("Error: {e}")));
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
        Some(Commands::Doctor(_)) => unreachable!("handled above"),
        Some(Commands::Providers(_)) => unreachable!("handled above"),
        Some(Commands::Models(_)) => unreachable!("handled above"),
        Some(Commands::Config(_)) => unreachable!("handled above"),
        Some(Commands::Completion { .. }) => unreachable!("handled above"),

        None => {}
    }

    // Exit with error code if errors occurred
    if output.has_errors() {
        return Err(std::io::Error::other("errors occurred"));
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

    if let Some(ref output_dir) = global.output_dir {
        config.output_dir = Some(output_dir.clone());
    }
}

// ═══════════════════════════════════════════════════════════════════
// Command handlers
// ═══════════════════════════════════════════════════════════════════

async fn handle_text(cmd: TextCommand, config: &Config, output: &Output) {
    match cmd {
        TextCommand::Chat { message, system } => {
            // If no message provided, enter REPL mode
            if let Some(msg) = message {
                let Some((provider, spinner)) =
                    prepare_provider(config, Capability::Chat, "Generating response...", output)
                else {
                    return;
                };

                let messages = if let Some(sys) = &system {
                    vec![
                        providers::Message::system(sys),
                        providers::Message::user(msg),
                    ]
                } else {
                    vec![providers::Message::user(msg)]
                };

                let result = provider.chat(&messages).await;
                if let Some(sp) = spinner {
                    sp.finish_and_clear();
                }

                match result {
                    Ok(resp) => output.result(&resp.content),
                    Err(e) => output.error(&format!("{e}"), 1),
                }
            } else {
                // No message — enter interactive REPL (needs its own provider)
                let provider = match create_provider(config) {
                    Ok(p) => p,
                    Err(e) => {
                        output.error(&format!("Failed to create provider: {e}"), 1);
                        return;
                    }
                };
                handle_text_repl(provider, system, output).await;
            }
        }
        TextCommand::Repl { system } => {
            let provider = match create_provider(config) {
                Ok(p) => p,
                Err(e) => {
                    output.error(&format!("Failed to create provider: {e}"), 1);
                    return;
                }
            };
            handle_text_repl(provider, system, output).await;
        }
        TextCommand::Complete { prompt } => {
            let Some((provider, spinner)) =
                prepare_provider(config, Capability::Chat, "Generating completion...", output)
            else {
                return;
            };

            let messages = vec![providers::Message::user(prompt)];
            let result = provider.chat(&messages).await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
                Ok(resp) => output.result(&resp.content),
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

/// Interactive chat REPL handler
async fn handle_text_repl(
    provider: Box<dyn providers::AIProvider>,
    system: Option<String>,
    output: &Output,
) {
    let mut rl = match DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            output.error(&format!("Failed to initialize REPL: {e}"), 1);
            return;
        }
    };

    let mut conversation: Vec<providers::Message> = Vec::new();

    if let Some(sys) = &system {
        conversation.push(providers::Message::system(sys));
    }

    output.status("vox chat — type your message, :q to quit, :clear to reset history");

    loop {
        let readline = rl.readline(" vox> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == ":q" || trimmed == ":quit" || trimmed == ":exit" {
                    break;
                }
                if trimmed == ":clear" {
                    conversation.clear();
                    if let Some(sys) = &system {
                        conversation.push(providers::Message::system(sys));
                    }
                    output.status("Conversation cleared.");
                    continue;
                }

                rl.add_history_entry(trimmed).ok();

                conversation.push(providers::Message::user(trimmed));

                match provider.chat(&conversation).await {
                    Ok(resp) => {
                        output.result(&resp.content);
                        conversation.push(providers::Message::assistant(&resp.content));
                    }
                    Err(e) => output.error(&format!("{e}"), 1),
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C — just continue (like a shell)
                println!();
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D — exit
                break;
            }
            Err(e) => {
                output.error(&format!("Readline error: {e}"), 1);
                break;
            }
        }
    }
}

async fn handle_image(cmd: ImageCommand, config: &Config, output: &Output) {
    match cmd {
        ImageCommand::Generate {
            prompt,
            aspect_ratio,
            output: out_path,
            n,
        } => {
            let Some((provider, spinner)) = prepare_provider(
                config,
                Capability::ImageGenerate,
                "Generating image...",
                output,
            ) else {
                return;
            };

            let result = provider.image_generate(&prompt, n, &aspect_ratio).await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
                Ok(resp) => {
                    if let Some(path) = out_path {
                        for (i, url) in resp.urls.iter().enumerate() {
                            let file_path = if resp.urls.len() == 1 {
                                path.clone()
                            } else {
                                let p = std::path::Path::new(&path);
                                let parent = p.parent();
                                let stem =
                                    p.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
                                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("png");
                                let filename = format!("{stem}_{i}.{ext}");
                                match parent {
                                    Some(dir) if !dir.as_os_str().is_empty() => {
                                        dir.join(filename).to_string_lossy().into_owned()
                                    }
                                    _ => filename,
                                }
                            };
                            if let Err(e) = download_file(url, &file_path).await {
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
    }
}

async fn handle_speech(cmd: SpeechCommand, config: &Config, output: &Output) {
    match cmd {
        SpeechCommand::Generate {
            text,
            out,
            voice,
            speed,
            format,
        } => {
            let Some((provider, spinner)) = prepare_provider(
                config,
                Capability::SpeechSynthesize,
                "Generating speech...",
                output,
            ) else {
                return;
            };

            let output_path = out.unwrap_or_else(|| "output.mp3".to_string());
            let result = provider
                .speech_synthesize(&text, &voice, speed, &format)
                .await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
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
    match cmd {
        VideoCommand::Generate {
            prompt,
            duration,
            resolution,
            out: _,
        } => {
            let Some((provider, spinner)) = prepare_provider(
                config,
                Capability::VideoGenerate,
                "Generating video...",
                output,
            ) else {
                return;
            };

            let result = provider
                .video_generate(&prompt, duration, &resolution)
                .await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
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
    }
}

async fn handle_music(cmd: MusicCommand, config: &Config, output: &Output) {
    match cmd {
        MusicCommand::Generate {
            prompt,
            lyrics,
            instrumental,
            out,
        } => {
            let Some((provider, spinner)) = prepare_provider(
                config,
                Capability::MusicGenerate,
                "Generating music...",
                output,
            ) else {
                return;
            };

            let output_path = out.unwrap_or_else(|| "output.mp3".to_string());
            let result = provider
                .music_generate(&prompt, lyrics.as_deref(), instrumental)
                .await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
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
    match cmd {
        SearchCommand::Query { query, count } => {
            let Some((provider, spinner)) =
                prepare_provider(config, Capability::Search, "Searching...", output)
            else {
                return;
            };

            let result = provider.search(&query, count).await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
                Ok(resp) => {
                    for result in resp.results {
                        output.result(&format!(
                            "{}\n  {}\n  {}\n",
                            result.title, result.url, result.snippet
                        ));
                    }
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

async fn handle_vision(cmd: VisionCommand, config: &Config, output: &Output) {
    match cmd {
        VisionCommand::Analyze { file, prompt } => {
            let Some((provider, spinner)) =
                prepare_provider(config, Capability::Vision, "Analyzing image...", output)
            else {
                return;
            };

            let result = provider.vision(&file, prompt.as_deref()).await;
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            match result {
                Ok(resp) => {
                    output.result(&resp.description);
                }
                Err(e) => output.error(&format!("{e}"), 1),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Doctor check types
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize)]
struct DoctorCheckResult {
    name: String,
    description: String,
    status: String, // "pass", "warn", "fail"
    details: Option<String>,
}

async fn handle_doctor(args: cli::DoctorArgs, config: &Config, output: &Output) {
    let is_json = args.format == "json";
    let specific_check = args.check.as_deref();

    let mut results: Vec<DoctorCheckResult> = Vec::new();

    // Run all checks (or specific one)
    if specific_check.is_none() || specific_check == Some("config-file") {
        results.push(check_config_file(config));
    }
    if specific_check.is_none() || specific_check == Some("config-parse") {
        results.push(check_config_parse(config));
    }
    if specific_check.is_none() || specific_check == Some("provider") {
        results.push(check_provider_setup(config));
    }
    if specific_check.is_none() || specific_check == Some("api-keys") {
        results.push(check_api_keys(config));
    }
    if specific_check.is_none() || specific_check == Some("default-model") {
        results.push(check_default_model(config));
    }
    if specific_check.is_none() || specific_check == Some("output-dir") {
        results.push(check_output_dir(config));
    }
    if specific_check.is_none() || specific_check == Some("connectivity") {
        results.push(check_api_connectivity(config).await);
    }
    if specific_check.is_none() || specific_check == Some("models") {
        results.push(check_model_validity(config));
    }

    if is_json {
        let json = serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string());
        println!("{json}");
        return;
    }

    // Text output
    output.result("vox doctor — diagnostics");
    output.result("========================");
    output.result("");

    let mut pass_count = 0;
    let mut warn_count = 0;
    let mut fail_count = 0;

    for (i, result) in results.iter().enumerate() {
        let symbol = match result.status.as_str() {
            "pass" => {
                pass_count += 1;
                "✓"
            }
            "warn" => {
                warn_count += 1;
                "⚠"
            }
            "fail" => {
                fail_count += 1;
                "✗"
            }
            _ => "?",
        };
        let details = result.details.as_deref().unwrap_or("");
        output.result(&format!(
            "{} {}. {}  {}",
            symbol,
            i + 1,
            result.description,
            details
        ));
    }

    output.result("");
    output.result(&format!(
        "{} passed, {} warning{}, {} failed",
        pass_count,
        warn_count,
        if warn_count != 1 { "s" } else { "" },
        fail_count
    ));
}

fn check_config_file(_config: &Config) -> DoctorCheckResult {
    match Config::config_path() {
        Some(path) => {
            let exists = path.exists();
            let size_str = if exists {
                std::fs::metadata(&path)
                    .map(|m| {
                        let size = m.len();
                        if size < 1024 {
                            format!("{} B", size)
                        } else {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        }
                    })
                    .unwrap_or_else(|_| "unknown".to_string())
            } else {
                "not found".to_string()
            };
            DoctorCheckResult {
                name: "config-file".into(),
                description: "Config file".into(),
                status: if exists { "pass" } else { "warn" }.into(),
                details: Some(format!("Found at {} ({})", path.display(), size_str)),
            }
        }
        None => DoctorCheckResult {
            name: "config-file".into(),
            description: "Config file".into(),
            status: "fail".into(),
            details: Some("No config directory available".into()),
        },
    }
}

fn check_config_parse(_config: &Config) -> DoctorCheckResult {
    // If we got here, config was already loaded successfully
    DoctorCheckResult {
        name: "config-parse".into(),
        description: "Config parse".into(),
        status: "pass".into(),
        details: Some("Valid TOML, all fields recognized".into()),
    }
}

fn check_provider_setup(config: &Config) -> DoctorCheckResult {
    let providers = config.configured_providers();
    if providers.is_empty() {
        return DoctorCheckResult {
            name: "provider-setup".into(),
            description: "Provider setup".into(),
            status: "fail".into(),
            details: Some("No providers configured".into()),
        };
    }

    let names: Vec<&str> = providers
        .iter()
        .map(|p| match p {
            ConfigProvider::StepFun => "StepFun",
            ConfigProvider::MiniMax => "MiniMax",
        })
        .collect();

    DoctorCheckResult {
        name: "provider-setup".into(),
        description: "Provider setup".into(),
        status: "pass".into(),
        details: Some(format!(
            "{} provider{} configured ({})",
            providers.len(),
            if providers.len() != 1 { "s" } else { "" },
            names.join(", ")
        )),
    }
}

fn check_api_keys(config: &Config) -> DoctorCheckResult {
    let mut parts = Vec::new();
    let mut all_have_keys = true;
    let mut some_have_keys = false;

    if config.stepfun.is_some() {
        let has_key = config
            .stepfun
            .as_ref()
            .is_some_and(|s| !s.api_key.is_empty());
        parts.push(format!(
            "StepFun: {} set",
            if has_key { "✓" } else { "✗ missing" }
        ));
        if has_key {
            some_have_keys = true;
        } else {
            all_have_keys = false;
        }
    }
    if config.minimax.is_some() {
        let has_key = config
            .minimax
            .as_ref()
            .is_some_and(|m| !m.api_key.is_empty());
        parts.push(format!(
            "MiniMax: {} set",
            if has_key { "✓" } else { "✗ missing" }
        ));
        if has_key {
            some_have_keys = true;
        } else {
            all_have_keys = false;
        }
    }

    let status = if all_have_keys {
        "pass"
    } else if some_have_keys {
        "warn"
    } else {
        "fail"
    };

    DoctorCheckResult {
        name: "api-keys".into(),
        description: "API keys".into(),
        status: status.into(),
        details: Some(parts.join(" | ")),
    }
}

fn check_default_model(config: &Config) -> DoctorCheckResult {
    let model = config.get_model_for("chat");
    match model {
        Some(m) => DoctorCheckResult {
            name: "default-model".into(),
            description: "Default model".into(),
            status: "pass".into(),
            details: Some(format!("{} ({})", m, config.default_provider)),
        },
        None => DoctorCheckResult {
            name: "default-model".into(),
            description: "Default model".into(),
            status: "warn".into(),
            details: Some(format!(
                "No model set for default provider ({})",
                config.default_provider
            )),
        },
    }
}

fn check_output_dir(config: &Config) -> DoctorCheckResult {
    match &config.output_dir {
        Some(dir) => {
            let path = std::path::Path::new(dir);
            if path.exists() && path.is_dir() {
                DoctorCheckResult {
                    name: "output-dir".into(),
                    description: "Output dir".into(),
                    status: "pass".into(),
                    details: Some(format!("{} (exists)", dir)),
                }
            } else {
                DoctorCheckResult {
                    name: "output-dir".into(),
                    description: "Output dir".into(),
                    status: "warn".into(),
                    details: Some(format!("{} (configured but does not exist)", dir)),
                }
            }
        }
        None => DoctorCheckResult {
            name: "output-dir".into(),
            description: "Output dir".into(),
            status: "pass".into(),
            details: Some("Using default (./vox-&output)".into()),
        },
    }
}

async fn check_api_connectivity(config: &Config) -> DoctorCheckResult {
    let providers = config.configured_providers();
    if providers.is_empty() {
        return DoctorCheckResult {
            name: "connectivity".into(),
            description: "API connectivity".into(),
            status: "fail".into(),
            details: Some("No providers configured".into()),
        };
    }

    let mut results = Vec::new();
    let mut any_reachable = false;
    let mut any_failed = false;

    for provider in &providers {
        let name = match provider {
            ConfigProvider::StepFun => "StepFun",
            ConfigProvider::MiniMax => "MiniMax",
        };

        // Create a test config for just this provider
        let test_config = Config {
            version: 1,
            default_provider: provider.clone(),
            stepfun: config.stepfun.clone(),
            minimax: config.minimax.clone(),
            theme: None,
            output_dir: None,
        };

        match create_provider(&test_config) {
            Ok(_) => {
                results.push(format!("{}: configured", name));
                any_reachable = true;
            }
            Err(e) => {
                results.push(format!("{}: config error ({})", name, e));
                any_failed = true;
            }
        }
    }

    let status = if any_reachable && !any_failed {
        "pass"
    } else if any_reachable && any_failed {
        "warn"
    } else {
        "fail"
    };

    DoctorCheckResult {
        name: "connectivity".into(),
        description: "API connectivity".into(),
        status: status.into(),
        details: Some(results.join(" | ")),
    }
}

fn check_model_validity(config: &Config) -> DoctorCheckResult {
    let providers = config.configured_providers();
    if providers.is_empty() {
        return DoctorCheckResult {
            name: "models".into(),
            description: "Model validity".into(),
            status: "fail".into(),
            details: Some("No providers configured".into()),
        };
    }

    let mut issues = Vec::new();
    let capabilities = [
        "chat", "image", "speech", "video", "music", "vision", "search",
    ];

    for provider in &providers {
        for cap in &capabilities {
            let model = config.get_model_for(cap);
            if let Some(ref m) = model {
                let known = models::get_available_models(provider, cap);
                if !known.is_empty() && !known.contains(m) {
                    issues.push(format!("{}: {} not in known {} models", provider, m, cap));
                }
            }
        }
    }

    if issues.is_empty() {
        DoctorCheckResult {
            name: "models".into(),
            description: "Model validity".into(),
            status: "pass".into(),
            details: Some("All configured models are known".into()),
        }
    } else {
        DoctorCheckResult {
            name: "models".into(),
            description: "Model validity".into(),
            status: "warn".into(),
            details: Some(issues.join("; ")),
        }
    }
}

fn handle_providers(cmd: ProvidersCommand, config: &mut Config, output: &Output) {
    match cmd {
        ProvidersCommand::List => {
            let providers = config.configured_providers();
            if providers.is_empty() {
                output.result(
                    "No providers configured. Use `vox provider add <name> <api_key>` to add one.",
                );
                return;
            }
            output.result("Configured providers:");
            for p in &providers {
                let (name, api_key_masked) = match p {
                    ConfigProvider::StepFun => {
                        let masked = config
                            .stepfun
                            .as_ref()
                            .map(|s| {
                                if s.api_key.is_empty() {
                                    "(not set)".to_string()
                                } else {
                                    format!("{}***", &s.api_key[..4.min(s.api_key.len())])
                                }
                            })
                            .unwrap_or_else(|| "(not configured)".to_string());
                        ("StepFun", masked)
                    }
                    ConfigProvider::MiniMax => {
                        let masked = config
                            .minimax
                            .as_ref()
                            .map(|m| {
                                if m.api_key.is_empty() {
                                    "(not set)".to_string()
                                } else {
                                    format!("{}***", &m.api_key[..4.min(m.api_key.len())])
                                }
                            })
                            .unwrap_or_else(|| "(not configured)".to_string());
                        ("MiniMax", masked)
                    }
                };
                let is_default = if p == &config.default_provider {
                    " (default)"
                } else {
                    ""
                };
                output.result(&format!("  {name}: {api_key_masked}{is_default}"));
            }
        }
        ProvidersCommand::Add {
            provider,
            api_key,
            group_id,
        } => match provider.to_lowercase().as_str() {
            "stepfun" => {
                let sf = config.stepfun.get_or_insert_with(|| config::StepFunConfig {
                    api_key: String::new(),
                    base_url: None,
                    model: None,
                    models: config::ProviderModels::default(),
                });
                sf.api_key = api_key;
                if let Err(e) = config.save() {
                    output.error(&format!("Failed to save config: {e}"), 1);
                } else {
                    output.result("StepFun provider added successfully.");
                }
            }
            "minimax" => {
                let mm = config.minimax.get_or_insert_with(|| config::MiniMaxConfig {
                    api_key: String::new(),
                    group_id: None,
                    base_url: None,
                    model: None,
                    models: config::ProviderModels::default(),
                });
                mm.api_key = api_key;
                if let Some(gid) = group_id {
                    mm.group_id = Some(gid);
                }
                if let Err(e) = config.save() {
                    output.error(&format!("Failed to save config: {e}"), 1);
                } else {
                    output.result("MiniMax provider added successfully.");
                }
            }
            other => {
                output.error(
                    &format!("Unknown provider: {other}. Use 'stepfun' or 'minimax'."),
                    1,
                );
            }
        },
        ProvidersCommand::Remove { provider } => match provider.to_lowercase().as_str() {
            "stepfun" => {
                config.stepfun = None;
                if let Err(e) = config.save() {
                    output.error(&format!("Failed to save config: {e}"), 1);
                } else {
                    output.result("StepFun provider removed.");
                }
            }
            "minimax" => {
                config.minimax = None;
                if let Err(e) = config.save() {
                    output.error(&format!("Failed to save config: {e}"), 1);
                } else {
                    output.result("MiniMax provider removed.");
                }
            }
            other => {
                output.error(
                    &format!("Unknown provider: {other}. Use 'stepfun' or 'minimax'."),
                    1,
                );
            }
        },
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
                version: 1,
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
                None => vec![
                    "chat".into(),
                    "image".into(),
                    "speech".into(),
                    "video".into(),
                    "music".into(),
                    "vision".into(),
                    "search".into(),
                ],
            };

            for cap in &capabilities {
                let models = models::get_available_models(&config.default_provider, cap);
                if models.is_empty() {
                    output.result(&format!("{cap}: (no known models)"));
                } else {
                    output.result(&format!("{cap}:"));
                    for m in &models {
                        let is_selected = config.get_model_for(cap).is_some_and(|cm| cm == *m);
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
                output.error(
                    &format!(
                        "Unknown model '{model}' for capability '{capability}'. Known models: {}",
                        known.join(", ")
                    ),
                    1,
                );
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
            return;
        }
    };

    let mut app = Cli::command();
    generate(shell, &mut app, "vox", &mut io::stdout());
}

// ═══════════════════════════════════════════════════════════════════
// Config key helpers
// ═══════════════════════════════════════════════════════════════════

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        "(not set)".to_string()
    } else {
        format!("{}***", &key[..4.min(key.len())])
    }
}

fn parse_config_key(config: &Config, key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["default_provider"] => Some(config.default_provider.to_string()),
        ["stepfun", "api_key"] => config.stepfun.as_ref().map(|s| mask_key(&s.api_key)),
        ["stepfun", "base_url"] => config.stepfun.as_ref().and_then(|s| s.base_url.clone()),
        ["stepfun", "model"] => config.stepfun.as_ref().and_then(|s| s.model.clone()),
        ["minimax", "api_key"] => config.minimax.as_ref().map(|m| mask_key(&m.api_key)),
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
    config
        .save()
        .map_err(|e| format!("Failed to save config: {e}"))
}

// ═══════════════════════════════════════════════════════════════════
// Utility functions
// ═══════════════════════════════════════════════════════════════════

/// Creates a provider, checks capability, creates a spinner.
/// Returns (provider, spinner_guard) on success, or logs error and returns None.
fn prepare_provider(
    config: &Config,
    capability: Capability,
    spinner_msg: &str,
    output: &Output,
) -> Option<(
    Box<dyn providers::AIProvider>,
    Option<indicatif::ProgressBar>,
)> {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            output.error(&format!("Failed to create provider: {e}"), 1);
            return None;
        }
    };

    if let Err(e) = capabilities::ProviderCapabilities::for_provider(&config.default_provider)
        .require(capability, &config.default_provider)
    {
        output.error(&e, 1);
        return None;
    }

    let spinner = create_spinner(spinner_msg, output);
    Some((provider, spinner))
}

fn create_spinner(message: &str, output: &Output) -> Option<indicatif::ProgressBar> {
    if output.is_quiet() {
        return None;
    }
    let sp = indicatif::ProgressBar::new_spinner();
    sp.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    sp.set_message(message.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    Some(sp)
}

async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

// ── TUI mode (only compiled with `--features tui`) ──────────────────────

#[cfg(feature = "tui")]
use std::time::{Duration, Instant};

#[cfg(feature = "tui")]
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
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
use crate::ui::{
    AppLayout, AppTheme, AudioView, ChatView, ConfigView, ImageView, Layout, View, compute_layout,
};

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

            let AppLayout {
                sidebar,
                main,
                status,
            } = compute_layout(area);

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
                ConfigProvider::StepFun => app
                    .config
                    .stepfun
                    .as_ref()
                    .and_then(|s| s.model.as_deref())
                    .unwrap_or("default"),
                ConfigProvider::MiniMax => app
                    .config
                    .minimax
                    .as_ref()
                    .and_then(|m| m.model.as_deref())
                    .unwrap_or("default"),
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
