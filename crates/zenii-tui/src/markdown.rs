use std::sync::LazyLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SynStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Convert markdown text to styled ratatui lines.
///
/// Handles: **bold**, *italic*, `inline code`, ```code blocks```,
/// # headers, - lists, and plain text.
pub fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = Vec::new();

    for raw_line in text.lines() {
        if raw_line.starts_with("```") {
            if in_code_block {
                // End of code block — highlight and emit
                let code = code_buf.join("\n");
                lines.extend(highlight_code(&code, &code_lang));
                code_buf.clear();
                in_code_block = false;
            } else {
                // Start of code block
                code_lang = raw_line.trim_start_matches('`').trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_buf.push(raw_line.to_string());
            continue;
        }

        // Headers
        if let Some(header) = raw_line.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(header) = raw_line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(header) = raw_line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        // List items
        if let Some(item) = raw_line.strip_prefix("- ") {
            let mut spans = vec![Span::raw("  \u{2022} ".to_string())];
            spans.extend(parse_inline(item));
            lines.push(Line::from(spans));
            continue;
        }

        // Plain line with inline formatting
        lines.push(Line::from(parse_inline(raw_line)));
    }

    // Flush unclosed code block
    if in_code_block && !code_buf.is_empty() {
        let code = code_buf.join("\n");
        lines.extend(highlight_code(&code, &code_lang));
    }

    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

/// Parse inline markdown formatting: **bold**, *italic*, `code`.
fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut buf = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '`' => {
                if !buf.is_empty() {
                    spans.push(Span::raw(buf.clone()));
                    buf.clear();
                }
                let mut code = String::new();
                for c in chars.by_ref() {
                    if c == '`' {
                        break;
                    }
                    code.push(c);
                }
                spans.push(Span::styled(code, Style::default().fg(Color::Yellow)));
            }
            '*' => {
                if !buf.is_empty() {
                    spans.push(Span::raw(buf.clone()));
                    buf.clear();
                }
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    let mut bold = String::new();
                    while let Some(c) = chars.next() {
                        if c == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold.push(c);
                    }
                    spans.push(Span::styled(
                        bold,
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    let mut italic = String::new();
                    for c in chars.by_ref() {
                        if c == '*' {
                            break;
                        }
                        italic.push(c);
                    }
                    spans.push(Span::styled(
                        italic,
                        Style::default().add_modifier(Modifier::ITALIC),
                    ));
                }
            }
            _ => buf.push(ch),
        }
    }

    if !buf.is_empty() {
        spans.push(Span::raw(buf));
    }

    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }

    spans
}

/// Highlight a code block using syntect.
pub fn highlight_code(code: &str, language: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_by_token(language)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {language} "),
        Style::default().bg(Color::DarkGray).fg(Color::White),
    )));

    for line in code.lines() {
        let highlighted = highlighter
            .highlight_line(line, &SYNTAX_SET)
            .unwrap_or_default();

        let spans: Vec<Span<'static>> = highlighted
            .into_iter()
            .map(|(style, text)| Span::styled(text.to_string(), syn_style_to_ratatui(style)))
            .collect();

        lines.push(Line::from(spans));
    }

    lines
}

fn syn_style_to_ratatui(style: SynStyle) -> Style {
    let fg = style.foreground;
    Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_text() {
        let lines = markdown_to_lines("**hello**");
        let spans = &lines[0].spans;
        assert!(
            spans
                .iter()
                .any(|s| { s.content.contains("hello") && s.style.add_modifier == Modifier::BOLD })
        );
    }

    #[test]
    fn inline_code() {
        let lines = markdown_to_lines("use `foo` here");
        let spans = &lines[0].spans;
        assert!(
            spans
                .iter()
                .any(|s| s.content.contains("foo") && s.style.fg == Some(Color::Yellow))
        );
    }

    #[test]
    fn code_block_highlighting() {
        let md = "```rust\nfn main() {}\n```";
        let lines = markdown_to_lines(md);
        // Should have language header + at least one highlighted line
        assert!(lines.len() >= 2);
        // First line is language header
        assert!(lines[0].spans.iter().any(|s| s.content.contains("rust")));
    }

    #[test]
    fn plain_text() {
        let lines = markdown_to_lines("hello world");
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn nested_formatting() {
        let lines = markdown_to_lines("**bold *and italic***");
        // Should produce at least one styled span
        assert!(!lines.is_empty());
        let has_bold = lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier == Modifier::BOLD);
        assert!(has_bold);
    }
}
