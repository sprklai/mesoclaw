use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table};

use crate::app::App;

pub fn render_plugins(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Plugins ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    if app.plugins.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No plugins installed",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'i' to install a plugin",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(empty_lines)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        render_plugin_footer(frame, area);
        return;
    }

    // Build table rows
    let header = Row::new(vec!["Name", "Version", "State", "Tools", "Description"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .plugins
        .iter()
        .enumerate()
        .map(|(i, plugin)| {
            let state = if plugin.enabled { "on" } else { "off" };
            let state_style = if plugin.enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let selected = app.selected_plugin == Some(i);
            let row_style = if selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Span::styled(plugin.name.clone(), row_style),
                Span::styled(plugin.version.clone(), row_style),
                Span::styled(state.to_string(), state_style),
                Span::styled(plugin.tools_count.to_string(), row_style),
                Span::styled(truncate(&plugin.description, 40), row_style.fg(Color::Gray)),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(20),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Length(6),
        ratatui::layout::Constraint::Length(6),
        ratatui::layout::Constraint::Min(20),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, area);
    render_plugin_footer(frame, area);

    // Show error if present
    if let Some(ref err) = app.plugin_error {
        let err_area = Rect {
            x: area.x + 2,
            y: area.y + area.height.saturating_sub(3),
            width: area.width.saturating_sub(4),
            height: 1,
        };
        let err_line = Paragraph::new(Span::styled(
            truncate(err, err_area.width as usize),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(err_line, err_area);
    }
}

fn render_plugin_footer(frame: &mut Frame, area: Rect) {
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let local_indicator = " l:local";

    let footer = Line::from(vec![
        Span::styled(" j/k", Style::default().fg(Color::Yellow)),
        Span::raw(":nav  "),
        Span::styled("e", Style::default().fg(Color::Yellow)),
        Span::raw(":toggle  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(":remove  "),
        Span::styled("i", Style::default().fg(Color::Yellow)),
        Span::raw(":install  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(":refresh"),
        Span::styled(local_indicator, Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(":back"),
    ]);

    let paragraph = Paragraph::new(footer).style(Style::default().bg(Color::Black));
    frame.render_widget(paragraph, footer_area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}
