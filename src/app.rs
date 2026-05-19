use crate::command::{self, SlashCommand};
use crate::config::{Config, ConfigEditor, Provider as ConfigProvider};
use crate::input::{self, InputMode, TextInputState};
use crate::providers::{create_provider, WorkResult};
use crate::ui::widget::{ChatMessage, MessageRole};
use crate::ui::View;

/// Returns a timestamped path under `~/.config/vox/images/`.
fn image_save_path() -> Option<std::path::PathBuf> {
    let dir = Config::config_dir()?.join("images");
    std::fs::create_dir_all(&dir).ok()?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    Some(dir.join(format!("{}.png", timestamp)))
}

pub struct AppState {
    pub running: bool,
    pub status: String,
    pub current_view: View,
    pub input_mode: InputMode,
    pub input: TextInputState,
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: u16,
    pub config: Config,
    pub config_editor: ConfigEditor,
    /// Last image generation result (URLs or error), shown in Image view preview
    pub image_result: Option<String>,
    /// Inline image protocol for ratatui-image rendering
    pub image_protocol: Option<ratatui_image::protocol::Protocol>,
    /// Image picker for creating protocols from decoded images
    pub picker: Option<ratatui_image::picker::Picker>,
    /// Last audio result, shown in Audio view preview
    pub audio_result: Option<String>,
    /// Which view initiated the current async operation (for routing errors/results)
    pending_view: Option<View>,
    work_rx: tokio::sync::mpsc::Receiver<WorkResult>,
    work_tx: tokio::sync::mpsc::Sender<WorkResult>,
}

impl AppState {
    pub fn new() -> Self {
        let (work_tx, work_rx) = tokio::sync::mpsc::channel(16);
        let config = Config::load().unwrap_or_default();

        Self {
            running: true,
            status: "vox — 1-4: switch view, Tab/⇧Tab: next/prev, Enter: send, q/Ctrl+C: quit".into(),
            current_view: View::default(),
            input_mode: InputMode::Normal,
            input: TextInputState::new(),
            messages: Vec::new(),
            scroll_offset: 0,
            config,
            config_editor: ConfigEditor::new(),
            image_result: None,
            image_protocol: None,
            picker: None,
            audio_result: None,
            pending_view: None,
            work_rx,
            work_tx,
        }
    }

    pub fn new_for_tui() -> Self {
        let mut state = Self::new();
        if state.config.stepfun.is_none() && state.config.minimax.is_none() {
            state.messages.push(ChatMessage::system(
                "Welcome to vox! No providers configured yet.\n\n\
                 Press 4 or Tab to switch to Config view and set up your API keys.\
                 Or edit ~/.config/vox/config.toml manually:\n\n\
                 [minimax]\n\
                 api_key = \"your-api-key\"\n\n\
                 Then use /provider minimax to activate.".to_string()
            ));
        }
        state
    }

    pub fn init_picker(&mut self) {
        self.picker = Self::create_picker();
    }

    fn create_picker() -> Option<ratatui_image::picker::Picker> {
        Some(ratatui_image::picker::Picker::halfblocks())
    }

    pub fn tick(&mut self) {
        while let Ok(result) = self.work_rx.try_recv() {
            match result {
                WorkResult::ChatResponse { content, model } => {
                    self.messages.push(ChatMessage::assistant(&content));
                    self.pending_view = None;
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
                    self.input_mode = InputMode::Normal;
                    self.status = format!("Error: {}", msg);
                    match self.pending_view {
                        Some(View::Image) => self.image_result = Some(format!("Error: {}", msg)),
                        Some(View::Audio) => self.audio_result = Some(format!("Error: {}", msg)),
                        _ => {
                            self.messages.push(ChatMessage::system(format!("Error: {}", msg)));
                        }
                    }
                    self.pending_view = None;
                }
                WorkResult::ImageGenerated { urls } => {
                    self.pending_view = None;
                    if let Some(url) = urls.first() {
                        let url_clone = url.clone();
                        let tx = self.work_tx.clone();
                        tokio::spawn(async move {
                            if let Ok(resp) = reqwest::get(&url_clone).await
                                && let Ok(bytes) = resp.bytes().await
                            {
                                // Save to file
                                if let Some(dir) = dirs::config_dir() {
                                    let dir = dir.join("vox").join("images");
                                    if std::fs::create_dir_all(&dir).is_ok() {
                                        let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                                        let path = dir.join(format!("{}.png", ts));
                                        if std::fs::write(&path, &bytes).is_ok() {
                                            let _ = open::that(&path);
                                        }
                                    }
                                }
                                // Forward to ImageDownloaded for TUI protocol creation
                                let _ = tx.send(WorkResult::ImageDownloaded { image_data: bytes.to_vec() }).await;
                            }
                        });
                        self.status = "Downloading image...".into();
                        self.image_result = Some(format!("Image URL: {}", url));
                    } else {
                        self.input_mode = InputMode::Normal;
                        self.status = "No images generated".into();
                        self.image_result = Some("No images were returned.".into());
                    }
                }
                WorkResult::ImageDownloaded { image_data } => {
                    self.pending_view = None;
                    if let Some(picker) = &self.picker
                        && let Ok(dyn_img) = image::load_from_memory(&image_data)
                    {
                        let font_size = picker.font_size();
                        let max_w = 60u16;
                        let max_h = 30u16;
                        let img_cell_w = (dyn_img.width() as f32 / font_size.0 as f32).ceil() as u16;
                        let img_cell_h = (dyn_img.height() as f32 / font_size.1 as f32).ceil() as u16;
                        let size = ratatui::layout::Rect::new(
                            0,
                            0,
                            img_cell_w.min(max_w),
                            img_cell_h.min(max_h),
                        );
                        if let Ok(proto) = picker.new_protocol(dyn_img, size, ratatui_image::Resize::Fit(None)) {
                            self.image_protocol = Some(proto);
                        }
                    }
                    self.input_mode = InputMode::Normal;
                    self.status = "Image generated — opened in viewer".into();
                    self.image_result = Some("🖼️ Image generated — opened in system viewer".into());
                }
                WorkResult::SpeechGenerated { audio_data, format: fmt } => {
                    self.pending_view = None;
                    self.input_mode = InputMode::Normal;
                    self.status = "Speech generated".into();
                    self.audio_result = Some(format!("Speech generated ({} bytes, {})", audio_data.len(), fmt));
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
        self.input.clear();
        self.sync_mode_for_view();
    }

    pub fn prev_view(&mut self) {
        let views = View::all();
        let idx = views.iter().position(|v| v == &self.current_view).unwrap_or(0);
        let new_idx = if idx == 0 { views.len() - 1 } else { idx - 1 };
        self.current_view = views[new_idx];
        self.input.clear();
        self.sync_mode_for_view();
    }

    pub fn switch_view(&mut self, idx: usize) {
        let views = View::all();
        if idx < views.len() {
            self.current_view = views[idx];
        }
        self.input.clear();
        self.sync_mode_for_view();
    }

    /// Auto-transition input mode when switching views
    fn sync_mode_for_view(&mut self) {
        match self.current_view {
            View::Config => {
                if self.input_mode == InputMode::Normal || self.input_mode == InputMode::Typing {
                    self.input_mode = InputMode::ConfigNavigating;
                }
            }
            _ => {
                if self.input_mode == InputMode::ConfigNavigating || self.input_mode == InputMode::ConfigEditing {
                    self.input_mode = InputMode::Normal;
                }
            }
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
                    input::InputAction::SwitchView(digit) if digit <= 9 => {
                        self.input.insert_char(char::from_digit(digit as u32, 10).unwrap());
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
                    input::InputAction::SwitchView(digit) if digit <= 9 => {
                        self.config_editor.type_char(char::from_digit(digit as u32, 10).unwrap());
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
                    input::InputAction::Char(c) => {
                        self.input.insert_char(c);
                        self.input_mode = InputMode::Typing;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn send_message(&mut self) {
        let content = self.input.submit();

        if let Some(cmd) = command::parse_slash_command(&content) {
            self.handle_slash_command(cmd);
            return;
        }

        if content.is_empty() {
            return;
        }

        match self.current_view {
            View::Chat => self.send_chat(content),
            View::Image => self.send_image(content),
            View::Audio => self.send_audio(content),
            View::Config => {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    fn send_chat(&mut self, content: String) {
        self.messages.push(ChatMessage::user(&content));
        self.input_mode = InputMode::Streaming;
        self.status = "Waiting for response...".into();
        self.scroll_offset = u16::MAX;

        let provider_messages: Vec<crate::providers::Message> = self.messages.iter().map(|m| {
            match m.role {
                MessageRole::User => crate::providers::Message::user(&m.content),
                MessageRole::Assistant => crate::providers::Message::assistant(&m.content),
                MessageRole::System => crate::providers::Message::system(&m.content),
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

    fn send_image(&mut self, prompt: String) {
        self.input_mode = InputMode::Streaming;
        self.status = "Generating image...".into();
        self.pending_view = Some(View::Image);

        let config = self.config.clone();
        let tx = self.work_tx.clone();

        tokio::spawn(async move {
            let result = match create_provider(&config) {
                Ok(provider) => match provider.image_generate(&prompt, 1, "1:1").await {
                    Ok(response) => {
                        if let Some(url) = response.urls.first() {
                            if let Ok(resp) = reqwest::get(url).await {
                                if let Ok(bytes) = resp.bytes().await {
                                    let path = image_save_path().unwrap_or_else(|| std::env::temp_dir().join("vox-last-image.png"));
                                    let _ = std::fs::write(&path, &bytes);
                                    let _ = open::that(&path);
                                    WorkResult::ImageDownloaded { image_data: bytes.to_vec() }
                                } else {
                                    WorkResult::ImageGenerated { urls: response.urls }
                                }
                            } else {
                                WorkResult::ImageGenerated { urls: response.urls }
                            }
                        } else {
                            WorkResult::ImageGenerated { urls: response.urls }
                        }
                    }
                    Err(e) => WorkResult::Error(e.to_string()),
                },
                Err(e) => WorkResult::Error(e.to_string()),
            };
            let _ = tx.send(result).await;
        });
    }

    fn send_audio(&mut self, text: String) {
        self.input_mode = InputMode::Streaming;
        self.status = "Generating speech...".into();
        self.pending_view = Some(View::Audio);

        let config = self.config.clone();
        let tx = self.work_tx.clone();

        tokio::spawn(async move {
            let result = match create_provider(&config) {
                Ok(provider) => match provider.speech_synthesize(&text, "default", 1.0, "mp3").await {
                    Ok(response) => WorkResult::SpeechGenerated {
                        audio_data: response.audio_data,
                        format: response.format,
                    },
                    Err(e) => WorkResult::Error(e.to_string()),
                },
                Err(e) => WorkResult::Error(e.to_string()),
            };
            let _ = tx.send(result).await;
        });
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
                    /save - Save conversation to markdown\n\
                    /status - Show current provider and model info";
                self.messages.push(ChatMessage::system(help_text));
            }
            SlashCommand::Clear => {
                self.messages.clear();
                self.messages.push(ChatMessage::system("Chat history cleared".to_string()));
            }
            SlashCommand::Save => {
                match self.save_conversation() {
                    Ok(path) => {
                        self.messages.push(ChatMessage::system(format!("Conversation saved to {path}")));
                    }
                    Err(e) => {
                        self.messages.push(ChatMessage::system(format!("Failed to save: {e}")));
                    }
                }
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
    }

    fn save_conversation(&self) -> Result<String, Box<dyn std::error::Error>> {
        let history_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("vox")
            .join("history");
        std::fs::create_dir_all(&history_dir)?;

        let now = chrono::Local::now();
        let filename = format!("{}.md", now.format("%Y-%m-%d_%H-%M"));
        let path = history_dir.join(&filename);

        let mut content = String::new();
        content.push_str(&format!("# Vox Conversation — {}\n\n", now.format("%Y-%m-%d %H:%M")));
        for msg in &self.messages {
            let role = msg.role_label();
            let ts = msg.timestamp.as_deref().unwrap_or("");
            content.push_str(&format!("## {role} ({ts})\n\n{body}\n\n", body = msg.content));
        }

        std::fs::write(&path, content)?;
        Ok(path.to_string_lossy().into_owned())
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
    }

    #[test]
    fn test_app_state_tick_streaming_mode() {
        let mut app = AppState::new();
        app.input_mode = InputMode::Streaming;
        app.tick();
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
    fn test_handle_slash_command_provider() {
        let mut app = AppState::new();
        app.config = Config {
            version: 1,
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
            output_dir: None,
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
        assert!(
            last_msg.content.contains("saved to") || last_msg.content.contains("Failed to save"),
            "Expected save confirmation or error, got: {}",
            last_msg.content
        );
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
        assert_ne!(app.input_mode, InputMode::Streaming);
        assert!(!app.messages.is_empty());
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Available slash commands"));
    }

    #[test]
    fn test_config_navigation_scroll_down() {
        let mut app = AppState::new();
        app.switch_view(3);
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
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        app.config_editor.selected = 2;
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.config_editor.selected, 1);
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.config_editor.selected, 0);
        app.handle_input(input::InputAction::ScrollUp);
        assert_eq!(app.config_editor.selected, 0);
    }

    #[test]
    fn test_config_edit_start() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
        assert!(!app.config_editor.editing);
        app.handle_input(input::InputAction::Submit);
        assert!(!app.config_editor.editing);
        assert_eq!(app.input_mode, InputMode::ConfigNavigating);

        app.config_editor.selected = 1;
        app.handle_input(input::InputAction::Submit);
        assert!(app.config_editor.editing);
        assert_eq!(app.input_mode, InputMode::ConfigEditing);
    }

    #[test]
    fn test_config_edit_cancel() {
        let mut app = AppState::new();
        app.switch_view(3);
        app.input_mode = InputMode::ConfigNavigating;
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
        app.config_editor.selected = 1;
        app.handle_input(input::InputAction::Submit);
        app.handle_input(input::InputAction::ScrollDown);
        assert_eq!(app.config_editor.selected, 1);
        assert!(app.config_editor.editing);
    }
}
