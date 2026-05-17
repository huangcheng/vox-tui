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
use ui::{AudioView, ChatView, ConfigView, ImageView, Layout, View, AppTheme, AppLayout, compute_layout, widget::{ChatMessage, MessageRole}};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::cli::Cli;
use crate::command::SlashCommand;
use crate::config::{Config, Provider as ConfigProvider, ConfigEditor};
use crate::input::InputMode;
use crate::provider::{WorkResult, create_provider};

pub struct AppState {
    pub running: bool,
    pub status: String,
    pub current_view: View,
    pub input_mode: InputMode,
    pub input: input::TextInputState,
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: u16,
    pub config: Config,
    pub config_editor: ConfigEditor,
    work_rx: tokio::sync::mpsc::Receiver<WorkResult>,
    work_tx: tokio::sync::mpsc::Sender<WorkResult>,
}

impl AppState {
    pub fn new() -> Self {
        let (work_tx, work_rx) = tokio::sync::mpsc::channel(16);
        let config = Config::load().unwrap_or_default();

        Self {
            running: true,
            status: "vox-tui — 1-4: switch view, Tab/⇧Tab: next/prev, Enter: send, q/Ctrl+C: quit".into(),
            current_view: View::default(),
            input_mode: InputMode::Normal,
            input: input::TextInputState::new(),
            messages: Vec::new(),
            scroll_offset: 0,
            config,
            config_editor: ConfigEditor::new(),
            work_rx,
            work_tx,
        }
    }

    pub fn new_for_tui() -> Self {
        let mut state = Self::new();
        if state.config.stepfun.is_none() && state.config.minimax.is_none() {
            state.messages.push(ChatMessage::system(
                "Welcome to vox! No providers configured yet.\n\n\
                 Press 4 or Tab to switch to Config view and set up your API keys.\n\
                 Or edit ~/.config/vox/config.toml manually:\n\n\
                 [minimax]\n\
                 api_key = \"your-api-key\"\n\n\
                 Then use /provider minimax to activate.".to_string()
            ));
        }
        state
    }

    pub fn tick(&mut self) {
        while let Ok(result) = self.work_rx.try_recv() {
            match result {
                WorkResult::ChatResponse { content, model } => {
                    self.messages.push(ChatMessage::assistant(&content));
                    self.input_mode = InputMode::Normal;
                    self.status = format!("Ready — {}", model);
                }
                WorkResult::StreamChunk(chunk) => {
                    if let Some(last) = self.messages.last_mut() {
                        last.content.push_str(&chunk);
                    }
                }
                WorkResult::StreamDone => {
                    self.input_mode = InputMode::Normal;
                    self.status = "Ready".into();
                }
                WorkResult::Error(msg) => {
                    self.messages.push(ChatMessage::system(format!("Error: {}", msg)));
                    self.input_mode = InputMode::Normal;
                    self.status = "Error — press any key".into();
                }
                WorkResult::ImageGenerated { urls } => {
                    self.messages.push(ChatMessage::assistant(
                        format!("Generated {} image(s):\n{}", urls.len(), urls.join("\n"))
                    ));
                    self.input_mode = InputMode::Normal;
                    self.status = "Image generated".into();
                }
                WorkResult::SpeechGenerated { .. } => {
                    self.messages.push(ChatMessage::assistant("Speech generated successfully."));
                    self.input_mode = InputMode::Normal;
                    self.status = "Speech generated".into();
                }
                WorkResult::VideoGenerated { task_id, status, video_url } => {
                    let msg = format!("Video task {} — status: {}", task_id, status);
                    self.messages.push(ChatMessage::assistant(
                        if let Some(url) = video_url { format!("{}\nURL: {}", msg, url) } else { msg }
                    ));
                    self.input_mode = InputMode::Normal;
                    self.status = "Video task submitted".into();
                }
                WorkResult::MusicGenerated { .. } => {
                    self.messages.push(ChatMessage::assistant("Music generated successfully."));
                    self.input_mode = InputMode::Normal;
                    self.status = "Music generated".into();
                }
                WorkResult::SearchResults { results } => {
                    let text = results.iter()
                        .map(|r| format!("{}\n  {}", r.title, r.url))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    self.messages.push(ChatMessage::assistant(text));
                    self.input_mode = InputMode::Normal;
                    self.status = "Search complete".into();
                }
                WorkResult::VisionResult { description } => {
                    self.messages.push(ChatMessage::assistant(&description));
                    self.input_mode = InputMode::Normal;
                    self.status = "Vision complete".into();
                }
            }
        }
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
        match self.input_mode {
            InputMode::Streaming => {
                match action {
                    input::InputAction::Escape => {
                        self.input_mode = InputMode::Normal;
                        self.status = "Streaming cancelled".into();
                    }
                    input::InputAction::Quit => self.running = false,
                    _ => {}
                }
            }
            InputMode::Typing => {
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
                        self.input_mode = InputMode::Normal;
                    }
                    input::InputAction::ScrollUp => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    }
                    input::InputAction::ScrollDown => {
                        self.scroll_offset = self.scroll_offset.saturating_add(1);
                    }
                    input::InputAction::NextView => {
                        self.input_mode = InputMode::Normal;
                        self.next_view();
                    }
                    _ => {}
                }
            }
            InputMode::ConfigEditing => {
                match action {
                    input::InputAction::Char(c) => self.config_editor.type_char(c),
                    input::InputAction::Backspace => self.config_editor.backspace(),
                    input::InputAction::Submit => {
                        if let Err(e) = self.config_editor.apply_edit(&mut self.config) {
                            self.messages.push(ChatMessage::system(e));
                        }
                        self.input_mode = InputMode::ConfigNavigating;
                    }
                    input::InputAction::Escape => {
                        self.config_editor.cancel_edit();
                        self.input_mode = InputMode::ConfigNavigating;
                    }
                    _ => {}
                }
            }
            InputMode::ConfigNavigating => {
                match action {
                    input::InputAction::Quit => self.running = false,
                    input::InputAction::NextView => self.next_view(),
                    input::InputAction::PrevView => self.prev_view(),
                    input::InputAction::SwitchView(idx) => self.switch_view(idx),
                    input::InputAction::ScrollUp => self.config_editor.navigate_up(&self.config),
                    input::InputAction::ScrollDown => self.config_editor.navigate_down(&self.config),
                    input::InputAction::Left => self.config_editor.cycle_field(&mut self.config, -1),
                    input::InputAction::Right => self.config_editor.cycle_field(&mut self.config, 1),
                    input::InputAction::Submit => {
                        self.config_editor.start_edit(&self.config);
                        if self.config_editor.editing {
                            self.input_mode = InputMode::ConfigEditing;
                        }
                    }
                    input::InputAction::Escape => {
                        self.input_mode = InputMode::Normal;
                        self.current_view = View::Chat;
                    }
                    _ => {}
                }
            }
            InputMode::Normal => {
                match action {
                    input::InputAction::Quit => self.running = false,
                    input::InputAction::NextView => self.next_view(),
                    input::InputAction::PrevView => self.prev_view(),
                    input::InputAction::SwitchView(idx) => self.switch_view(idx),
                    input::InputAction::Submit => {
                        if self.current_view == View::Config {
                            self.config_editor.start_edit(&self.config);
                            self.input_mode = InputMode::ConfigEditing;
                        } else {
                            self.input_mode = InputMode::Typing;
                        }
                    }
                    input::InputAction::Escape => {
                        self.input.clear();
                    }
                    input::InputAction::ScrollUp if self.current_view == View::Chat => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    }
                    input::InputAction::ScrollDown if self.current_view == View::Chat => {
                        self.scroll_offset = self.scroll_offset.saturating_add(1);
                    }
                    _ => {}
                }
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
        self.input_mode = InputMode::Streaming;
        self.status = "Waiting for response...".into();
        self.scroll_offset = u16::MAX;

        // Build provider messages from history
        let provider_messages: Vec<crate::provider::Message> = self.messages.iter().map(|m| {
            match m.role {
                MessageRole::User => crate::provider::Message::user(&m.content),
                MessageRole::Assistant => crate::provider::Message::assistant(&m.content),
                MessageRole::System => crate::provider::Message::system(&m.content),
            }
        }).collect();

        let config = self.config.clone();
        let tx = self.work_tx.clone();

        tokio::spawn(async move {
            let result = match create_provider(&config) {
                Ok(provider) => match provider.chat(&provider_messages).await {
                    Ok(response) => WorkResult::ChatResponse {
                        content: response.content,
                        model: response.model,
                    },
                    Err(e) => WorkResult::Error(e.to_string()),
                },
                Err(e) => WorkResult::Error(e.to_string()),
            };
            let _ = tx.send(result).await;
        });
    }

    pub fn append_stream(&mut self, text: &str) {
        if let Some(last) = self.messages.last_mut() {
            last.content.push_str(text);
        }
    }

    pub fn handle_slash_command(&mut self, cmd: SlashCommand) {
        self.input_mode = InputMode::Normal;

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
                if name.is_empty() {
                    let models = self.config.get_available_models("chat");
                    if models.is_empty() {
                        self.messages.push(ChatMessage::system(
                            "No model list available. Use /model <name> to set a specific model.".to_string()
                        ));
                    } else {
                        let current = self.config.get_model_for("chat").unwrap_or_default();
                        let model_list: String = models.iter()
                            .map(|m| if m == &current { format!("► {}", m) } else { format!("  {}", m) })
                            .collect::<Vec<_>>()
                            .join("\n");
                        self.messages.push(ChatMessage::system(
                            format!("Available models:\n{}\n\nUse /model <name> to switch.", model_list)
                        ));
                    }
                } else {
                    match self.config.default_provider {
                        ConfigProvider::StepFun => {
                            if let Some(ref mut stepfun) = self.config.stepfun {
                                stepfun.model = Some(name.clone());
                            }
                        }
                        ConfigProvider::MiniMax => {
                            if let Some(ref mut minimax) = self.config.minimax {
                                minimax.model = Some(name.clone());
                            }
                        }
                    }
                    self.messages.push(ChatMessage::system(format!("Model set to: {}", name)));
                }
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
        // Don't set input_mode to Streaming for slash commands
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
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.messages.is_empty());
        assert_ne!(app.input_mode, InputMode::Streaming);
        assert_ne!(app.input_mode, InputMode::Typing);
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
        assert_ne!(app.input_mode, InputMode::Typing);
        app.handle_input(input::InputAction::Submit);
        assert_eq!(app.input_mode, InputMode::Typing);
    }

    #[test]
    fn test_app_state_handle_input_escape() {
        let mut app = AppState::new();
        app.input.insert_char('h');
        app.input_mode = InputMode::Typing;
        app.handle_input(input::InputAction::Escape);
        assert_ne!(app.input_mode, InputMode::Typing);
        assert!(app.input.content.is_empty());
    }

    #[test]
    fn test_app_state_handle_input_typing() {
        let mut app = AppState::new();
        app.input_mode = InputMode::Typing;
        app.handle_input(input::InputAction::Char('h'));
        app.handle_input(input::InputAction::Char('i'));
        assert_eq!(app.input.content, "hi");
    }

    #[tokio::test]
    async fn test_app_state_handle_input_submit_message() {
        let mut app = AppState::new();
        app.input_mode = InputMode::Typing;
        app.handle_input(input::InputAction::Char('h'));
        app.handle_input(input::InputAction::Char('i'));
        app.handle_input(input::InputAction::Submit);
        assert_ne!(app.input_mode, InputMode::Typing);
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "hi");
        assert_eq!(app.input_mode, InputMode::Streaming);
    }

    #[test]
    fn test_app_state_tick() {
        let mut app = AppState::new();
        app.tick();
        // No assertions needed - just verify it doesn't panic
    }

    #[test]
    fn test_app_state_tick_streaming_mode() {
        let mut app = AppState::new();
        app.input_mode = InputMode::Streaming;
        app.tick();
        // Without receiving a WorkResult, mode stays Streaming
        assert_eq!(app.input_mode, InputMode::Streaming);
    }

    #[test]
    fn test_app_state_scroll() {
        let mut app = AppState::new();
        assert_eq!(app.scroll_offset, 0);
        app.input_mode = InputMode::Typing;
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
                models: crate::config::ProviderModels::default(),
            }),
            minimax: Some(crate::config::MiniMaxConfig {
                api_key: "test".to_string(),
                group_id: None,
                base_url: None,
                model: None,
                models: crate::config::ProviderModels::default(),
            }),
            theme: None,
        };

        app.handle_slash_command(SlashCommand::Provider { name: "minimax".to_string() });
        assert_eq!(app.config.default_provider, ConfigProvider::MiniMax);
        assert_ne!(app.input_mode, InputMode::Streaming);
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
        app.input_mode = InputMode::Typing;
        app.input.insert_char('/');
        app.input.insert_char('h');
        app.input.insert_char('e');
        app.input.insert_char('l');
        app.input.insert_char('p');
        app.handle_input(input::InputAction::Submit);
        // Should not set Streaming since it's a slash command
        assert_ne!(app.input_mode, InputMode::Streaming);
        assert!(!app.messages.is_empty());
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Available slash commands"));
    }

    // Config navigation and editing tests

    #[test]
    fn test_config_navigation_scroll_down() {
        let mut app = AppState::new();
        app.switch_view(3); // Config view
        app.input_mode = InputMode::ConfigNavigating;
        assert_eq!(app.config_editor.selected, 0);
        app.handle_input(input::InputAction::ScrollDown);
        assert_eq!(app.config_editor.selected, 1);
        app.handle_input(input::InputAction::ScrollDown);
        assert_eq!(app.config_editor.selected, 2);
    }

    #[test]
    fn test_config_navigation_scroll_up() {
        let mut app = AppState::new();
        app.switch_view(3); // Config view
        app.input_mode = InputMode::ConfigNavigating;
        app.config_editor.selected = 2;
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.config_editor.selected, 1);
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.config_editor.selected, 0);
        // Should not go below 0
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.config_editor.selected, 0);
    }

    #[test]
    fn test_config_edit_start() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        // ActiveProvider (index 0) is a selector, not editable
        assert!(!app.config_editor.editing);
        app.handle_input(input::InputAction::Submit);
        assert!(!app.config_editor.editing);
        assert_eq!(app.input_mode, InputMode::ConfigNavigating);

        // Navigate to an editable field (API key) and try again
        app.config_editor.selected = 1; // StepFunApiKey or MiniMaxApiKey
        app.handle_input(input::InputAction::Submit);
        assert!(app.config_editor.editing);
        assert_eq!(app.input_mode, InputMode::ConfigEditing);
    }

    #[test]
    fn test_config_edit_cancel() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        // Navigate to an editable field (index 1)
        app.config_editor.selected = 1;
        app.handle_input(input::InputAction::Submit);
        app.config_editor.edit_buffer.push('x');
        app.handle_input(input::InputAction::Escape);
        assert!(!app.config_editor.editing);
        assert!(app.config_editor.edit_buffer.is_empty());
    }

    #[test]
    fn test_config_edit_type_char() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        // Navigate to an editable field (index 1)
        app.config_editor.selected = 1;
        app.handle_input(input::InputAction::Submit);
        let initial_len = app.config_editor.edit_buffer.len();
        app.handle_input(input::InputAction::Char('n'));
        app.handle_input(input::InputAction::Char('e'));
        app.handle_input(input::InputAction::Char('w'));
        assert_eq!(app.config_editor.edit_buffer.len(), initial_len + 3);
        assert!(app.config_editor.edit_buffer.ends_with("new"));
    }

    #[test]
    fn test_config_edit_backspace() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        // Navigate to an editable field (index 1)
        app.config_editor.selected = 1;
        app.handle_input(input::InputAction::Submit);
        app.config_editor.edit_buffer.push('x');
        app.config_editor.edit_buffer.push('y');
        let len_before = app.config_editor.edit_buffer.len();
        app.handle_input(input::InputAction::Backspace);
        assert_eq!(app.config_editor.edit_buffer.len(), len_before - 1);
    }

    #[test]
    fn test_config_navigation_blocked_while_editing() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        // Navigate to an editable field (index 1)
        app.config_editor.selected = 1;
        app.handle_input(input::InputAction::Submit);
        app.handle_input(input::InputAction::ScrollDown);
        assert_eq!(app.config_editor.selected, 1);
        assert!(app.config_editor.editing);
    }
}

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
            match provider.image_generate(&prompt, n, &aspect_ratio).await {
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

    let mut app = AppState::new_for_tui();
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
                        .streaming(app.input_mode == InputMode::Streaming)
                        .scroll_offset(app.scroll_offset);
                    chat_view.render(f, main);
                }
                View::Image => {
                    let image_view = ImageView::new(&app.input.content, &theme)
                        .generating(app.input_mode == InputMode::Streaming);
                    image_view.render(f, main);
                }
                View::Audio => {
                    let audio_view = AudioView::new(&app.input.content, &app.status, &theme)
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

            let provider_name = match app.config.default_provider {
                ConfigProvider::StepFun => "StepFun",
                ConfigProvider::MiniMax => "MiniMax",
            };
            let model_name = match app.config.default_provider {
                ConfigProvider::StepFun => app.config.stepfun.as_ref().and_then(|s| s.model.as_deref()).unwrap_or("default"),
                ConfigProvider::MiniMax => app.config.minimax.as_ref().and_then(|m| m.model.as_deref()).unwrap_or("default"),
            };
            let mode_label = provider_name;
            let position_label = format!("vox | {}", model_name);
            let help_label = "Tab: switch view  Enter: send  q: quit";

            Layout::render_status_bar(f, status, mode_label, &position_label, help_label, &theme);
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
