pub const DEFAULT_SYSTEM_PROMPT: &str = include_str!("defaults/system-prompt.md");
pub const DEFAULT_SUMMARIZE: &str = include_str!("defaults/summarize.md");

/// All bundled skills as (id, content) pairs.
pub const BUNDLED_SKILLS: &[(&str, &str)] = &[
    ("system-prompt", DEFAULT_SYSTEM_PROMPT),
    ("summarize", DEFAULT_SUMMARIZE),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_system_prompt_skill_not_empty() {
        assert!(!DEFAULT_SYSTEM_PROMPT.is_empty());
        assert!(DEFAULT_SYSTEM_PROMPT.contains("system-prompt"));
    }

    #[test]
    fn bundled_summarize_skill_not_empty() {
        assert!(!DEFAULT_SUMMARIZE.is_empty());
        assert!(DEFAULT_SUMMARIZE.contains("summarize"));
    }
}
