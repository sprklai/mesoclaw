mod chat;
mod help;
mod input;
mod onboard;
mod sessions;
mod status;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::App;

/// Render the full TUI layout.
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Top-level split: sidebar (20%) | main (80%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(size);

    // Sidebar
    sessions::render_sessions(frame, horizontal[0], app);

    // Main area: chat (flex) | input (3 lines) | status (1 line)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(horizontal[1]);

    chat::render_chat(frame, vertical[0], app);
    input::render_input(frame, vertical[1], app);
    status::render_status(frame, vertical[2], app);

    // Onboard overlay
    if app.mode == crate::app::AppMode::Onboard {
        onboard::render_onboard(frame, size, app);
        return;
    }

    // Help overlay on top
    if app.show_help {
        let overlay = centered_rect(60, 70, size);
        help::render_help(frame, overlay);
    }
}

/// Create a centered rect using percentages of the parent area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::app::{App, AppMode, ChatMessage, ChatStatus, ConnectionStatus, SessionSummary};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn layout_splits_correctly() {
        let mut terminal = test_terminal();
        let app = App::new();
        terminal.draw(|f| render(f, &app)).unwrap();
        // If we get here without panic, layout is correct
    }

    #[test]
    fn session_list_renders_titles() {
        let mut terminal = test_terminal();
        let mut app = App::new();
        app.sessions.push(SessionSummary {
            id: "s1".into(),
            title: "My Session".into(),
            updated_at: "2026-01-01".into(),
            message_count: 5,
        });
        app.selected_session = Some(0);
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("My Session"),
            "Session title should appear in buffer"
        );
    }

    #[test]
    fn chat_renders_messages() {
        let mut terminal = test_terminal();
        let mut app = App::new();
        app.mode = AppMode::Chat;
        app.messages.push(ChatMessage {
            role: "user".into(),
            content: "Hello there".into(),
            timestamp: String::new(),
            tool_calls: vec![],
        });
        app.messages.push(ChatMessage {
            role: "assistant".into(),
            content: "Hi back".into(),
            timestamp: String::new(),
            tool_calls: vec![],
        });
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("Hello there"),
            "User message should render"
        );
        assert!(
            content.contains("Hi back"),
            "Assistant message should render"
        );
    }

    #[test]
    fn status_bar_connection() {
        let mut terminal = test_terminal();
        let mut app = App::new();
        app.connection_status = ConnectionStatus::Connected;
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("Connected"),
            "Status should show Connected"
        );
    }

    #[test]
    fn help_overlay_renders() {
        let mut terminal = test_terminal();
        let mut app = App::new();
        app.toggle_help();
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("Keybindings"),
            "Help overlay should show keybinding header"
        );
    }

    #[test]
    fn empty_sessions_placeholder() {
        let mut terminal = test_terminal();
        let app = App::new();
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("No sessions"),
            "Empty state should show placeholder"
        );
    }

    #[test]
    fn streaming_cursor_visible() {
        let mut terminal = test_terminal();
        let mut app = App::new();
        app.mode = AppMode::Chat;
        app.chat_status = ChatStatus::Streaming;
        app.streaming_buffer = "partial response".into();
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("partial response"),
            "Streaming buffer should render"
        );
    }
}
