/// Convert standard Markdown bold (**text**) to Slack mrkdwn bold (*text*).
pub fn markdown_to_mrkdwn_bold(text: &str) -> String {
    text.replace("**", "*")
}

/// Convert standard Markdown italic (*text* or _text_) to Slack mrkdwn italic (_text_).
/// Only converts *text* that isn't already bold (**text**).
pub fn markdown_to_mrkdwn_italic(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '*' {
            // Check if this is bold (** or already converted *)
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                // Bold marker — keep as-is
                result.push('*');
                result.push('*');
                i += 2;
            } else {
                // Single * — convert to _ for italic
                result.push('_');
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Convert standard Markdown code blocks (```lang\n...\n```) to Slack mrkdwn (```\n...\n```).
/// Slack doesn't support language hints in code blocks.
pub fn markdown_to_mrkdwn_code_block(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("```") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + 3..];

        // Skip language identifier (everything until newline)
        if let Some(newline_pos) = remaining.find('\n') {
            let lang_hint = &remaining[..newline_pos];
            // If there's a non-empty language hint, skip it
            if !lang_hint.trim().is_empty()
                && lang_hint
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                remaining = &remaining[newline_pos..];
            }
        }

        result.push_str("```");
    }

    result.push_str(remaining);
    result
}

/// Convert standard Markdown links [text](url) to Slack mrkdwn <url|text>.
pub fn markdown_to_mrkdwn_link(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(bracket_start) = remaining.find('[') {
        result.push_str(&remaining[..bracket_start]);
        remaining = &remaining[bracket_start..];

        if let Some(bracket_end) = remaining.find("](") {
            let link_text = &remaining[1..bracket_end];
            let after_paren = &remaining[bracket_end + 2..];

            if let Some(paren_end) = after_paren.find(')') {
                let url = &after_paren[..paren_end];
                result.push_str(&format!("<{url}|{link_text}>"));
                remaining = &after_paren[paren_end + 1..];
                continue;
            }
        }

        // Not a valid link, keep the bracket
        result.push('[');
        remaining = &remaining[1..];
    }

    result.push_str(remaining);
    result
}

/// Escape special mrkdwn characters: &, <, >
pub fn escape_mrkdwn(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Apply all markdown-to-mrkdwn conversions.
pub fn to_mrkdwn(text: &str) -> String {
    let text = markdown_to_mrkdwn_code_block(text);
    let text = markdown_to_mrkdwn_bold(&text);
    let text = markdown_to_mrkdwn_italic(&text);
    markdown_to_mrkdwn_link(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_conversion() {
        assert_eq!(markdown_to_mrkdwn_bold("**hello**"), "*hello*");
        assert_eq!(
            markdown_to_mrkdwn_bold("normal **bold** text"),
            "normal *bold* text"
        );
    }

    #[test]
    fn italic_conversion() {
        assert_eq!(markdown_to_mrkdwn_italic("*hello*"), "_hello_");
        assert_eq!(
            markdown_to_mrkdwn_italic("**bold** and *italic*"),
            "**bold** and _italic_"
        );
    }

    #[test]
    fn code_block_conversion() {
        let input = "```rust\nfn main() {}\n```";
        let expected = "```\nfn main() {}\n```";
        assert_eq!(markdown_to_mrkdwn_code_block(input), expected);

        // No language hint — unchanged
        let input2 = "```\ncode\n```";
        assert_eq!(markdown_to_mrkdwn_code_block(input2), input2);
    }

    #[test]
    fn link_conversion() {
        assert_eq!(
            markdown_to_mrkdwn_link("[click here](https://example.com)"),
            "<https://example.com|click here>"
        );
        assert_eq!(
            markdown_to_mrkdwn_link("before [link](http://foo.bar) after"),
            "before <http://foo.bar|link> after"
        );
    }

    #[test]
    fn escape_special() {
        assert_eq!(escape_mrkdwn("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert_eq!(escape_mrkdwn("normal text"), "normal text");
    }
}
