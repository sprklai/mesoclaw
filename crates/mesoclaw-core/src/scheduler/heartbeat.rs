/// Parse checklist items from a heartbeat markdown file.
///
/// Lines starting with `- [ ]`, `- [x]`, or `- [X]` are extracted.
/// Returns only the item text (checkbox prefix stripped).
pub fn parse_heartbeat_items(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("- [ ]") {
                Some(rest.trim().to_owned())
            } else if let Some(rest) = trimmed.strip_prefix("- [x]") {
                Some(rest.trim().to_owned())
            } else {
                trimmed
                    .strip_prefix("- [X]")
                    .map(|rest| rest.trim().to_owned())
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Error back-off levels in seconds: 30s -> 60s -> 300s -> 900s -> 3600s.
pub const ERROR_BACKOFF_SECS: &[u64] = &[30, 60, 300, 900, 3_600];

/// Return the back-off delay for `error_count` consecutive failures.
pub fn backoff_secs(error_count: u32) -> u64 {
    let idx = (error_count as usize).min(ERROR_BACKOFF_SECS.len() - 1);
    ERROR_BACKOFF_SECS[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# Heartbeat Checks

- [ ] Verify disk space is below 90%
- [ ] Check API key validity
- [x] Confirm log rotation is active
- [X] Validate config files exist

## Notes

Just text.
"#;

    // 16.28 — Parse markdown checklist items
    #[test]
    fn parse_checklist() {
        let items = parse_heartbeat_items(SAMPLE);
        assert_eq!(items.len(), 4);
        assert!(items.contains(&"Verify disk space is below 90%".to_string()));
        assert!(items.contains(&"Check API key validity".to_string()));
        assert!(items.contains(&"Confirm log rotation is active".to_string()));
        assert!(items.contains(&"Validate config files exist".to_string()));
    }

    // 16.29 — Empty checklist returns empty
    #[test]
    fn parse_empty() {
        let items = parse_heartbeat_items("");
        assert!(items.is_empty());
    }

    // 16.30 — Mixed checked/unchecked items
    #[test]
    fn parse_mixed() {
        let content = "- [ ] Todo\n- [x] Done\nNot a checklist";
        let items = parse_heartbeat_items(content);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "Todo");
        assert_eq!(items[1], "Done");
    }
}
