/// Workflow generator helpers: parse LLM-produced JSON into a `Workflow` struct,
/// then validate that it can be serialized back to TOML and deserialized again.
///
/// The NL workflow creation flow works as follows:
/// 1. User describes the workflow in plain language in chat (Workflow mode).
/// 2. The gateway chat handler detects the workflow intent and asks the LLM
///    to produce a JSON block containing a `Workflow`.
/// 3. The LLM response JSON is extracted and parsed here before being saved
///    to the `WorkflowRegistry`.
///
/// This module owns the parsing function so it can be unit-tested in isolation
/// without a live LLM.
use crate::{Result, ZeniiError};

use super::definition::Workflow;

/// Parse a `Workflow` from a JSON string produced by an LLM.
///
/// The LLM is asked to return a JSON object that matches the `Workflow` schema.
/// This function deserializes it so the caller can persist it via `WorkflowRegistry`.
pub fn parse_workflow_json(json: &str) -> Result<Workflow> {
    serde_json::from_str(json)
        .map_err(|e| ZeniiError::Workflow(format!("failed to parse LLM workflow JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::definition::{FailurePolicy, StepType};

    /// Helper: a minimal but realistic JSON workflow as an LLM might produce it.
    fn realistic_llm_json() -> &'static str {
        r#"{
            "id": "daily-summary",
            "name": "Daily Summary",
            "description": "Fetch news and summarize with LLM",
            "steps": [
                {
                    "name": "fetch",
                    "type": "tool",
                    "tool": "web_search",
                    "args": { "query": "latest AI news", "max_results": 5 }
                },
                {
                    "name": "summarize",
                    "type": "llm",
                    "prompt": "Summarize these results: {{steps.fetch.output}}",
                    "depends_on": ["fetch"]
                }
            ],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#
    }

    // P.7a — parse_workflow_json succeeds on realistic LLM output
    #[test]
    fn parse_workflow_json_succeeds() {
        let wf = parse_workflow_json(realistic_llm_json()).unwrap();
        assert_eq!(wf.id, "daily-summary");
        assert_eq!(wf.name, "Daily Summary");
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps[0].name, "fetch");
        assert_eq!(wf.steps[1].name, "summarize");
        assert_eq!(wf.steps[1].depends_on, vec!["fetch"]);
        match &wf.steps[0].step_type {
            StepType::Tool { tool, args } => {
                assert_eq!(tool, "web_search");
                assert_eq!(args["max_results"], 5);
            }
            _ => panic!("expected Tool step"),
        }
        match &wf.steps[1].step_type {
            StepType::Llm { prompt, .. } => {
                assert!(prompt.contains("{{steps.fetch.output}}"));
            }
            _ => panic!("expected Llm step"),
        }
        // Defaults populated
        assert!(matches!(wf.steps[0].failure_policy, FailurePolicy::Stop));
        assert!(wf.steps[0].retry.is_none());
    }

    // P.7b — generator_output_is_valid_toml: parsed workflow round-trips through TOML
    #[test]
    fn generator_output_is_valid_toml() {
        let wf = parse_workflow_json(realistic_llm_json()).unwrap();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&wf)
            .expect("Workflow must serialize to valid TOML");

        // Deserialize back
        let restored: Workflow = toml::from_str(&toml_str)
            .expect("TOML produced by the generator must parse back to Workflow");

        // Fields must be identical
        assert_eq!(restored.id, wf.id);
        assert_eq!(restored.name, wf.name);
        assert_eq!(restored.description, wf.description);
        assert_eq!(restored.steps.len(), wf.steps.len());
        assert_eq!(restored.steps[0].name, wf.steps[0].name);
        assert_eq!(restored.steps[1].name, wf.steps[1].name);
        assert_eq!(restored.steps[1].depends_on, wf.steps[1].depends_on);

        // Step types preserved
        match (&restored.steps[0].step_type, &wf.steps[0].step_type) {
            (StepType::Tool { tool: t1, .. }, StepType::Tool { tool: t2, .. }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("step type changed after TOML round-trip"),
        }
    }

    // P.7c — parse_workflow_json returns error on invalid JSON
    #[test]
    fn parse_workflow_json_invalid_returns_error() {
        let result = parse_workflow_json("{ not valid json }");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed to parse LLM workflow JSON"),
            "error message was: {err}"
        );
    }

    // P.7d — parse_workflow_json returns error when required fields missing
    #[test]
    fn parse_workflow_json_missing_fields_returns_error() {
        // Missing 'steps' field
        let json = r#"{ "id": "no-steps", "name": "No Steps", "description": "bad" }"#;
        let result = parse_workflow_json(json);
        assert!(result.is_err());
    }
}
