use async_trait::async_trait;
use rig::message::Message;
use tracing::{debug, info};

use crate::Result;
use crate::ai::agent::MesoAgent;

pub mod continuation;

/// Context passed to strategies for evaluation.
#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub original_prompt: String,
    pub response: String,
    pub intervention_count: u32,
    /// Number of actual (non-cached) tool executions so far in this request.
    /// When > 0, strategies can skip nudges since tools were already used.
    pub tool_calls_made: u32,
}

/// Result of a reasoned chat.
#[derive(Debug, Clone)]
pub struct ChatResult {
    pub response: String,
    pub interventions_used: u32,
    pub strategy_used: Option<String>,
}

/// A reasoning strategy that can inspect and augment agent responses.
///
/// Strategies are composable — multiple can be applied in sequence.
/// The first strategy returning `Some(nudge)` triggers a continuation.
#[async_trait]
pub trait ReasoningStrategy: Send + Sync {
    /// Human-readable name for logging/debugging.
    fn name(&self) -> &str;

    /// Inspect the agent's response and optionally produce a follow-up prompt.
    /// Returns `None` if the response is satisfactory (no intervention needed).
    /// Returns `Some(nudge)` if the strategy wants the agent to try again.
    async fn evaluate(&self, context: &StrategyContext) -> Option<String>;

    /// Maximum number of times this strategy can intervene per request.
    fn max_interventions(&self) -> u32;
}

/// Orchestrates reasoning strategies around `agent.chat()`.
pub struct ReasoningEngine {
    strategies: Vec<Box<dyn ReasoningStrategy>>,
    global_max_interventions: u32,
}

impl ReasoningEngine {
    pub fn new(global_max: u32) -> Self {
        Self {
            strategies: vec![],
            global_max_interventions: global_max,
        }
    }

    pub fn add_strategy(&mut self, s: impl ReasoningStrategy + 'static) {
        self.strategies.push(Box::new(s));
    }

    /// Drop-in replacement for `agent.chat()`. Applies all strategies post-response.
    ///
    /// The engine:
    /// 1. Calls `agent.chat(prompt, history)`
    /// 2. Builds `StrategyContext`
    /// 3. Iterates strategies in order; first one returning `Some(nudge)` triggers continuation
    /// 4. Extends history, calls `agent.chat(&nudge, extended_history)`
    /// 5. Repeats until no strategy intervenes or global max reached
    pub async fn chat(
        &self,
        agent: &MesoAgent,
        prompt: &str,
        history: Vec<Message>,
    ) -> Result<ChatResult> {
        let mut current_response = agent.chat(prompt, history.clone()).await?;
        let mut interventions_used = 0u32;
        let mut strategy_used: Option<String> = None;
        let mut current_history = history;

        loop {
            if interventions_used >= self.global_max_interventions {
                debug!(
                    "ReasoningEngine: global max interventions ({}) reached",
                    self.global_max_interventions
                );
                break;
            }

            let context = StrategyContext {
                original_prompt: prompt.to_string(),
                response: current_response.clone(),
                intervention_count: interventions_used,
                tool_calls_made: agent.tool_calls_made(),
            };

            // Find first strategy that wants to intervene
            let mut nudge = None;
            for strategy in &self.strategies {
                if interventions_used >= strategy.max_interventions() {
                    continue;
                }
                if let Some(n) = strategy.evaluate(&context).await {
                    info!(
                        "ReasoningEngine: strategy '{}' intervening (attempt {}/{})",
                        strategy.name(),
                        interventions_used + 1,
                        self.global_max_interventions
                    );
                    nudge = Some((strategy.name().to_string(), n));
                    break;
                }
            }

            let Some((name, nudge_prompt)) = nudge else {
                break;
            };

            // Extend history with the incomplete exchange and nudge
            current_history.push(Message::user(prompt));
            current_history.push(Message::assistant(current_response));

            interventions_used += 1;
            strategy_used = Some(name);

            current_response = agent.chat(&nudge_prompt, current_history.clone()).await?;
        }

        Ok(ChatResult {
            response: current_response,
            interventions_used,
            strategy_used,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    struct NoOpStrategy;

    #[async_trait]
    impl ReasoningStrategy for NoOpStrategy {
        fn name(&self) -> &str {
            "noop"
        }
        async fn evaluate(&self, _context: &StrategyContext) -> Option<String> {
            None
        }
        fn max_interventions(&self) -> u32 {
            3
        }
    }

    struct AlwaysNudgeStrategy {
        nudge_msg: String,
    }

    #[async_trait]
    impl ReasoningStrategy for AlwaysNudgeStrategy {
        fn name(&self) -> &str {
            "always_nudge"
        }
        async fn evaluate(&self, _context: &StrategyContext) -> Option<String> {
            Some(self.nudge_msg.clone())
        }
        fn max_interventions(&self) -> u32 {
            3
        }
    }

    // 8.11.10 — engine with no strategies returns response unchanged
    #[test]
    fn engine_no_strategies_passes_through() {
        let engine = ReasoningEngine::new(3);
        assert!(engine.strategies.is_empty());
        assert_eq!(engine.global_max_interventions, 3);
    }

    // 8.11.11 — engine with continuation detects incomplete
    #[tokio::test]
    async fn engine_strategy_evaluate_nudge() {
        let strategy = AlwaysNudgeStrategy {
            nudge_msg: "try again".into(),
        };
        let ctx = StrategyContext {
            original_prompt: "hello".into(),
            response: "I will search for files".into(),
            intervention_count: 0,
            tool_calls_made: 0,
        };
        let result = strategy.evaluate(&ctx).await;
        assert_eq!(result, Some("try again".into()));
    }

    // 8.11.12 — engine respects global max interventions
    #[test]
    fn engine_respects_global_max() {
        let engine = ReasoningEngine::new(5);
        assert_eq!(engine.global_max_interventions, 5);
    }

    // 8.11.13 — chat_result tracks strategy name
    #[test]
    fn chat_result_tracks_strategy_name() {
        let result = ChatResult {
            response: "done".into(),
            interventions_used: 1,
            strategy_used: Some("continuation".into()),
        };
        assert_eq!(result.strategy_used.as_deref(), Some("continuation"));
        assert_eq!(result.interventions_used, 1);
    }

    // TC-C4 — StrategyContext includes tool_calls_made
    #[test]
    fn tc_c4_strategy_context_includes_tool_calls_made() {
        let ctx = StrategyContext {
            original_prompt: "hello".into(),
            response: "done".into(),
            intervention_count: 0,
            tool_calls_made: 5,
        };
        assert_eq!(ctx.tool_calls_made, 5);
    }
}
