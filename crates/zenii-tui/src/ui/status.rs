use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, ChatStatus, ConnectionStatus};

pub fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let conn_indicator = match app.connection_status {
        ConnectionStatus::Connected => Span::styled("\u{25CF} Connected", app.theme.connection_ok),
        ConnectionStatus::Disconnected => {
            Span::styled("\u{25CF} Disconnected", app.theme.connection_err)
        }
        ConnectionStatus::Reconnecting => {
            Span::styled("\u{25CF} Reconnecting...", app.theme.tool_call)
        }
    };

    let model = Span::styled(
        format!(" \u{2502} {} ", app.current_model),
        app.theme.status_bar,
    );

    let status_text = match &app.chat_status {
        ChatStatus::Streaming => " \u{2502} Streaming... ".to_string(),
        ChatStatus::Error(e) => format!(" \u{2502} Error: {} ", truncate(e, 30)),
        _ => String::new(),
    };
    let status_span = if !status_text.is_empty() {
        Span::styled(status_text, app.theme.status_bar)
    } else {
        Span::raw("")
    };

    let help_hint = Span::styled(" Tab=switch  ?=help  q=quit ", app.theme.status_bar);

    let notification = if let Some(ref text) = app.notification_text {
        if !text.starts_with("__") {
            Span::styled(format!(" {text} "), app.theme.tool_call)
        } else {
            Span::raw("")
        }
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        conn_indicator,
        model,
        status_span,
        notification,
        help_hint,
    ]);

    let paragraph = Paragraph::new(line).style(app.theme.status_bar);
    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max.saturating_sub(1)).collect();
        result.push('\u{2026}');
        result
    }
}
