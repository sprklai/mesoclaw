use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::app::{AgentDisplayStatus, App, AppMode, ChatStatus, DelegationState};
use crate::markdown;
use crate::theme::Theme;

pub fn render_chat(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = matches!(app.mode, AppMode::Chat | AppMode::Input);
    let border_style = if is_active {
        app.theme.border_active
    } else {
        app.theme.border_inactive
    };

    let block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);

    if app.messages.is_empty() && app.streaming_buffer.is_empty() {
        let placeholder = Paragraph::new(Line::from(Span::styled(
            "Start a conversation...",
            Style::default().add_modifier(Modifier::DIM),
        )))
        .block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    for msg in &app.messages {
        let (prefix, style) = if msg.role == "user" {
            ("You: ", app.theme.user_message)
        } else {
            ("AI: ", app.theme.assistant_message)
        };

        lines.push(Line::from(Span::styled(
            prefix.to_string(),
            style.add_modifier(Modifier::BOLD),
        )));

        let content_lines = markdown::markdown_to_lines(&msg.content);
        for line in content_lines {
            lines.push(line);
        }

        // Tool calls
        for tool in &msg.tool_calls {
            let icon = match tool.success {
                Some(true) => "\u{2713}",
                Some(false) => "\u{2717}",
                None => "\u{26A1}",
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "  {icon} {} ({}ms)",
                    tool.tool_name,
                    tool.duration_ms.unwrap_or(0)
                ),
                app.theme.tool_call,
            )));
            if let Some(ref output) = tool.output {
                let preview = if output.len() > 200 {
                    format!("{}...", &output[..200])
                } else {
                    output.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("    {preview}"),
                    app.theme.tool_result,
                )));
            }
        }

        lines.push(Line::from("")); // spacing between messages
    }

    // Agent delegation tree (above streaming buffer)
    if let Some(ref delegation) = app.delegation {
        lines.extend(render_delegation_tree(delegation, &app.theme));
        lines.push(Line::from("")); // spacing
    }

    // Streaming buffer
    if !app.streaming_buffer.is_empty() {
        lines.push(Line::from(Span::styled(
            "AI: ".to_string(),
            app.theme.assistant_message.add_modifier(Modifier::BOLD),
        )));
        let buffer_lines = markdown::markdown_to_lines(&app.streaming_buffer);
        for line in buffer_lines {
            lines.push(line);
        }
        if app.chat_status == ChatStatus::Streaming {
            lines.push(Line::from(Span::styled(
                "\u{258C}".to_string(),
                app.theme.streaming_cursor,
            )));
        }
    }

    let total_lines = lines.len();
    let visible = inner.height as usize;

    // Clamp scroll
    let scroll_offset = if total_lines <= visible {
        0
    } else if app.scroll_offset > total_lines - visible {
        total_lines - visible
    } else {
        app.scroll_offset
    };

    let paragraph = Paragraph::new(lines.clone())
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));

    frame.render_widget(paragraph, area);

    // Scrollbar
    if total_lines > visible {
        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(scroll_offset)
            .viewport_content_length(visible);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M tokens", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k tokens", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens} tokens")
    }
}

fn render_delegation_tree<'a>(delegation: &DelegationState, theme: &Theme) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let elapsed = delegation.start_time.elapsed().as_secs();
    let agent_count = delegation.agents.len();

    // Header line: ● Running N agents... (Xs)
    lines.push(Line::from(Span::styled(
        format!("\u{25CF} Running {agent_count} agents... ({elapsed}s)"),
        theme.agent_header,
    )));

    for (i, agent) in delegation.agents.iter().enumerate() {
        let is_last = i == agent_count - 1;
        let connector = if is_last { "\u{2514}\u{2500} " } else { "\u{251C}\u{2500} " };
        let sub_connector = if is_last { "   " } else { "\u{2502}  " };

        // Build the agent summary line
        let (status_icon, status_style, status_suffix) = match &agent.status {
            AgentDisplayStatus::Pending => ("", theme.agent_activity, String::new()),
            AgentDisplayStatus::Running => ("", theme.agent_running, String::new()),
            AgentDisplayStatus::Completed { duration_ms } => {
                let secs = *duration_ms as f64 / 1000.0;
                ("\u{2713} ", theme.agent_complete, format!("completed ({secs:.1}s) \u{00B7} "))
            }
            AgentDisplayStatus::Failed { .. } => {
                ("\u{2717} ", theme.agent_failed, "failed \u{00B7} ".to_string())
            }
        };

        let stats = format!(
            "{} tool uses \u{00B7} {}",
            agent.tool_uses,
            format_tokens(agent.tokens_used),
        );

        lines.push(Line::from(vec![
            Span::styled(connector.to_string(), theme.agent_connector),
            Span::styled(
                format!("{}{}", agent.description, if status_suffix.is_empty() && stats.is_empty() { "" } else { " \u{00B7} " }),
                status_style,
            ),
            Span::styled(format!("{status_icon}{status_suffix}"), status_style),
            Span::styled(stats, theme.agent_activity),
        ]));

        // Activity sub-line
        let activity_text = if agent.current_activity.is_empty() {
            match &agent.status {
                AgentDisplayStatus::Pending => "Pending...".to_string(),
                AgentDisplayStatus::Completed { .. } | AgentDisplayStatus::Failed { .. } => {
                    String::new()
                }
                AgentDisplayStatus::Running => String::new(),
            }
        } else {
            agent.current_activity.clone()
        };

        if !activity_text.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("{sub_connector}\u{2514} "), theme.agent_connector),
                Span::styled(activity_text, theme.agent_activity),
            ]));
        }
    }

    lines
}
