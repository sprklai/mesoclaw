/// Telegram's maximum message length.
const TELEGRAM_MAX_LENGTH: usize = 4096;

/// Characters that must be escaped in MarkdownV2 mode.
/// See: https://core.telegram.org/bots/api#markdownv2-style
const RESERVED_CHARS: &[char] = &[
    '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
];

/// Escape reserved MarkdownV2 characters in a string.
pub fn escape_markdown_v2(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if RESERVED_CHARS.contains(&ch) {
            result.push('\\');
        }
        result.push(ch);
    }
    result
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
    fn escape_all_reserved() {
        let input = "_*[]()~`>#+-=|{}.!";
        let escaped = escape_markdown_v2(input);
        assert_eq!(
            escaped,
            "\\_\\*\\[\\]\\(\\)\\~\\`\\>\\#\\+\\-\\=\\|\\{\\}\\.\\!"
        );
    }

    #[test]
    fn escape_preserves_normal() {
        let input = "Hello, world! This is a test.";
        let escaped = escape_markdown_v2(input);
        // Only '!' and '.' should be escaped
        assert!(escaped.contains("Hello, world\\!"));
        assert!(escaped.contains("test\\."));
        // Letters, spaces, commas are preserved
        assert!(escaped.contains("Hello"));
        assert!(escaped.contains("world"));
    }

    #[test]
    fn escape_empty_string() {
        assert_eq!(escape_markdown_v2(""), "");
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
        // First part should split at the paragraph boundary
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
        // Create a message with sentences, no newlines, > 4096 chars
        let sentence = "This is a sentence. ";
        let msg = sentence.repeat(250); // ~5000 chars
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        // Each part should end at a sentence boundary (period)
        assert!(parts[0].ends_with('.'));
    }

    #[test]
    fn split_at_word() {
        // Create a message with only word boundaries, > 4096 chars
        let word = "word ";
        let msg = word.repeat(1000); // 5000 chars
        let parts = split_message(&msg);
        assert!(parts.len() >= 2);
        assert!(parts[0].len() <= TELEGRAM_MAX_LENGTH);
    }

    #[test]
    fn force_split_max() {
        // Single continuous string with no split points
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

        // Concatenating all parts should reproduce the original
        // (minus any trimmed whitespace between parts)
        let reconstructed: String = parts.join("");
        // All content should be present
        assert!(reconstructed.contains(&part1));
        assert!(reconstructed.contains(&part2));
    }
}
