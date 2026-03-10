use async_trait::async_trait;

use super::{ReasoningStrategy, StrategyContext};

/// ReAct-style continuation strategy that detects incomplete responses.
///
/// Ported from v1 `loop_.rs:504-530` with additional refusal detection.
/// When the agent describes what it *would* do instead of actually calling tools,
/// or when it refuses with "I can't access", this strategy nudges it to try again.
pub struct ContinuationStrategy {
    max_interventions: u32,
}

impl ContinuationStrategy {
    pub fn new(max: u32) -> Self {
        Self {
            max_interventions: max,
        }
    }

    /// Check if a response looks incomplete — the agent described actions
    /// instead of taking them, or refused prematurely.
    pub fn looks_incomplete(content: &str) -> bool {
        if content.is_empty() {
            return false;
        }

        let lower = content.to_lowercase();

        // Planning language — agent says what it will do instead of doing it
        let plan_markers = [
            "i will ",
            "i'll ",
            "let me ",
            "next step",
            "step 1",
            "first, i'll",
            "i need to ",
            "i'm going to ",
            "first i should",
            "the next step",
            "my plan is",
            "i would need to",
            "i should search",
            "i should look",
            "to answer this",
            "to find out",
            "to check",
            "first, let's",
            "first i'll",
        ];

        for marker in &plan_markers {
            if lower.contains(marker) {
                return true;
            }
        }

        // Refusal language — agent gives up without trying tools
        let refusal_markers = [
            "i can't access",
            "i don't have access",
            "i cannot access",
            "unable to access",
            "i'm unable to",
        ];

        for marker in &refusal_markers {
            if lower.contains(marker) {
                return true;
            }
        }

        // Numbered list detection: lines starting with "1." and "2." suggest a plan
        let has_numbered_list = lower.lines().any(|l| {
            let t = l.trim();
            t.starts_with("1.") || t.starts_with("1)")
        }) && lower.lines().any(|l| {
            let t = l.trim();
            t.starts_with("2.") || t.starts_with("2)")
        });
        if has_numbered_list {
            return true;
        }

        false
    }

    fn build_nudge(&self, intervention_count: u32) -> String {
        format!(
            "You described what you would do but did NOT call any tools (attempt {}/{}). \
             You MUST use your available tools now. Do NOT say 'I will search' — call file_list, \
             shell, or other tools directly. If you truly cannot help, provide your final answer.",
            intervention_count + 1,
            self.max_interventions
        )
    }
}

#[async_trait]
impl ReasoningStrategy for ContinuationStrategy {
    fn name(&self) -> &str {
        "continuation"
    }

    async fn evaluate(&self, context: &StrategyContext) -> Option<String> {
        if Self::looks_incomplete(&context.response) {
            Some(self.build_nudge(context.intervention_count))
        } else {
            None
        }
    }

    fn max_interventions(&self) -> u32 {
        self.max_interventions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 8.11.1 — looks_incomplete with plan markers
    #[test]
    fn looks_incomplete_with_plan_markers() {
        assert!(ContinuationStrategy::looks_incomplete(
            "I will search for the files on your desktop"
        ));
        assert!(ContinuationStrategy::looks_incomplete(
            "Let me check your downloads folder"
        ));
        assert!(ContinuationStrategy::looks_incomplete(
            "I'm going to look at the directory"
        ));
        assert!(ContinuationStrategy::looks_incomplete(
            "First, I'll list the contents"
        ));
    }

    // 8.11.2 — looks_incomplete with numbered steps (lines starting with numbers)
    #[test]
    fn looks_incomplete_with_numbered_steps() {
        assert!(ContinuationStrategy::looks_incomplete(
            "Here's my plan:\n1. Search the desktop\n2. Open the files"
        ));
        // Inline numbers like "file1.txt" should NOT trigger
        assert!(!ContinuationStrategy::looks_incomplete(
            "Here are the results: file1.txt, file2.txt"
        ));
    }

    // 8.11.3 — looks_incomplete with direct answer returns false
    #[test]
    fn looks_incomplete_with_direct_answer() {
        assert!(!ContinuationStrategy::looks_incomplete(
            "The file is at /home/user/test.txt"
        ));
        assert!(!ContinuationStrategy::looks_incomplete(
            "Here are the results: file1.txt, file2.txt"
        ));
    }

    // 8.11.4 — looks_incomplete with refusal
    #[test]
    fn looks_incomplete_with_refusal() {
        assert!(ContinuationStrategy::looks_incomplete(
            "I can't access your desktop directory"
        ));
        assert!(ContinuationStrategy::looks_incomplete(
            "I'm unable to view your files"
        ));
        assert!(ContinuationStrategy::looks_incomplete(
            "Unable to access the downloads folder"
        ));
    }

    // 8.11.5 — looks_incomplete empty string
    #[test]
    fn looks_incomplete_empty_string() {
        assert!(!ContinuationStrategy::looks_incomplete(""));
    }

    // 8.11.6 — looks_incomplete short definitive
    #[test]
    fn looks_incomplete_short_definitive() {
        assert!(!ContinuationStrategy::looks_incomplete("Done."));
        assert!(!ContinuationStrategy::looks_incomplete("The answer is 42."));
    }

    // 8.11.7 — strategy name
    #[test]
    fn continuation_strategy_name() {
        let strategy = ContinuationStrategy::new(3);
        assert_eq!(strategy.name(), "continuation");
    }

    // 8.11.8 — strategy max interventions
    #[test]
    fn continuation_strategy_max() {
        let strategy = ContinuationStrategy::new(5);
        assert_eq!(strategy.max_interventions(), 5);
    }

    // 8.11.9 — evaluate returns nudge when incomplete
    #[tokio::test]
    async fn evaluate_returns_nudge_when_incomplete() {
        let strategy = ContinuationStrategy::new(3);
        let ctx = StrategyContext {
            original_prompt: "find text files on my desktop".into(),
            response: "I will search for text files on your desktop".into(),
            intervention_count: 0,
        };
        let result = strategy.evaluate(&ctx).await;
        assert!(result.is_some());
        let nudge = result.unwrap();
        assert!(nudge.contains("attempt 1/3"));
        assert!(nudge.contains("MUST use your available tools"));
    }
}
