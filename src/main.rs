mod app;
mod config;
mod input;
pub mod minimax;
pub mod provider;
mod stepfun;
pub mod ui;
pub mod cli;
pub mod command;

use std::{
    io,
    time::{Duration, Instant},
};

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::AppState;
use crate::cli::Cli;
use crate::config::{Config, Provider as ConfigProvider};
use crate::input::InputMode;
use crate::provider::{create_provider};
use crate::ui::{AudioView, ChatView, ConfigView, ImageView, Layout, View, AppTheme, AppLayout, compute_layout};

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.command.is_some() {
        run_cli(cli).await
    } else {
        run_tui().await
    }
}

async fn run_cli(cli: Cli) -> io::Result<()> {
    let mut config = Config::load().unwrap_or_default();

    let provider_name = cli.global.provider.to_lowercase();
    match provider_name.as_str() {
        "stepfun" => config.default_provider = ConfigProvider::StepFun,
        "minimax" => config.default_provider = ConfigProvider::MiniMax,
        _ => {
            eprintln!("Unknown provider: {}", cli.global.provider);
            return Ok(());
        }
    }

    if let Some(model) = &cli.global.model {
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

    if let Some(api_key) = &cli.global.api_key {
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

    if let Some(group_id) = &cli.global.group_id
        && let Some(ref mut minimax) = config.minimax
    {
        minimax.group_id = Some(group_id.clone());
    }

    if let Some(base_url) = &cli.global.base_url {
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

    let provider = match create_provider(&config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create provider: {e}");
            return Ok(());
        }
    };

    match cli.command {
        Some(cli::Commands::Image { prompt, aspect_ratio, output, n }) => {
            match provider.image_generate(&prompt, n, &aspect_ratio).await {
                Ok(resp) => {
                    if let Some(output_path) = output {
                        for (i, url) in resp.urls.iter().enumerate() {
                            let path = if resp.urls.len() == 1 {
                                output_path.clone()
                            } else {
                                let p = std::path::Path::new(&output_path);
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
                            if let Err(e) = download_file(url, &path) {
                                eprintln!("Failed to download {url}: {e}");
                            } else {
                                println!("Saved to {path}");
                            }
                        }
                    } else {
                        for url in resp.urls {
                            println!("{url}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        Some(cli::Commands::Speech { text, out, voice, speed, format }) => {
            let output_path = out.unwrap_or_else(|| "output.mp3".to_string());
            match provider.speech_synthesize(&text, &voice, speed, &format).await {
                Ok(resp) => {
                    if let Err(e) = std::fs::write(&output_path, &resp.audio_data) {
                        eprintln!("Failed to write audio file: {e}");
                    } else {
                        println!("Saved to {output_path}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        Some(cli::Commands::Video { prompt, duration, resolution }) => {
            match provider.video_generate(&prompt, duration, &resolution).await {
                Ok(resp) => {
                    println!("Task ID: {}", resp.task_id);
                    println!("Status: {}", resp.status);
                    if let Some(url) = resp.video_url {
                        println!("Video URL: {url}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        Some(cli::Commands::Music { prompt, lyrics, instrumental, out }) => {
            let output_path = out.unwrap_or_else(|| "output.mp3".to_string());
            match provider.music_generate(&prompt, lyrics.as_deref(), instrumental).await {
                Ok(resp) => {
                    if let Err(e) = std::fs::write(&output_path, &resp.audio_data) {
                        eprintln!("Failed to write audio file: {e}");
                    } else {
                        println!("Saved to {output_path}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        Some(cli::Commands::Search { query, count }) => {
            match provider.search(&query, count).await {
                Ok(resp) => {
                    for result in resp.results {
                        println!("{}\n  {}\n  {}\n", result.title, result.url, result.snippet);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        Some(cli::Commands::Vision { file, prompt }) => {
            match provider.vision(&file, prompt.as_deref()).await {
                Ok(resp) => {
                    println!("{}", resp.description);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        None => {}
    }

    Ok(())
}

fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;
    std::fs::write(path, bytes)?;
    Ok(())
}

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
