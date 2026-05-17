mod config;
mod input;
pub mod minimax;
pub mod minimax_api;
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
use ui::{AudioView, ChatView, ConfigView, ImageView, Layout, View, AppTheme, AppLayout, compute_layout, widget::ChatMessage};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::cli::Cli;
use crate::command::SlashCommand;
use crate::config::{Config, Provider as ConfigProvider};
use crate::provider::create_provider;

pub struct AppState {
    pub running: bool,
    pub status: String,
    pub current_view: View,
    pub work_count: u32,
    pub input: input::TextInputState,
    pub input_focused: bool,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
    pub scroll_offset: u16,
    pub config: Config,
    work_tx: tokio::sync::mpsc::Sender<()>,
    work_rx: tokio::sync::mpsc::Receiver<()>,
}

impl AppState {
    pub fn new() -> Self {
        let (work_tx, work_rx) = tokio::sync::mpsc::channel(1);
        let config = Config::load().unwrap_or_default();
        Self {
            running: true,
            status: "vox-tui — 1-4: switch view, Tab/⇧Tab: next/prev, Enter: send, q/Ctrl+C: quit".into(),
            current_view: View::default(),
            work_count: 0,
            input: input::TextInputState::new(),
            input_focused: false,
            messages: Vec::new(),
            is_streaming: false,
            scroll_offset: 0,
            config,
            work_tx,
            work_rx,
        }
    }

    pub fn tick(&mut self) {
        while let Ok(()) = self.work_rx.try_recv() {
            if self.is_streaming {
                self.finish_stream();
            }
            self.work_count += 1;
            self.status = format!("vox-tui — {count} async tasks completed", count = self.work_count);
        }
    }

    pub fn trigger_work(&self) {
        let tx = self.work_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = tx.send(()).await;
        });
    }

    pub fn next_view(&mut self) {
        self.current_view = self.current_view.next();
    }

    pub fn prev_view(&mut self) {
        let views = View::all();
        let idx = views.iter().position(|v| v == &self.current_view).unwrap_or(0);
        let new_idx = if idx == 0 { views.len() - 1 } else { idx - 1 };
        self.current_view = views[new_idx];
    }

    pub fn switch_view(&mut self, idx: usize) {
        let views = View::all();
        if idx < views.len() {
            self.current_view = views[idx];
        }
    }

    pub fn handle_input(&mut self, action: input::InputAction) {
        if self.is_streaming {
            match action {
                input::InputAction::Escape => {
                    self.is_streaming = false;
                    self.status = "Streaming cancelled".into();
                }
                input::InputAction::Quit => self.running = false,
                _ => {}
            }
            return;
        }

        if self.input_focused {
            match action {
                input::InputAction::Char(c) => self.input.insert_char(c),
                input::InputAction::Backspace => self.input.backspace(),
                input::InputAction::Delete => self.input.delete(),
                input::InputAction::Home => self.input.move_home(),
                input::InputAction::End => self.input.move_end(),
                input::InputAction::Left => self.input.move_left(),
                input::InputAction::Right => self.input.move_right(),
                input::InputAction::Submit if !self.input.content.is_empty() => {
                    self.send_message();
                }
                input::InputAction::Escape => {
                    self.input.clear();
                    self.input_focused = false;
                }
                input::InputAction::ScrollUp => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
                input::InputAction::ScrollDown => {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                }
                input::InputAction::NextView => {
                    self.input_focused = false;
                    self.next_view();
                }
                _ => {}
            }
        } else {
            match action {
                input::InputAction::Quit => self.running = false,
                input::InputAction::NextView => self.next_view(),
                input::InputAction::PrevView => self.prev_view(),
                input::InputAction::SwitchView(idx) => self.switch_view(idx),
                input::InputAction::Submit => {
                    self.input_focused = true;
                }
                input::InputAction::Escape => {
                    self.input.clear();
                    self.input_focused = false;
                }
                _ => {}
            }
        }
    }

    pub fn send_message(&mut self) {
        let content = self.input.submit();

        // Check for slash commands before sending to API
        if let Some(cmd) = command::parse_slash_command(&content) {
            self.handle_slash_command(cmd);
            return;
        }

        self.messages.push(ChatMessage::user(&content));
        self.input_focused = false;
        self.is_streaming = true;
        self.status = "Waiting for response...".into();
        self.scroll_offset = u16::MAX;

        let tx = self.work_tx.clone();

        tokio::spawn(async move {
            // Simulated response — real API integration will replace this
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = tx.send(()).await;
        });
    }

    pub fn append_stream(&mut self, text: &str) {
        if let Some(last) = self.messages.last_mut() {
            last.content.push_str(text);
        }
    }

    pub fn finish_stream(&mut self) {
        self.is_streaming = false;
        self.status = "Ready".into();
    }

    pub fn handle_slash_command(&mut self, cmd: SlashCommand) {
        self.input_focused = false;

        match cmd {
            SlashCommand::Provider { name } => {
                let provider_name = name.to_lowercase();
                let new_provider = match provider_name.as_str() {
                    "stepfun" => ConfigProvider::StepFun,
                    "minimax" => ConfigProvider::MiniMax,
                    _ => {
                        self.messages.push(ChatMessage::system(
                            format!("Unknown provider: {name}. Available: stepfun, minimax"),
                        ));
                        return;
                    }
                };

                if !self.config.has_provider(&new_provider) {
                    self.messages.push(ChatMessage::system(format!(
                        "Provider {name} is not configured. Please add it to your config."
                    )));
                    return;
                }

                self.config.default_provider = new_provider;
                let provider_display = match self.config.default_provider {
                    ConfigProvider::StepFun => "StepFun",
                    ConfigProvider::MiniMax => "MiniMax",
                };
                self.messages.push(ChatMessage::system(format!(
                    "Switched to provider: {provider_display}",
                )));
            }
            SlashCommand::Model { name } => {
                let model_name = if name.is_empty() {
                    "default".to_string()
                } else {
                    name.clone()
                };

                match self.config.default_provider {
                    ConfigProvider::StepFun => {
                        if let Some(ref mut stepfun) = self.config.stepfun {
                            stepfun.model = Some(model_name.clone());
                        }
                    }
                    ConfigProvider::MiniMax => {
                        if let Some(ref mut minimax) = self.config.minimax {
                            minimax.model = Some(model_name.clone());
                        }
                    }
                }
                self.messages.push(ChatMessage::system(format!(
                    "Model set to: {model_name}",
                )));
            }
            SlashCommand::Help => {
                let help_text = "Available slash commands:\n\
                    /provider <name> - Switch AI provider (stepfun, minimax)\n\
                    /model <name> - Set model for current provider\n\
                    /help - Show this help message\n\
                    /clear - Clear chat history\n\
                    /save - Save conversation (not yet implemented)\n\
                    /status - Show current provider and model info";
                self.messages.push(ChatMessage::system(help_text));
            }
            SlashCommand::Clear => {
                self.messages.clear();
                self.messages.push(ChatMessage::system("Chat history cleared".to_string()));
            }
            SlashCommand::Save => {
                self.messages.push(ChatMessage::system("Save not yet implemented".to_string()));
            }
            SlashCommand::Status => {
                let provider_info = match self.config.default_provider {
                    ConfigProvider::StepFun => {
                        let model = self.config.stepfun.as_ref()
                            .and_then(|s| s.model.as_deref())
                            .unwrap_or("default");
                        format!("Provider: StepFun\nModel: {model}")
                    }
                    ConfigProvider::MiniMax => {
                        let model = self.config.minimax.as_ref()
                            .and_then(|m| m.model.as_deref())
                            .unwrap_or("default");
                        format!("Provider: MiniMax\nModel: {model}")
                    }
                };
                self.messages.push(ChatMessage::system(provider_info));
            }
            SlashCommand::Unknown(cmd) => {
                self.messages.push(ChatMessage::system(format!(
                    "Unknown command: /{cmd}. Type /help for available commands."
                )));
            }
        }
        // Don't set is_streaming for slash commands
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let app = AppState::new();
        assert!(app.running);
        assert_eq!(app.current_view, View::Chat);
        assert_eq!(app.work_count, 0);
        assert!(app.messages.is_empty());
        assert!(!app.is_streaming);
        assert!(!app.input_focused);
    }

    #[test]
    fn test_app_state_default() {
        let app = AppState::default();
        assert!(app.running);
    }

    #[test]
    fn test_app_state_next_view() {
        let mut app = AppState::new();
        assert_eq!(app.current_view, View::Chat);
        app.next_view();
        assert_eq!(app.current_view, View::Image);
        app.next_view();
        assert_eq!(app.current_view, View::Audio);
        app.next_view();
        assert_eq!(app.current_view, View::Config);
        app.next_view();
        assert_eq!(app.current_view, View::Chat);
    }

    #[test]
    fn test_app_state_prev_view() {
        let mut app = AppState::new();
        app.prev_view();
        assert_eq!(app.current_view, View::Config);
        app.prev_view();
        assert_eq!(app.current_view, View::Audio);
    }

    #[test]
    fn test_app_state_switch_view() {
        let mut app = AppState::new();
        app.switch_view(2);
        assert_eq!(app.current_view, View::Audio);
        app.switch_view(0);
        assert_eq!(app.current_view, View::Chat);
        app.switch_view(99);
        assert_eq!(app.current_view, View::Chat);
    }

    #[test]
    fn test_app_state_handle_input_quit() {
        let mut app = AppState::new();
        app.handle_input(input::InputAction::Quit);
        assert!(!app.running);
    }

    #[test]
    fn test_app_state_handle_input_view_switch() {
        let mut app = AppState::new();
        app.handle_input(input::InputAction::SwitchView(1));
        assert_eq!(app.current_view, View::Image);
        app.handle_input(input::InputAction::SwitchView(3));
        assert_eq!(app.current_view, View::Config);
    }

    #[test]
    fn test_app_state_handle_input_focus() {
        let mut app = AppState::new();
        assert!(!app.input_focused);
        app.handle_input(input::InputAction::Submit);
        assert!(app.input_focused);
    }

    #[test]
    fn test_app_state_handle_input_escape() {
        let mut app = AppState::new();
        app.input.insert_char('h');
        app.input_focused = true;
        app.handle_input(input::InputAction::Escape);
        assert!(!app.input_focused);
        assert!(app.input.content.is_empty());
    }

    #[test]
    fn test_app_state_handle_input_typing() {
        let mut app = AppState::new();
        app.input_focused = true;
        app.handle_input(input::InputAction::Char('h'));
        app.handle_input(input::InputAction::Char('i'));
        assert_eq!(app.input.content, "hi");
    }

    #[tokio::test]
    async fn test_app_state_handle_input_submit_message() {
        let mut app = AppState::new();
        app.input_focused = true;
        app.handle_input(input::InputAction::Char('h'));
        app.handle_input(input::InputAction::Char('i'));
        app.handle_input(input::InputAction::Submit);
        assert!(!app.input_focused);
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "hi");
        assert!(app.is_streaming);
    }

    #[test]
    fn test_app_state_tick() {
        let mut app = AppState::new();
        app.tick();
        assert_eq!(app.work_count, 0);
    }

    #[test]
    fn test_app_state_tick_finishes_stream() {
        let mut app = AppState::new();
        app.is_streaming = true;
        app.tick();
        assert!(app.is_streaming);
    }

    #[test]
    fn test_app_state_scroll() {
        let mut app = AppState::new();
        assert_eq!(app.scroll_offset, 0);
        app.input_focused = true;
        app.handle_input(input::InputAction::ScrollDown);
        assert_eq!(app.scroll_offset, 1);
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_app_state_append_stream() {
        let mut app = AppState::new();
        app.messages.push(ChatMessage::assistant("Hello"));
        app.append_stream(" World");
        assert_eq!(app.messages[0].content, "Hello World");
    }

    #[test]
    fn test_app_state_finish_stream() {
        let mut app = AppState::new();
        app.is_streaming = true;
        app.status = "Streaming...".into();
        app.finish_stream();
        assert!(!app.is_streaming);
        assert_eq!(app.status, "Ready");
    }

    // Slash command tests

    #[test]
    fn test_handle_slash_command_provider() {
        let mut app = AppState::new();
        app.config = Config {
            default_provider: ConfigProvider::StepFun,
            stepfun: Some(crate::config::StepFunConfig {
                api_key: "test".to_string(),
                base_url: None,
                model: None,
            }),
            minimax: Some(crate::config::MiniMaxConfig {
                api_key: "test".to_string(),
                group_id: None,
                base_url: None,
                model: None,
            }),
            theme: None,
        };

        app.handle_slash_command(SlashCommand::Provider { name: "minimax".to_string() });
        assert_eq!(app.config.default_provider, ConfigProvider::MiniMax);
        assert!(!app.is_streaming);
        assert!(!app.messages.is_empty());
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("MiniMax"));
    }

    #[test]
    fn test_handle_slash_command_provider_unknown() {
        let mut app = AppState::new();
        app.handle_slash_command(SlashCommand::Provider { name: "unknown".to_string() });
        assert!(!app.messages.is_empty());
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Unknown provider"));
    }

    #[test]
    fn test_handle_slash_command_help() {
        let mut app = AppState::new();
        app.handle_slash_command(SlashCommand::Help);
        assert!(!app.messages.is_empty());
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Available slash commands"));
    }

    #[test]
    fn test_handle_slash_command_clear() {
        let mut app = AppState::new();
        app.messages.push(ChatMessage::user("test"));
        app.handle_slash_command(SlashCommand::Clear);
        assert_eq!(app.messages.len(), 1);
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("cleared"));
    }

    #[test]
    fn test_handle_slash_command_save() {
        let mut app = AppState::new();
        app.handle_slash_command(SlashCommand::Save);
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("not yet implemented"));
    }

    #[test]
    fn test_handle_slash_command_status() {
        let mut app = AppState::new();
        app.handle_slash_command(SlashCommand::Status);
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Provider"));
        assert!(last_msg.content.contains("Model"));
    }

    #[test]
    fn test_handle_slash_command_unknown() {
        let mut app = AppState::new();
        app.handle_slash_command(SlashCommand::Unknown("foo".to_string()));
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Unknown command"));
        assert!(last_msg.content.contains("foo"));
    }

    #[test]
    fn test_send_message_triggers_slash_command_check() {
        let mut app = AppState::new();
        app.input_focused = true;
        app.input.insert_char('/');
        app.input.insert_char('h');
        app.input.insert_char('e');
        app.input.insert_char('l');
        app.input.insert_char('p');
        app.handle_input(input::InputAction::Submit);
        // Should not set is_streaming since it's a slash command
        assert!(!app.is_streaming);
        assert!(!app.messages.is_empty());
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Available slash commands"));
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.command.is_some() {
        run_cli(cli)
    } else {
        run_tui().await
    }
}

fn run_cli(cli: Cli) -> io::Result<()> {
    // Load config
    let mut config = Config::load().unwrap_or_default();

    // Override config with CLI flags
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

    // Override API key if provided
    if let Some(api_key) = &cli.global.api_key {
        match config.default_provider {
            ConfigProvider::StepFun => {
                if let Some(ref mut stepfun) = config.stepfun {
                    stepfun.api_key = api_key.clone();
                }
            }
            ConfigProvider::MiniMax => {
                if let Some(ref mut minimax) = config.minimax {
                    minimax.api_key = api_key.clone();
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

    // Create provider
    let provider = match create_provider(&config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create provider: {e}");
            return Ok(());
        }
    };

    match cli.command {
        Some(cli::Commands::Image { prompt, aspect_ratio, output, n }) => {
            match provider.image_generate(&prompt, n, &aspect_ratio) {
                Ok(resp) => {
                    if let Some(output_path) = output {
                        // Download and save images
                        for (i, url) in resp.urls.iter().enumerate() {
                            let path = if resp.urls.len() == 1 {
                                output_path.clone()
                            } else {
                                let ext = if output_path.contains('.') {
                                    output_path.rsplit_once('.').map(|(_, e)| e).unwrap_or("png")
                                } else {
                                    "png"
                                };
                                let stem = output_path.rsplit_once('/').map(|(_, s)| s).unwrap_or(&output_path);
                                let stem = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem);
                                format!("{stem}_{i}.{ext}")
                            };
                            if let Err(e) = download_file(url, &path) {
                                eprintln!("Failed to download {url}: {e}");
                            } else {
                                println!("Saved to {path}");
                            }
                        }
                    } else {
                        // Print URLs
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
            match provider.speech_synthesize(&text, &voice, speed, &format) {
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
            match provider.video_generate(&prompt, duration, &resolution) {
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
            match provider.music_generate(&prompt, lyrics.as_deref(), instrumental) {
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
            match provider.search(&query, count) {
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
            match provider.vision(&file, prompt.as_deref()) {
                Ok(resp) => {
                    println!("{}", resp.description);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            }
        }
        None => {
            // Should not happen, but run TUI just in case
            // This is handled at the call site
        }
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

    let mut app = AppState::new();
    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    while app.running {
        terminal.draw(|f| {
            let area = f.area();

            let theme = AppTheme::from_config(app.config.theme.as_ref());

            let AppLayout { sidebar, main, status } = compute_layout(area);

            Layout::render_sidebar(f, sidebar, app.current_view, &theme);

            match app.current_view {
                View::Chat => {
                    let chat_view = ChatView::new(&app.messages, &app.input.content, &theme)
                        .streaming(app.is_streaming)
                        .scroll_offset(app.scroll_offset);
                    chat_view.render(f, main);
                }
                View::Image => {
                    let image_view = ImageView::new(&app.input.content, &theme)
                        .generating(app.is_streaming);
                    image_view.render(f, main);
                }
                View::Audio => {
                    let audio_view = AudioView::new(&app.input.content, &app.status, &theme)
                        .generating(app.is_streaming);
                    audio_view.render(f, main);
                }
                View::Config => {
                    let config_view = ConfigView::new(&app.config, &theme);
                    config_view.render(f, main);
                }
            }

            Layout::render_status_bar(f, status, &app.status, &theme);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let action = input::handle_key_event(key);
            app.handle_input(action);
        }

        app.tick();

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}
