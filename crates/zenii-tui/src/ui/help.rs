use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_help(frame: &mut Frame, area: Rect) {
    // Clear area behind overlay
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let help_lines = vec![
        Line::from(Span::styled(
            "Global",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        help_line("Ctrl+C", "Quit"),
        help_line("?", "Toggle this help"),
        help_line("Tab", "Switch pane focus"),
        help_line("Ctrl+R", "Refresh data"),
        Line::from(""),
        Line::from(Span::styled(
            "Session List",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        help_line("j/\u{2193}", "Move down"),
        help_line("k/\u{2191}", "Move up"),
        help_line("Enter", "Open session"),
        help_line("n", "New session"),
        help_line("d", "Delete session"),
        help_line("g", "Jump to first"),
        help_line("G", "Jump to last"),
        help_line("p", "Plugin list"),
        Line::from(""),
        Line::from(Span::styled(
            "Chat",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        help_line("j/k", "Scroll up/down"),
        help_line("PgUp/PgDn", "Scroll by page"),
        help_line("g/G", "Top/bottom"),
        help_line("i", "Enter input mode"),
        help_line("Esc", "Back to session list"),
        Line::from(""),
        Line::from(Span::styled(
            "Input",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        help_line("Enter", "Send message"),
        help_line("Esc", "Exit input mode"),
        help_line("\u{2191}/\u{2193}", "History navigation"),
        help_line("Ctrl+U", "Clear line"),
        help_line("Ctrl+W", "Delete word"),
        help_line("Ctrl+A/E", "Home/End"),
    ];

    let paragraph = Paragraph::new(help_lines).block(block);
    frame.render_widget(paragraph, area);
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {key:<14}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(desc),
    ])
}
