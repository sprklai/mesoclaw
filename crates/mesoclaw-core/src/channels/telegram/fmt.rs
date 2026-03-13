use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Telegram's maximum message length.
const TELEGRAM_MAX_LENGTH: usize = 4096;

/// Escape HTML special characters in text content.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Convert standard Markdown to Telegram-compatible HTML.
///
/// Telegram supports: `<b>`, `<i>`, `<code>`, `<pre>`, `<a href="">`,
/// `<s>`, `<u>`, `<blockquote>`, `<pre><code class="language-X">`.
pub fn markdown_to_html(markdown: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(markdown, options);

    let mut html = String::with_capacity(markdown.len());

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Strong => html.push_str("<b>"),
                Tag::Emphasis => html.push_str("<i>"),
                Tag::Strikethrough => html.push_str("<s>"),
                Tag::Link { dest_url, .. } => {
                    html.push_str("<a href=\"");
                    html.push_str(&escape_html(&dest_url));
                    html.push_str("\">");
                }
                Tag::CodeBlock(kind) => match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                        html.push_str("<pre><code class=\"language-");
                        html.push_str(&escape_html(&lang));
                        html.push_str("\">");
                    }
                    _ => html.push_str("<pre><code>"),
                },
                Tag::BlockQuote(_) => html.push_str("<blockquote>"),
                Tag::List(_) => {}
                Tag::Item => html.push_str("• "),
                Tag::Heading { .. } => html.push_str("<b>"),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => html.push_str("\n\n"),
                TagEnd::Strong => html.push_str("</b>"),
                TagEnd::Emphasis => html.push_str("</i>"),
                TagEnd::Strikethrough => html.push_str("</s>"),
                TagEnd::Link => html.push_str("</a>"),
                TagEnd::CodeBlock => {
                    html.push_str("</code></pre>\n");
                }
                TagEnd::BlockQuote(_) => html.push_str("</blockquote>\n"),
                TagEnd::Item => html.push('\n'),
                TagEnd::Heading(_) => html.push_str("</b>\n"),
                _ => {}
            },
            Event::Text(text) => {
                html.push_str(&escape_html(&text));
            }
            Event::Code(code) => {
                html.push_str("<code>");
                html.push_str(&escape_html(&code));
                html.push_str("</code>");
            }
            Event::SoftBreak => html.push('\n'),
            Event::HardBreak => html.push('\n'),
            Event::Rule => html.push_str("\n---\n"),
            _ => {}
        }
    }

    // Trim trailing whitespace/newlines
    html.trim_end().to_string()
}

/// Split a message into chunks that fit within Telegram's 4096-char limit.
/// Prefers splitting at paragraph > newline > sentence > word boundaries.
pub fn split_message(text: &str) -> Vec<String> {
    if text.len() <= TELEGRAM_MAX_LENGTH {
        return vec![text.to_string()];
    }

    let mut parts = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= TELEGRAM_MAX_LENGTH {
            parts.push(remaining.to_string());
            break;
        }

        let chunk = &remaining[..TELEGRAM_MAX_LENGTH];

        // Try split boundaries in preference order
        let split_pos = find_split_point(chunk);

        parts.push(remaining[..split_pos].to_string());
        remaining = remaining[split_pos..].trim_start();
    }

    parts
}

fn find_split_point(chunk: &str) -> usize {
    // 1. Paragraph boundary (double newline)
    if let Some(pos) = chunk.rfind("\n\n").filter(|&p| p > 0) {
        return pos;
    }

    // 2. Newline boundary
    if let Some(pos) = chunk.rfind('\n').filter(|&p| p > 0) {
        return pos;
    }

    // 3. Sentence boundary (". ")
    if let Some(pos) = chunk.rfind(". ").filter(|&p| p > 0) {
        return pos + 1; // Include the period
    }

    // 4. Word boundary (space)
    if let Some(pos) = chunk.rfind(' ').filter(|&p| p > 0) {
        return pos;
    }

    // 5. Hard cut at max length
    chunk.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_bold() {
        let result = markdown_to_html("Hello **world**!");
        assert!(result.contains("<b>world</b>"));
        assert!(result.contains("Hello "));
    }

    #[test]
    fn html_italic() {
        let result = markdown_to_html("Hello *world*!");
        assert!(result.contains("<i>world</i>"));
    }

    #[test]
    fn html_inline_code() {
        let result = markdown_to_html("Use `cargo build` here");
        assert!(result.contains("<code>cargo build</code>"));
    }

    #[test]
    fn html_code_block() {
        let result = markdown_to_html("```rust\nfn main() {}\n```");
        assert!(result.contains("<pre><code class=\"language-rust\">"));
        assert!(result.contains("fn main() {}"));
        assert!(result.contains("</code></pre>"));
    }

    #[test]
    fn html_code_block_no_lang() {
        let result = markdown_to_html("```\nsome code\n```");
        assert!(result.contains("<pre><code>"));
        assert!(result.contains("some code"));
    }

    #[test]
    fn html_link() {
        let result = markdown_to_html("[click](https://example.com)");
        assert!(result.contains("<a href=\"https://example.com\">click</a>"));
    }

    #[test]
    fn html_strikethrough() {
        let result = markdown_to_html("~~deleted~~");
        assert!(result.contains("<s>deleted</s>"));
    }

    #[test]
    fn html_blockquote() {
        let result = markdown_to_html("> quoted text");
        assert!(result.contains("<blockquote>"));
        assert!(result.contains("quoted text"));
        assert!(result.contains("</blockquote>"));
    }

    #[test]
    fn html_escapes_special_chars() {
        let result = markdown_to_html("a < b & c > d");
        assert!(result.contains("&lt;"));
        assert!(result.contains("&amp;"));
        assert!(result.contains("&gt;"));
        // Original chars should not appear unescaped
        assert!(!result.contains(" < "));
        assert!(!result.contains(" & "));
        assert!(!result.contains(" > "));
    }

    #[test]
    fn html_heading() {
        let result = markdown_to_html("# Title");
        assert!(result.contains("<b>Title</b>"));
    }

    #[test]
    fn html_empty_string() {
        assert_eq!(markdown_to_html(""), "");
    }

    #[test]
    fn html_plain_text() {
        let result = markdown_to_html("Just plain text");
        assert_eq!(result, "Just plain text");
    }

    #[test]
    fn html_paragraph_spacing_preserved() {
        let result = markdown_to_html("First paragraph.\n\nSecond paragraph.");
        assert!(
            result.contains("\n\n"),
            "Paragraph break should produce double newline, got: {result}"
        );
        assert!(result.contains("First paragraph."));
        assert!(result.contains("Second paragraph."));
    }

    #[test]
    fn short_message_no_split() {
        let msg = "Hello, world!";
        let parts = split_message(msg);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], msg);
    }

    #[test]
    fn split_at_paragraph() {
        let part1 = "a".repeat(3000);
        let part2 = "b".repeat(3000);
        let msg = format!("{part1}\n\n{part2}");
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        assert!(parts[0].len() <= TELEGRAM_MAX_LENGTH);
    }

    #[test]
    fn split_at_newline() {
        let part1 = "a".repeat(3000);
        let part2 = "b".repeat(3000);
        let msg = format!("{part1}\n{part2}");
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        assert!(parts[0].len() <= TELEGRAM_MAX_LENGTH);
    }

    #[test]
    fn split_at_sentence() {
        let sentence = "This is a sentence. ";
        let msg = sentence.repeat(250);
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        assert!(parts[0].ends_with('.'));
    }

    #[test]
    fn split_at_word() {
        let word = "word ";
        let msg = word.repeat(1000);
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        assert!(parts[0].len() <= TELEGRAM_MAX_LENGTH);
    }

    #[test]
    fn force_split_max() {
        let msg = "x".repeat(5000);
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        assert_eq!(parts[0].len(), TELEGRAM_MAX_LENGTH);
    }

    #[test]
    fn split_parts_concatenate() {
        let part1 = "a".repeat(3000);
        let part2 = "b".repeat(3000);
        let original = format!("{part1}\n\n{part2}");
        let parts = split_message(&original);
        let reconstructed: String = parts.join("");
        assert!(reconstructed.contains(&part1));
        assert!(reconstructed.contains(&part2));
    }
}
