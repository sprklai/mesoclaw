use crate::config::AppConfig;

/// Translates hint prefix strings into concrete `provider_id:model_id` pairs.
///
/// Hint strings like `"hint:reasoning"` are resolved to configured model targets.
/// Concrete model strings (e.g. `"openai:gpt-4o"`) pass through unchanged.
/// Unknown hint prefixes with no configured target return `None`, falling through
/// to the normal resolution chain in `resolve_agent`.
pub struct ModelRouter<'a> {
    config: &'a AppConfig,
}

impl<'a> ModelRouter<'a> {
    pub fn new(config: &'a AppConfig) -> Self {
        Self { config }
    }

    /// Translate a hint prefix to a concrete "provider_id:model_id" string,
    /// or pass through an existing concrete model string unchanged.
    /// Returns `None` if input is `None` OR if the hint has no configured target
    /// (caller falls through to normal resolution chain).
    pub fn route(&self, requested: Option<&str>) -> Option<String> {
        let req = requested?;
        match req {
            "hint:reasoning" => self.config.routing_hint_reasoning.clone(),
            "hint:fast" => self.config.routing_hint_fast.clone(),
            "hint:vision" => self.config.routing_hint_vision.clone(),
            "hint:summarize" => self.config.routing_hint_summarize.clone(),
            other => Some(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config_with_routing() -> AppConfig {
        let mut c = AppConfig::default();
        c.routing_hint_reasoning = Some("anthropic:claude-opus-4-7".to_string());
        c.routing_hint_fast = Some("openai:gpt-4o-mini".to_string());
        c
    }

    // 1. hint_reasoning_resolves_to_configured_model
    #[test]
    fn hint_reasoning_resolves_to_configured_model() {
        let config = make_config_with_routing();
        let router = ModelRouter::new(&config);
        assert_eq!(
            router.route(Some("hint:reasoning")),
            Some("anthropic:claude-opus-4-7".to_string())
        );
    }

    // 2. hint_fast_resolves_to_configured_model
    #[test]
    fn hint_fast_resolves_to_configured_model() {
        let config = make_config_with_routing();
        let router = ModelRouter::new(&config);
        assert_eq!(
            router.route(Some("hint:fast")),
            Some("openai:gpt-4o-mini".to_string())
        );
    }

    // 3. unconfigured_hint_returns_none (hint:vision when routing_hint_vision is None)
    #[test]
    fn unconfigured_hint_returns_none() {
        let config = make_config_with_routing();
        // routing_hint_vision is None (not configured)
        let router = ModelRouter::new(&config);
        assert_eq!(router.route(Some("hint:vision")), None);
    }

    // 4. concrete_model_string_passes_through_unchanged
    #[test]
    fn concrete_model_string_passes_through_unchanged() {
        let config = make_config_with_routing();
        let router = ModelRouter::new(&config);
        assert_eq!(
            router.route(Some("openai:gpt-4o")),
            Some("openai:gpt-4o".to_string())
        );
    }

    // 5. none_input_returns_none
    #[test]
    fn none_input_returns_none() {
        let config = make_config_with_routing();
        let router = ModelRouter::new(&config);
        assert_eq!(router.route(None), None);
    }

    // 6. unknown_prefix_passes_through_unchanged
    #[test]
    fn unknown_prefix_passes_through_unchanged() {
        let config = make_config_with_routing();
        let router = ModelRouter::new(&config);
        assert_eq!(
            router.route(Some("custom:my-model")),
            Some("custom:my-model".to_string())
        );
    }
}
