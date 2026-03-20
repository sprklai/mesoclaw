use crate::theme::Theme;

/// Focus mode / which pane is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    SessionList,
    Chat,
    Input,
    Help,
    Onboard,
    PluginList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardStep {
    ProviderSelect,
    ApiKey,
    ModelSelect,
    Channels,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardField {
    Name,
    Location,
    Timezone,
}

#[derive(Debug, Clone)]
pub struct ChannelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub credentials: &'static [(&'static str, &'static str, bool)], // (key, label, is_secret)
}

pub const ONBOARD_CHANNELS: &[ChannelDef] = &[
    ChannelDef {
        id: "telegram",
        name: "Telegram",
        credentials: &[
            ("token", "Bot Token", true),
            ("allowed_chat_ids", "Allowed Chat IDs", false),
        ],
    },
    ChannelDef {
        id: "slack",
        name: "Slack",
        credentials: &[
            ("bot_token", "Bot Token", true),
            ("app_token", "App Token", true),
            ("allowed_channel_ids", "Allowed Channel IDs", false),
        ],
    },
    ChannelDef {
        id: "discord",
        name: "Discord",
        credentials: &[
            ("token", "Bot Token", true),
            ("allowed_guild_ids", "Allowed Server IDs", false),
            ("allowed_channel_ids", "Allowed Channel IDs", false),
        ],
    },
];

/// Chat streaming status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatStatus {
    Idle,
    Composing,
    Streaming,
    Error(String),
}

/// Connection to gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
}

/// Minimal session info for the sidebar.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub message_count: i64,
}

/// A chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub tool_calls: Vec<ToolEvent>,
}

/// Tool call event for display.
#[derive(Debug, Clone)]
pub struct ToolEvent {
    pub tool_name: String,
    pub args: String,
    pub output: Option<String>,
    pub success: Option<bool>,
    pub duration_ms: Option<u64>,
}

/// Multi-line text input state.
#[derive(Debug, Clone)]
pub struct TextInput {
    pub content: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
        }
    }

    pub fn insert(&mut self, ch: char) {
        let byte_pos = self
            .content
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.content.len());
        self.content.insert(byte_pos, ch);
        self.cursor_pos += 1;
    }

    pub fn delete_back(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            let byte_pos = self
                .content
                .char_indices()
                .nth(self.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(self.content.len());
            self.content.remove(byte_pos);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let char_count = self.content.chars().count();
        if self.cursor_pos < char_count {
            self.cursor_pos += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.content.chars().count();
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_pos = 0;
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let chars: Vec<char> = self.content.chars().collect();
        let mut new_pos = self.cursor_pos;
        // Skip whitespace
        while new_pos > 0 && chars[new_pos - 1].is_whitespace() {
            new_pos -= 1;
        }
        // Skip word chars
        while new_pos > 0 && !chars[new_pos - 1].is_whitespace() {
            new_pos -= 1;
        }
        let start_byte = chars[..new_pos].iter().map(|c| c.len_utf8()).sum::<usize>();
        let end_byte = chars[..self.cursor_pos]
            .iter()
            .map(|c| c.len_utf8())
            .sum::<usize>();
        self.content.replace_range(start_byte..end_byte, "");
        self.cursor_pos = new_pos;
    }

    pub fn take_content(&mut self) -> String {
        let text = self.content.clone();
        if !text.is_empty() {
            self.history.push(text.clone());
        }
        self.clear();
        self.history_index = None;
        text
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            None => self.history.len() - 1,
            Some(0) => return,
            Some(i) => i - 1,
        };
        self.history_index = Some(idx);
        self.content = self.history[idx].clone();
        self.cursor_pos = self.content.chars().count();
    }

    pub fn history_down(&mut self) {
        let idx = match self.history_index {
            None => return,
            Some(i) => i + 1,
        };
        if idx >= self.history.len() {
            self.history_index = None;
            self.clear();
        } else {
            self.history_index = Some(idx);
            self.content = self.history[idx].clone();
            self.cursor_pos = self.content.chars().count();
        }
    }
}

/// Plugin info for display in TUI.
#[derive(Debug, Clone)]
pub struct PluginListItem {
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub tools_count: usize,
    pub skills_count: usize,
}

/// Display status for a single sub-agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentDisplayStatus {
    Pending,
    Running,
    Completed { duration_ms: u64 },
    Failed { error: String },
}

/// Per-agent display state.
#[derive(Debug, Clone)]
pub struct AgentState {
    pub id: String,
    pub description: String,
    pub tool_uses: u32,
    pub tokens_used: u64,
    pub current_activity: String,
    pub status: AgentDisplayStatus,
}

/// Active delegation display state.
#[derive(Debug, Clone)]
pub struct DelegationState {
    pub delegation_id: String,
    pub agents: Vec<AgentState>,
    pub start_time: std::time::Instant,
}

/// Top-level application state.
pub struct App {
    pub mode: AppMode,
    pub previous_mode: AppMode,
    pub sessions: Vec<SessionSummary>,
    pub selected_session: Option<usize>,
    pub messages: Vec<ChatMessage>,
    pub input: TextInput,
    pub scroll_offset: usize,
    pub chat_status: ChatStatus,
    pub connection_status: ConnectionStatus,
    pub current_model: String,
    pub show_help: bool,
    pub should_quit: bool,
    pub streaming_buffer: String,
    pub tool_events: Vec<ToolEvent>,
    pub notification_text: Option<String>,
    pub theme: Theme,
    pub current_session_id: Option<String>,
    pub confirm_delete: bool,
    pub onboard_step: OnboardStep,
    pub onboard_providers: Vec<serde_json::Value>,
    pub onboard_selected_provider: usize,
    pub onboard_models: Vec<serde_json::Value>,
    pub onboard_selected_model: usize,
    pub onboard_api_key: TextInput,
    pub onboard_name: TextInput,
    pub onboard_location: TextInput,
    pub onboard_timezone: TextInput,
    pub onboard_error: Option<String>,
    pub onboard_saving: bool,
    pub onboard_field: OnboardField,
    pub onboard_provider_id: String,
    pub onboard_requires_key: bool,
    pub onboard_selected_channel: usize,
    pub onboard_channel_cred_idx: usize,
    pub onboard_channel_input: TextInput,
    pub onboard_channel_saved: std::collections::HashSet<String>,
    pub plugins: Vec<PluginListItem>,
    pub selected_plugin: Option<usize>,
    pub plugin_install_input: TextInput,
    pub plugin_install_local: bool,
    pub plugin_error: Option<String>,
    pub plugin_loading: bool,
    pub delegation: Option<DelegationState>,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::SessionList,
            previous_mode: AppMode::SessionList,
            sessions: Vec::new(),
            selected_session: None,
            messages: Vec::new(),
            input: TextInput::new(),
            scroll_offset: 0,
            chat_status: ChatStatus::Idle,
            connection_status: ConnectionStatus::Disconnected,
            current_model: "unknown".to_string(),
            show_help: false,
            should_quit: false,
            streaming_buffer: String::new(),
            tool_events: Vec::new(),
            notification_text: None,
            theme: Theme::default(),
            current_session_id: None,
            confirm_delete: false,
            onboard_step: OnboardStep::ProviderSelect,
            onboard_providers: Vec::new(),
            onboard_selected_provider: 0,
            onboard_models: Vec::new(),
            onboard_selected_model: 0,
            onboard_api_key: TextInput::new(),
            onboard_name: TextInput::new(),
            onboard_location: TextInput::new(),
            onboard_timezone: TextInput::new(),
            onboard_error: None,
            onboard_saving: false,
            onboard_field: OnboardField::Name,
            onboard_provider_id: String::new(),
            onboard_requires_key: true,
            onboard_selected_channel: 0,
            onboard_channel_cred_idx: 0,
            onboard_channel_input: TextInput::new(),
            onboard_channel_saved: std::collections::HashSet::new(),
            plugins: Vec::new(),
            selected_plugin: None,
            plugin_install_input: TextInput::new(),
            plugin_install_local: false,
            plugin_error: None,
            plugin_loading: false,
            delegation: None,
        }
    }

    pub fn select_next_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected_session = Some(match self.selected_session {
            None => 0,
            Some(i) => (i + 1).min(self.sessions.len() - 1),
        });
    }

    pub fn select_prev_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected_session = Some(match self.selected_session {
            None => 0,
            Some(0) => 0,
            Some(i) => i - 1,
        });
    }

    pub fn select_first_session(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_session = Some(0);
        }
    }

    pub fn select_last_session(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_session = Some(self.sessions.len() - 1);
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        // Will be clamped during rendering
        self.scroll_offset = usize::MAX;
    }

    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    pub fn scroll_page_down(&mut self, page_size: usize) {
        self.scroll_offset += page_size;
    }

    pub fn toggle_help(&mut self) {
        if self.show_help {
            self.show_help = false;
            self.mode = self.previous_mode;
        } else {
            self.previous_mode = self.mode;
            self.show_help = true;
            self.mode = AppMode::Help;
        }
    }

    pub fn enter_chat_mode(&mut self) {
        self.mode = AppMode::Chat;
        self.scroll_offset = usize::MAX; // scroll to bottom
    }

    pub fn enter_input_mode(&mut self) {
        self.mode = AppMode::Input;
        self.chat_status = ChatStatus::Composing;
    }

    pub fn exit_input_mode(&mut self) {
        self.mode = AppMode::Chat;
        if self.chat_status == ChatStatus::Composing {
            self.chat_status = ChatStatus::Idle;
        }
    }

    pub fn enter_session_list_mode(&mut self) {
        self.mode = AppMode::SessionList;
    }

    pub fn enter_plugin_list(&mut self) {
        self.mode = AppMode::PluginList;
    }

    pub fn select_next_plugin(&mut self) {
        if self.plugins.is_empty() {
            return;
        }
        self.selected_plugin = Some(match self.selected_plugin {
            None => 0,
            Some(i) => (i + 1).min(self.plugins.len() - 1),
        });
    }

    pub fn select_prev_plugin(&mut self) {
        if self.plugins.is_empty() {
            return;
        }
        self.selected_plugin = Some(match self.selected_plugin {
            None => 0,
            Some(0) => 0,
            Some(i) => i - 1,
        });
    }

    /// Append streaming text.
    pub fn append_streaming_text(&mut self, content: &str) {
        self.streaming_buffer.push_str(content);
    }

    /// Flush streaming buffer into a new assistant message.
    pub fn flush_streaming_buffer(&mut self) {
        if !self.streaming_buffer.is_empty() {
            self.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: std::mem::take(&mut self.streaming_buffer),
                timestamp: String::new(),
                tool_calls: std::mem::take(&mut self.tool_events),
            });
        }
        self.chat_status = ChatStatus::Idle;
        self.scroll_offset = usize::MAX;
    }

    pub fn start_delegation(&mut self, delegation_id: String, agents: Vec<(String, String)>) {
        self.delegation = Some(DelegationState {
            delegation_id,
            agents: agents
                .into_iter()
                .map(|(id, desc)| AgentState {
                    id,
                    description: desc,
                    tool_uses: 0,
                    tokens_used: 0,
                    current_activity: String::new(),
                    status: AgentDisplayStatus::Pending,
                })
                .collect(),
            start_time: std::time::Instant::now(),
        });
    }

    pub fn update_agent(&mut self, agent_id: &str, tool_uses: u32, tokens: u64, activity: &str) {
        if let Some(ref mut d) = self.delegation
            && let Some(agent) = d.agents.iter_mut().find(|a| a.id == agent_id)
        {
            agent.tool_uses = tool_uses;
            agent.tokens_used = tokens;
            agent.current_activity = activity.to_string();
            agent.status = AgentDisplayStatus::Running;
        }
    }

    pub fn complete_agent(
        &mut self,
        agent_id: &str,
        status: &str,
        duration_ms: u64,
        tool_uses: u32,
        tokens: u64,
    ) {
        if let Some(ref mut d) = self.delegation
            && let Some(agent) = d.agents.iter_mut().find(|a| a.id == agent_id)
        {
            agent.tool_uses = tool_uses;
            agent.tokens_used = tokens;
            agent.status = if status == "failed" {
                AgentDisplayStatus::Failed {
                    error: "task failed".into(),
                }
            } else {
                AgentDisplayStatus::Completed { duration_ms }
            };
        }
    }

    /// Clamp scroll offset to valid range given total rendered lines.
    pub fn clamp_scroll(&mut self, total_lines: usize, visible_lines: usize) {
        if total_lines <= visible_lines {
            self.scroll_offset = 0;
        } else {
            let max = total_lines - visible_lines;
            if self.scroll_offset > max {
                self.scroll_offset = max;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_initializes_in_session_list_mode() {
        let app = App::new();
        assert_eq!(app.mode, AppMode::SessionList);
        assert!(!app.should_quit);
        assert_eq!(app.chat_status, ChatStatus::Idle);
        assert_eq!(app.connection_status, ConnectionStatus::Disconnected);
    }

    #[test]
    fn transition_session_list_to_chat() {
        let mut app = App::new();
        app.sessions.push(SessionSummary {
            id: "s1".into(),
            title: "Test".into(),
            updated_at: "2026-01-01".into(),
            message_count: 0,
        });
        app.selected_session = Some(0);
        app.enter_chat_mode();
        assert_eq!(app.mode, AppMode::Chat);
    }

    #[test]
    fn transition_chat_to_input() {
        let mut app = App::new();
        app.mode = AppMode::Chat;
        app.enter_input_mode();
        assert_eq!(app.mode, AppMode::Input);
        assert_eq!(app.chat_status, ChatStatus::Composing);
    }

    #[test]
    fn transition_input_to_chat() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        app.chat_status = ChatStatus::Composing;
        app.exit_input_mode();
        assert_eq!(app.mode, AppMode::Chat);
        assert_eq!(app.chat_status, ChatStatus::Idle);
    }

    #[test]
    fn transition_chat_to_session_list() {
        let mut app = App::new();
        app.mode = AppMode::Chat;
        app.enter_session_list_mode();
        assert_eq!(app.mode, AppMode::SessionList);
    }

    #[test]
    fn toggle_help_overlay() {
        let mut app = App::new();
        assert!(!app.show_help);
        app.toggle_help();
        assert!(app.show_help);
        assert_eq!(app.mode, AppMode::Help);
        app.toggle_help();
        assert!(!app.show_help);
        assert_eq!(app.mode, AppMode::SessionList);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::new();
        app.should_quit = true;
        assert!(app.should_quit);
    }

    #[test]
    fn session_selection_wraps() {
        let mut app = App::new();
        for i in 0..3 {
            app.sessions.push(SessionSummary {
                id: format!("s{i}"),
                title: format!("Session {i}"),
                updated_at: String::new(),
                message_count: 0,
            });
        }
        app.selected_session = Some(2);
        app.select_next_session();
        assert_eq!(app.selected_session, Some(2)); // clamped at max

        app.selected_session = Some(0);
        app.select_prev_session();
        assert_eq!(app.selected_session, Some(0)); // clamped at min
    }

    #[test]
    fn scroll_offset_clamps() {
        let mut app = App::new();
        app.scroll_offset = 100;
        app.clamp_scroll(10, 20);
        assert_eq!(app.scroll_offset, 0); // total < visible

        app.scroll_offset = 100;
        app.clamp_scroll(30, 10);
        assert_eq!(app.scroll_offset, 20); // clamped to max
    }

    #[test]
    fn streaming_buffer_appends() {
        let mut app = App::new();
        app.append_streaming_text("hello ");
        app.append_streaming_text("world");
        assert_eq!(app.streaming_buffer, "hello world");
    }

    #[test]
    fn done_flushes_buffer() {
        let mut app = App::new();
        app.chat_status = ChatStatus::Streaming;
        app.append_streaming_text("response text");
        app.flush_streaming_buffer();
        assert!(app.streaming_buffer.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "response text");
        assert_eq!(app.chat_status, ChatStatus::Idle);
    }

    #[test]
    fn error_sets_status() {
        let mut app = App::new();
        app.chat_status = ChatStatus::Error("oops".into());
        assert_eq!(app.chat_status, ChatStatus::Error("oops".into()));
    }

    #[test]
    fn onboard_mode_initial_state() {
        let app = App::new();
        // Default mode is SessionList, not Onboard
        assert_eq!(app.mode, AppMode::SessionList);
        assert_eq!(app.onboard_step, OnboardStep::ProviderSelect);
        assert!(app.onboard_providers.is_empty());
    }
}
