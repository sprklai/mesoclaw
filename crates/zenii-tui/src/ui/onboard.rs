use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, OnboardField, OnboardStep};

pub fn render_onboard(frame: &mut Frame, area: Rect, app: &App) {
    let overlay = centered_rect(60, 70, area);
    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Welcome to Zenii ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(overlay);
    frame.render_widget(block, overlay);

    // Step indicator
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // step indicator
            Constraint::Min(1),    // content
            Constraint::Length(2), // footer
        ])
        .split(inner);

    render_step_indicator(frame, chunks[0], app.onboard_step);

    match app.onboard_step {
        OnboardStep::ProviderSelect => render_provider_select(frame, chunks[1], app),
        OnboardStep::ApiKey => render_api_key(frame, chunks[1], app),
        OnboardStep::ModelSelect => render_model_select(frame, chunks[1], app),
        OnboardStep::Profile => render_profile(frame, chunks[1], app),
    }

    render_footer(frame, chunks[2], app);
}

fn render_step_indicator(frame: &mut Frame, area: Rect, step: OnboardStep) {
    let steps = ["Provider", "API Key", "Model", "Profile"];
    let current = match step {
        OnboardStep::ProviderSelect => 0,
        OnboardStep::ApiKey => 1,
        OnboardStep::ModelSelect => 2,
        OnboardStep::Profile => 3,
    };

    let mut spans = Vec::new();
    for (i, name) in steps.iter().enumerate() {
        let style = if i == current {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if i < current {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let marker = if i < current {
            "[x]"
        } else if i == current {
            "[>]"
        } else {
            "[ ]"
        };
        spans.push(Span::styled(format!(" {marker} {name} "), style));
        if i < steps.len() - 1 {
            spans.push(Span::raw("---"));
        }
    }

    let indicator = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    frame.render_widget(indicator, area);
}

fn render_provider_select(frame: &mut Frame, area: Rect, app: &App) {
    if app.onboard_providers.is_empty() {
        let msg = Paragraph::new("Loading providers...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .onboard_providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = p["name"].as_str().unwrap_or("Unknown");
            let style = if i == app.onboard_selected_provider {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let marker = if i == app.onboard_selected_provider {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{marker}{name}")).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Choose your AI provider ")
            .borders(Borders::TOP),
    );
    frame.render_widget(list, area);
}

fn render_api_key(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let prompt = Paragraph::new(format!("Enter your {} API key:", app.onboard_provider_id))
        .style(Style::default().fg(Color::White));
    frame.render_widget(prompt, chunks[0]);

    let masked: String = "*".repeat(app.onboard_api_key.content.len());
    let input = Paragraph::new(if masked.is_empty() {
        "(type your key)".to_string()
    } else {
        masked
    })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(input, chunks[1]);

    if let Some(ref err) = app.onboard_error {
        let err_msg = Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(err_msg, chunks[2]);
    }
}

fn render_model_select(frame: &mut Frame, area: Rect, app: &App) {
    if app.onboard_models.is_empty() {
        let msg = Paragraph::new("No models available for this provider.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .onboard_models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m["display_name"]
                .as_str()
                .unwrap_or(m["model_id"].as_str().unwrap_or("Unknown"));
            let style = if i == app.onboard_selected_model {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let marker = if i == app.onboard_selected_model {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{marker}{name}")).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Select a model ")
            .borders(Borders::TOP),
    );
    frame.render_widget(list, area);
}

fn render_profile(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Location
            Constraint::Length(3), // Timezone
            Constraint::Min(0),    // Error/status
        ])
        .split(area);

    let fields = [
        ("Name", &app.onboard_name, OnboardField::Name),
        ("Location", &app.onboard_location, OnboardField::Location),
        ("Timezone", &app.onboard_timezone, OnboardField::Timezone),
    ];

    for (i, (label, input, field)) in fields.iter().enumerate() {
        let is_active = app.onboard_field == *field;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let display = if input.content.is_empty() {
            format!("({label})")
        } else {
            input.content.clone()
        };
        let p = Paragraph::new(display).block(
            Block::default()
                .title(format!(" {label} "))
                .borders(Borders::ALL)
                .border_style(border_style),
        );
        frame.render_widget(p, chunks[i]);
    }

    if app.onboard_saving {
        let saving = Paragraph::new("Saving...").style(Style::default().fg(Color::Yellow));
        frame.render_widget(saving, chunks[3]);
    } else if let Some(ref err) = app.onboard_error {
        let err_msg = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        frame.render_widget(err_msg, chunks[3]);
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hint = match app.onboard_step {
        OnboardStep::ProviderSelect => "j/k: Navigate | Enter: Select | q: Quit",
        OnboardStep::ApiKey => "Type key | Enter: Save | Esc: Back",
        OnboardStep::ModelSelect => "j/k: Navigate | Enter: Select | Esc: Back",
        OnboardStep::Profile => "Tab: Next Field | Enter: Save | Esc: Back",
    };
    let footer = Paragraph::new(hint)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, area);
}

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
