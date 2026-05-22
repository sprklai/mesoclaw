use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::{App, AppMode};

pub fn render_sessions(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = matches!(app.mode, AppMode::SessionList);
    let border_style = if is_active {
        app.theme.border_active
    } else {
        app.theme.border_inactive
    };

    let title = format!(" Sessions ({}) ", app.sessions.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.sessions.is_empty() {
        let items = vec![ListItem::new(Line::from(Span::styled(
            "No sessions. Press 'n' to create.",
            Style::default().add_modifier(Modifier::DIM),
        )))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .map(|s| {
            let title_line = Line::from(Span::raw(truncate(&s.title, area.width.saturating_sub(4) as usize)));
            let meta = Line::from(Span::styled(
                format!("  {} msgs", s.message_count),
                Style::default().add_modifier(Modifier::DIM),
            ));
            ListItem::new(vec![title_line, meta])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(app.theme.selected.add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(app.selected_session);
    frame.render_stateful_widget(list, area, &mut state);
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
