//! Heartbeat checklist parsing from `HEARTBEAT.md`.
//!
//! The heartbeat is a periodic "health check" that the agent runs against a
//! configurable checklist.  The checklist is stored in `HEARTBEAT.md` inside
//! the identity directory.

/// Parse checklist items from a `HEARTBEAT.md` file.
///
/// Lines that start with `- [ ]` or `- [x]` (case-insensitive) are extracted
/// as action items.  The returned strings contain only the item text (the
/// checkbox prefix is stripped).
pub fn parse_heartbeat_items(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Match "- [ ] Item" or "- [x] Item" (completed items included
            // so the agent can verify they remain passing).
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

/// Default heartbeat interval in seconds (30 minutes).
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30 * 60;

/// Error back-off levels in seconds: 30s → 60s → 300s → 900s → 3600s.
pub const ERROR_BACKOFF_SECS: &[u64] = &[30, 60, 300, 900, 3_600];

/// Return the back-off delay for `error_count` consecutive failures.
///
/// After `ERROR_BACKOFF_SECS.len()` failures the maximum back-off is used.
pub fn backoff_secs(error_count: u32) -> u64 {
    let idx = (error_count as usize).min(ERROR_BACKOFF_SECS.len() - 1);
    ERROR_BACKOFF_SECS[idx]
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HEARTBEAT: &str = r#"# Heartbeat Checks

Run these checks periodically:

- [ ] Verify disk space is below 90%
- [ ] Check API key validity
- [x] Confirm log rotation is active
- [X] Validate config files exist

## Notes

These are handled automatically by the agent.
"#;

    #[test]
    fn parses_unchecked_items() {
        let items = parse_heartbeat_items(SAMPLE_HEARTBEAT);
        assert!(
            items.contains(&"Verify disk space is below 90%".to_string()),
            "should parse unchecked item"
        );
        assert!(
            items.contains(&"Check API key validity".to_string()),
            "should parse second unchecked item"
        );
    }

    #[test]
    fn parses_checked_items_lowercase() {
        let items = parse_heartbeat_items(SAMPLE_HEARTBEAT);
        assert!(
            items.contains(&"Confirm log rotation is active".to_string()),
            "should parse [x] item"
        );
    }

    #[test]
    fn parses_checked_items_uppercase() {
        let items = parse_heartbeat_items(SAMPLE_HEARTBEAT);
        assert!(
            items.contains(&"Validate config files exist".to_string()),
            "should parse [X] item"
        );
    }

    #[test]
    fn ignores_non_checklist_lines() {
        let items = parse_heartbeat_items(SAMPLE_HEARTBEAT);
        // Section headers and plain text should be absent.
        assert!(!items.iter().any(|i| i.contains("Notes")));
        assert!(!items.iter().any(|i| i.contains("Heartbeat")));
        assert!(!items.iter().any(|i| i.contains("automatically")));
    }

    #[test]
    fn empty_content_returns_empty() {
        let items = parse_heartbeat_items("");
        assert!(items.is_empty(), "empty content → no items");
    }

    #[test]
    fn no_checklist_lines_returns_empty() {
        let content = "# Heartbeat\n\nJust some text.";
        let items = parse_heartbeat_items(content);
        assert!(items.is_empty(), "no checklist lines → no items");
    }

    #[test]
    fn item_count_correct() {
        let items = parse_heartbeat_items(SAMPLE_HEARTBEAT);
        assert_eq!(items.len(), 4, "should find exactly 4 checklist items");
    }

    #[test]
    fn backoff_first_failure() {
        assert_eq!(backoff_secs(0), 30, "first failure → 30s");
    }

    #[test]
    fn backoff_second_failure() {
        assert_eq!(backoff_secs(1), 60, "second failure → 60s");
    }

    #[test]
    fn backoff_caps_at_max() {
        let max = *ERROR_BACKOFF_SECS.last().unwrap();
        assert_eq!(backoff_secs(100), max, "many failures → max back-off");
    }
}
