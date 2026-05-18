use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Current input mode of the TUI — used for flat (mode, action) dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Default mode: view switching, quit, focus input
    Normal,
    /// Text input is focused: typing, backspace, submit
    Typing,
    /// Waiting for an async response: only escape/quit work
    Streaming,
    /// Config view is active, field is being edited
    ConfigEditing,
    /// Config view is active, navigating between fields
    ConfigNavigating,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    // Navigation
    NextView,
    PrevView,
    SwitchView(usize),
    // Text input
    Char(char),
    Backspace,
    Delete,
    Home,
    End,
    Left,
    Right,
    // Control
    Submit,
    Escape,
    Quit,
    // Scroll
    ScrollUp,
    ScrollDown,
    // No-op
    None,
}

pub fn handle_key_event(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputAction::Quit
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputAction::Quit
        }
        KeyCode::Esc => InputAction::Escape,
        KeyCode::Enter => InputAction::Submit,
        KeyCode::Tab => InputAction::NextView,
        KeyCode::BackTab => InputAction::PrevView,
        KeyCode::Char('1') => InputAction::SwitchView(0),
        KeyCode::Char('2') => InputAction::SwitchView(1),
        KeyCode::Char('3') => InputAction::SwitchView(2),
        KeyCode::Char('4') => InputAction::SwitchView(3),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputAction::Char(c)
        }
        KeyCode::Backspace => InputAction::Backspace,
        KeyCode::Delete => InputAction::Delete,
        KeyCode::Home => InputAction::Home,
        KeyCode::End => InputAction::End,
        KeyCode::Left => InputAction::Left,
        KeyCode::Right => InputAction::Right,
        KeyCode::Up => InputAction::ScrollUp,
        KeyCode::Down => InputAction::ScrollDown,
        _ => InputAction::None,
    }
}

pub struct TextInputState {
    pub content: String,
    /// Byte offset of the cursor within `content`. Always on a UTF-8 char boundary.
    pub cursor_pos: usize,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
        }
    }

    #[allow(dead_code)]
    pub fn with_content(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            cursor_pos: content.len(),
            content,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let new_pos = self.prev_char_boundary(self.cursor_pos);
            self.content.replace_range(new_pos..self.cursor_pos, "");
            self.cursor_pos = new_pos;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor_pos < self.content.len() {
            let next_pos = self.next_char_boundary(self.cursor_pos);
            self.content.replace_range(self.cursor_pos..next_pos, "");
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.prev_char_boundary(self.cursor_pos);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.cursor_pos = self.next_char_boundary(self.cursor_pos);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.content.len();
    }

    pub fn submit(&mut self) -> String {
        let content = std::mem::take(&mut self.content);
        self.cursor_pos = 0;
        content
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_pos = 0;
    }

    fn prev_char_boundary(&self, pos: usize) -> usize {
        let mut p = pos.saturating_sub(1);
        while p > 0 && !self.content.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    fn next_char_boundary(&self, pos: usize) -> usize {
        let len = self.content.len();
        let mut p = (pos + 1).min(len);
        while p < len && !self.content.is_char_boundary(p) {
            p += 1;
        }
        p
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_new() {
        let input = TextInputState::new();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_with_content() {
        let input = TextInputState::with_content("hello");
        assert_eq!(input.content, "hello");
        assert_eq!(input.cursor_pos, 5);
    }

    #[test]
    fn test_text_input_insert_char() {
        let mut input = TextInputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        assert_eq!(input.content, "abc");
        assert_eq!(input.cursor_pos, 3);
    }

    #[test]
    fn test_text_input_insert_at_cursor() {
        let mut input = TextInputState::with_content("abc");
        input.move_home();
        input.insert_char('x');
        assert_eq!(input.content, "xabc");
        assert_eq!(input.cursor_pos, 1);
    }

    #[test]
    fn test_text_input_backspace() {
        let mut input = TextInputState::with_content("abc");
        input.backspace();
        assert_eq!(input.content, "ab");
        assert_eq!(input.cursor_pos, 2);
    }

    #[test]
    fn test_text_input_backspace_at_start() {
        let mut input = TextInputState::new();
        input.backspace();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_backspace_in_middle() {
        let mut input = TextInputState::with_content("abcd");
        input.move_home();
        input.move_right();
        input.move_right();
        input.backspace();
        assert_eq!(input.content, "acd");
        assert_eq!(input.cursor_pos, 1);
    }

    #[test]
    fn test_text_input_delete() {
        let mut input = TextInputState::with_content("abc");
        input.move_home();
        input.delete();
        assert_eq!(input.content, "bc");
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_delete_at_end() {
        let mut input = TextInputState::with_content("abc");
        input.move_end();
        input.delete();
        assert_eq!(input.content, "abc");
        assert_eq!(input.cursor_pos, 3);
    }

    #[test]
    fn test_text_input_move_left() {
        let mut input = TextInputState::with_content("abc");
        input.move_end();
        input.move_left();
        assert_eq!(input.cursor_pos, 2);
        input.move_left();
        assert_eq!(input.cursor_pos, 1);
        input.move_left();
        assert_eq!(input.cursor_pos, 0);
        input.move_left();
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_move_right() {
        let mut input = TextInputState::new();
        input.move_right();
        assert_eq!(input.cursor_pos, 0);
        input.insert_char('a');
        input.insert_char('b');
        input.move_home();
        input.move_right();
        assert_eq!(input.cursor_pos, 1);
    }

    #[test]
    fn test_text_input_move_home_end() {
        let mut input = TextInputState::with_content("hello");
        input.move_home();
        assert_eq!(input.cursor_pos, 0);
        input.move_end();
        assert_eq!(input.cursor_pos, 5);
    }

    #[test]
    fn test_text_input_submit() {
        let mut input = TextInputState::with_content("hello");
        let submitted = input.submit();
        assert_eq!(submitted, "hello");
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_clear() {
        let mut input = TextInputState::with_content("hello");
        input.clear();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_default() {
        let input = TextInputState::default();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_handle_key_event_quit() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(handle_key_event(key), InputAction::Quit);

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(handle_key_event(key), InputAction::Quit);
    }

    #[test]
    fn test_handle_key_event_navigation() {
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::NextView);

        let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::PrevView);

        let key = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::SwitchView(0));

        let key = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::SwitchView(1));
    }

    #[test]
    fn test_handle_key_event_text_input() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Char('a'));

        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Char('z'));
    }

    #[test]
    fn test_handle_key_event_control_keys() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Escape);

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Submit);

        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Backspace);

        let key = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Delete);
    }

    #[test]
    fn test_handle_key_event_scroll() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::ScrollUp);

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::ScrollDown);
    }

    #[test]
    fn test_handle_key_event_cursor_movement() {
        let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Home);

        let key = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::End);

        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Left);

        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Right);
    }

    #[test]
    fn test_handle_key_event_unknown() {
        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::None);

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), InputAction::Char('q'));
    }

    #[test]
    fn test_input_mode_variants() {
        // Just verify the enum exists and can be compared
        assert_ne!(InputMode::Normal, InputMode::Typing);
        assert_ne!(InputMode::Streaming, InputMode::ConfigEditing);
        assert_eq!(InputMode::ConfigNavigating, InputMode::ConfigNavigating);
    }

    #[test]
    fn test_text_input_multibyte_insert() {
        let mut input = TextInputState::new();
        input.insert_char('日');
        input.insert_char('本');
        input.insert_char('語');
        assert_eq!(input.content, "日本語");
        assert_eq!(input.cursor_pos, 9); // 3 chars × 3 bytes each
    }

    #[test]
    fn test_text_input_multibyte_backspace() {
        let mut input = TextInputState::with_content("日本");
        input.backspace();
        assert_eq!(input.content, "日");
        input.backspace();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_multibyte_delete() {
        let mut input = TextInputState::with_content("日本");
        input.move_home();
        input.delete();
        assert_eq!(input.content, "本");
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_text_input_multibyte_move() {
        let mut input = TextInputState::with_content("aé本z");
        input.move_home();
        input.move_right(); // past 'a'
        assert_eq!(input.cursor_pos, 1);
        input.move_right(); // past 'é' (2 bytes)
        assert_eq!(input.cursor_pos, 3);
        input.move_right(); // past '本' (3 bytes)
        assert_eq!(input.cursor_pos, 6);
        input.move_left();
        assert_eq!(input.cursor_pos, 3);
    }

    #[test]
    fn test_text_input_emoji_insert() {
        let mut input = TextInputState::new();
        input.insert_char('🚀');
        input.insert_char('a');
        assert_eq!(input.content, "🚀a");
        input.move_home();
        input.move_right(); // past emoji
        assert_eq!(input.cursor_pos, 4); // 🚀 is 4 bytes
    }
}
