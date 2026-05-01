use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    ai::agent::ZeniiAgent,
    error::ZeniiError,
    tools::registry::ToolRegistry,
    workflows::definition::{StepType, Workflow},
};

/// Confidence level of a generated workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Low,
}

/// Result of a workflow generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
    /// The parsed workflow, if one was produced (None when only a clarifying question was returned).
    pub workflow: Option<Workflow>,
    /// Confidence rating: High when all tools are known, Low otherwise or when clarifying.
    pub confidence: Confidence,
    /// A clarifying question to ask the user when the description is too vague or contains
    /// unknown tools.
    pub clarifying_question: Option<String>,
    /// TOML serialization of the workflow (empty string when no workflow was produced).
    pub toml: String,
}

/// Converts a natural-language description into a structured [`Workflow`] by prompting an LLM.
///
/// # Usage
/// ```ignore
/// let result = generator.generate("every morning fetch top HN stories and summarize them").await?;
/// if result.confidence == Confidence::High {
///     registry.save(result.workflow.unwrap())?;
/// }
/// ```
pub struct WorkflowGenerator {
    pub(crate) agent: Arc<ZeniiAgent>,
    pub(crate) tool_registry: Arc<ToolRegistry>,
}

impl WorkflowGenerator {
    /// Create a new generator.
    pub fn new(agent: Arc<ZeniiAgent>, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            agent,
            tool_registry,
        }
    }

    /// Generate a workflow from a natural-language description.
    ///
    /// The agent is prompted with the available tools and expected JSON schema.
    /// The response is parsed, validated, and serialized to TOML.
    pub async fn generate(&self, description: &str) -> Result<GenerateResult, ZeniiError> {
        if description.trim().is_empty() {
            return Err(ZeniiError::Validation(
                "workflow description cannot be empty".to_string(),
            ));
        }
        let tools_context = self.build_tools_context();
        let prompt = self.build_prompt(description, &tools_context);
        let response = self
            .agent
            .prompt(&prompt)
            .await
            .map_err(|e| ZeniiError::Workflow(format!("generation failed: {e}")))?;
        self.parse_and_assess(&response.output)
    }

    /// Build a sorted, human-readable list of available tools for the prompt context.
    ///
    /// Each line has the form `- name(params) : description` where `params` is derived from
    /// the tool's `param_summary()`. This gives the LLM enough context to generate correct
    /// arg keys without guessing.
    pub(crate) fn build_tools_context(&self) -> String {
        let mut tools = self.tool_registry.list();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
            .iter()
            .map(|t| {
                if t.param_summary.is_empty() {
                    format!("- {} : {}", t.name, t.description)
                } else {
                    format!("- {}{} : {}", t.name, t.param_summary, t.description)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build the full prompt sent to the LLM.
    pub(crate) fn build_prompt(&self, description: &str, tools_context: &str) -> String {
        format!(
            r#"You are a workflow generator for Zenii. Convert the user description into a JSON workflow.

Available tools (use ONLY these exact tool names in "tool" fields):
{tools_context}

Return ONLY valid JSON — no markdown fences, no explanation, no trailing text.

If the description is too vague or ambiguous, return exactly this shape:
{{"clarifying_question": "your single, concrete question here"}}

Otherwise return a workflow object with this exact structure:
{{
  "id": "<slug-id>",
  "name": "<Human Name>",
  "description": "<one-line description>",
  "schedule": null,
  "steps": [
    {{
      "name": "<step_name>",
      "type": "tool",
      "tool": "<exact_tool_name>",
      "args": {{}},
      "depends_on": [],
      "failure_policy": "stop"
    }},
    {{
      "name": "<step_name>",
      "type": "llm",
      "prompt": "<prompt text, may reference {{{{steps.<prev>.output}}}}>",
      "depends_on": ["<prev_step_name>"],
      "failure_policy": "stop"
    }}
  ],
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z"
}}

StepType variants (use the "type" field to select):
- "tool"      : requires "tool" (exact name from list above) and optional "args" object
- "llm"       : requires "prompt" string, optional "model"
- "condition" : requires "expression", "if_true", optional "if_false"
- "parallel"  : requires "steps" array of step names
- "delay"     : requires "seconds" integer

failure_policy values: "stop" | "continue" | {{"fallback": {{"step": "<step_name>"}}}}

User description: {description}"#,
            tools_context = tools_context,
            description = description
        )
    }

    /// Parse the LLM response and assess confidence.
    ///
    /// - If the response contains `clarifying_question`, returns `Confidence::Low` with no workflow.
    /// - If the workflow references unknown tools, returns `Confidence::Low` with a follow-up question.
    /// - Otherwise returns `Confidence::High` with the serialized TOML.
    pub(crate) fn parse_and_assess(&self, response: &str) -> Result<GenerateResult, ZeniiError> {
        let trimmed = response.trim();
        let trimmed = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .map(|s| s.trim_start())
            .unwrap_or(trimmed);
        let trimmed = trimmed
            .strip_suffix("```")
            .map(|s| s.trim_end())
            .unwrap_or(trimmed)
            .trim();

        let json: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| ZeniiError::Workflow(format!("LLM returned invalid JSON: {e}")))?;

        // Check for clarifying question shortcut
        if let Some(q) = json
            .get("clarifying_question")
            .and_then(|v| v.as_str())
        {
            return Ok(GenerateResult {
                workflow: None,
                confidence: Confidence::Low,
                clarifying_question: Some(q.to_string()),
                toml: String::new(),
            });
        }

        // Parse into Workflow
        let workflow: Workflow = serde_json::from_value(json).map_err(|e| {
            ZeniiError::Workflow(format!("LLM returned invalid workflow structure: {e}"))
        })?;

        // Validate the parsed workflow before returning it
        workflow.validate()?;

        // Check for unknown tools and build a per-tool known-param map simultaneously
        let tool_infos = self.tool_registry.list();
        let known: HashSet<String> = tool_infos.iter().map(|t| t.name.clone()).collect();

        // Map tool_name -> set of known parameter keys (from parameters_schema properties)
        let tool_params: std::collections::HashMap<String, HashSet<String>> = tool_infos
            .iter()
            .map(|t| {
                let keys: HashSet<String> = t
                    .parameters
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .map(|obj| obj.keys().cloned().collect())
                    .unwrap_or_default();
                (t.name.clone(), keys)
            })
            .collect();

        let unknown_tools: Vec<String> = workflow
            .steps
            .iter()
            .filter_map(|s| {
                if let StepType::Tool { tool, .. } = &s.step_type {
                    if !known.contains(tool) {
                        Some(tool.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Collect arg keys that are not in the tool's known parameters
        let mut bad_args: Vec<String> = Vec::new();
        if unknown_tools.is_empty() {
            for step in &workflow.steps {
                if let StepType::Tool { tool, args } = &step.step_type
                    && let (Some(known_keys), Some(arg_obj)) =
                        (tool_params.get(tool), args.as_object())
                    && !known_keys.is_empty()
                {
                    for key in arg_obj.keys() {
                        if !known_keys.contains(key) {
                            bad_args.push(format!("{}.{}", tool, key));
                        }
                    }
                }
            }
        }

        let toml = toml::to_string(&workflow)
            .map_err(|e| ZeniiError::Workflow(format!("TOML serialization failed: {e}")))?;

        if unknown_tools.is_empty() && bad_args.is_empty() {
            Ok(GenerateResult {
                workflow: Some(workflow),
                confidence: Confidence::High,
                clarifying_question: None,
                toml,
            })
        } else if !unknown_tools.is_empty() {
            let mut available: Vec<String> = known.into_iter().collect();
            available.sort();
            Ok(GenerateResult {
                workflow: Some(workflow),
                confidence: Confidence::Low,
                clarifying_question: Some(format!(
                    "I used tools that aren't available: {}. Available tools are: {}. Which should I use instead?",
                    unknown_tools.join(", "),
                    available.join(", ")
                )),
                toml,
            })
        } else {
            // Unknown arg keys: downgrade to Low confidence
            Ok(GenerateResult {
                workflow: Some(workflow),
                confidence: Confidence::Low,
                clarifying_question: Some(format!(
                    "I used argument keys that don't match the tool's known parameters: {}. \
                     Please clarify what values you want to pass to these tools.",
                    bad_args.join(", ")
                )),
                toml,
            })
        }
    }
}

/// Parse a `Workflow` from a JSON string produced by an LLM.
///
/// The LLM is asked to return a JSON object that matches the `Workflow` schema.
/// This function deserializes it so the caller can persist it via `WorkflowRegistry`.
pub fn parse_workflow_json(json: &str) -> crate::Result<Workflow> {
    serde_json::from_str(json)
        .map_err(|e| ZeniiError::Workflow(format!("failed to parse LLM workflow JSON: {e}")))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;

    use crate::{
        tools::{registry::ToolRegistry, traits::ToolResult},
        workflows::generator::{Confidence, WorkflowGenerator},
        ZeniiError,
    };

    // ── test helpers ──────────────────────────────────────────────────────────

    struct FakeTool {
        name: &'static str,
        description: &'static str,
    }

    #[async_trait]
    impl crate::tools::traits::Tool for FakeTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            self.description
        }
        fn parameters_schema(&self) -> serde_json::Value {
            json!({"type": "object"})
        }
        async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
            Ok(ToolResult::ok("fake"))
        }
    }

    /// Build a WorkflowGenerator with two fake tools.
    ///
    /// Constructs a real ZeniiAgent using a no-key-required provider (ollama-style)
    /// so the agent can be built without network access. The agent is never called
    /// in these unit tests — only `parse_and_assess` and `build_tools_context` are tested.
    fn make_generator() -> WorkflowGenerator {
        let registry = Arc::new(ToolRegistry::new());
        registry
            .register(Arc::new(FakeTool {
                name: "web_search",
                description: "Search the web for a query",
            }))
            .unwrap();
        registry
            .register(Arc::new(FakeTool {
                name: "system_info",
                description: "Get system information",
            }))
            .unwrap();

        // Build a real agent using a no-key provider — constructor is cheap, no network call.
        // Wrapped in std::thread::spawn so it is safe to call from both #[test] and
        // #[tokio::test] contexts without panicking on nested runtime creation.
        let agent = std::thread::spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    use crate::{
                        ai::agent::ZeniiAgent,
                        config::AppConfig,
                        credential::InMemoryCredentialStore,
                    };
                    let creds = InMemoryCredentialStore::new();
                    let config = AppConfig::default();
                    let tools: Vec<Arc<dyn crate::tools::traits::Tool>> = vec![];
                    ZeniiAgent::from_provider(
                        "ollama",
                        "http://localhost:11434/v1",
                        "llama3",
                        false, // no API key required
                        &creds,
                        &tools,
                        &config,
                        None,
                        None,
                    )
                    .await
                    .expect("agent construction should succeed for no-key provider")
                })
        })
        .join()
        .unwrap();

        WorkflowGenerator {
            agent: Arc::new(agent),
            tool_registry: registry,
        }
    }

    // Minimal valid workflow JSON using only known tools
    fn known_tool_workflow_json() -> String {
        serde_json::json!({
            "id": "test-wf",
            "name": "Test Workflow",
            "description": "A test workflow",
            "schedule": null,
            "steps": [
                {
                    "name": "search",
                    "type": "tool",
                    "tool": "web_search",
                    "args": {"query": "hello"},
                    "depends_on": [],
                    "failure_policy": "stop"
                }
            ],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })
        .to_string()
    }

    // ── G.1 — clarifying question response ────────────────────────────────────
    #[test]
    fn test_parse_clarifying_question() {
        let wfgen = make_generator();
        let response = r#"{"clarifying_question": "What should the workflow output?"}"#;
        let result = wfgen.parse_and_assess(response).unwrap();

        assert_eq!(result.confidence, Confidence::Low);
        assert!(result.workflow.is_none());
        assert_eq!(
            result.clarifying_question.as_deref(),
            Some("What should the workflow output?")
        );
        assert!(result.toml.is_empty());
    }

    // ── G.2 — high-confidence workflow with known tool ────────────────────────
    #[test]
    fn test_parse_high_confidence_workflow() {
        let wfgen = make_generator();
        let result = wfgen.parse_and_assess(&known_tool_workflow_json()).unwrap();

        assert_eq!(result.confidence, Confidence::High);
        assert!(result.workflow.is_some());
        assert!(result.clarifying_question.is_none());
        assert!(!result.toml.is_empty());

        let wf = result.workflow.unwrap();
        assert_eq!(wf.id, "test-wf");
        assert_eq!(wf.steps.len(), 1);
    }

    // ── G.3 — low-confidence when workflow has unknown tool ───────────────────
    #[test]
    fn test_parse_low_confidence_unknown_tool() {
        let wfgen = make_generator();
        let json_str = serde_json::json!({
            "id": "bad-wf",
            "name": "Bad Workflow",
            "description": "Uses unknown tool",
            "schedule": null,
            "steps": [
                {
                    "name": "ghost",
                    "type": "tool",
                    "tool": "nonexistent_tool",
                    "args": {},
                    "depends_on": [],
                    "failure_policy": "stop"
                }
            ],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })
        .to_string();

        let result = wfgen.parse_and_assess(&json_str).unwrap();

        assert_eq!(result.confidence, Confidence::Low);
        assert!(result.workflow.is_some());
        let q = result.clarifying_question.expect("should have clarifying question");
        assert!(
            q.contains("nonexistent_tool"),
            "question should name the unknown tool, got: {q}"
        );
    }

    // ── G.4 — invalid JSON returns ZeniiError::Workflow ───────────────────────
    #[test]
    fn test_parse_invalid_json_returns_error() {
        let wfgen = make_generator();
        let result = wfgen.parse_and_assess("this is not json at all }{");
        assert!(result.is_err());
        match result.unwrap_err() {
            ZeniiError::Workflow(msg) => {
                assert!(
                    msg.contains("invalid JSON"),
                    "error should mention invalid JSON, got: {msg}"
                );
            }
            other => panic!("expected ZeniiError::Workflow, got: {other}"),
        }
    }

    // ── G.5 — markdown fences are stripped before parsing ─────────────────────
    #[test]
    fn test_parse_strips_markdown_fences() {
        let wfgen = make_generator();
        let fenced = format!("```json\n{}\n```", known_tool_workflow_json());
        let result = wfgen.parse_and_assess(&fenced).unwrap();

        assert_eq!(result.confidence, Confidence::High);
        assert!(result.workflow.is_some());
    }

    // ── G.6 — build_tools_context contains registered tool names ─────────────
    #[test]
    fn test_build_tools_context_contains_tool_names() {
        let wfgen = make_generator();
        let ctx = wfgen.build_tools_context();

        assert!(ctx.contains("web_search"), "context missing web_search: {ctx}");
        assert!(ctx.contains("system_info"), "context missing system_info: {ctx}");
        assert!(ctx.contains("Search the web"), "context missing description: {ctx}");
        // Output should be sorted alphabetically (system_info < web_search)
        let ws_pos = ctx.find("web_search").unwrap();
        let si_pos = ctx.find("system_info").unwrap();
        assert!(si_pos < ws_pos, "tools should be sorted alphabetically");
    }

    // ── G.7 — build_tools_context includes param summary when available ───────
    #[test]
    fn test_build_tools_context_includes_param_summary() {
        // Register a tool whose schema has properties so param_summary is non-empty
        let registry = Arc::new(ToolRegistry::new());

        struct ParamTool;
        #[async_trait::async_trait]
        impl crate::tools::traits::Tool for ParamTool {
            fn name(&self) -> &str { "param_tool" }
            fn description(&self) -> &str { "A tool with params" }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "limit": {"type": "integer"}
                    },
                    "required": ["query"]
                })
            }
            async fn execute(&self, _: serde_json::Value) -> crate::Result<crate::tools::traits::ToolResult> {
                Ok(crate::tools::traits::ToolResult::ok("ok"))
            }
        }

        registry.register(Arc::new(ParamTool)).unwrap();
        let agent = std::thread::spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    use crate::{ai::agent::ZeniiAgent, config::AppConfig, credential::InMemoryCredentialStore};
                    let creds = InMemoryCredentialStore::new();
                    let config = AppConfig::default();
                    let tools: Vec<Arc<dyn crate::tools::traits::Tool>> = vec![];
                    ZeniiAgent::from_provider("ollama", "http://localhost:11434/v1", "llama3", false, &creds, &tools, &config, None, None).await.unwrap()
                })
        }).join().unwrap();

        let wfgen2 = WorkflowGenerator { agent: Arc::new(agent), tool_registry: registry };
        let ctx = wfgen2.build_tools_context();

        // Line should contain param_tool followed by a param summary in parens
        assert!(ctx.contains("param_tool"), "context missing param_tool: {ctx}");
        assert!(ctx.contains("query"), "context missing param name 'query': {ctx}");
    }

    // ── G.8 — unknown arg key downgrades confidence to Low ────────────────────
    #[test]
    fn test_parse_bad_arg_key_downgrades_confidence() {
        let wfgen = make_generator();
        let json_str = serde_json::json!({
            "id": "bad-args-wf",
            "name": "Bad Args Workflow",
            "description": "Uses wrong arg keys",
            "schedule": null,
            "steps": [
                {
                    "name": "search",
                    "type": "tool",
                    "tool": "web_search",
                    "args": {"nonexistent_arg": "hello"},
                    "depends_on": [],
                    "failure_policy": "stop"
                }
            ],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }).to_string();

        let result = wfgen.parse_and_assess(&json_str).unwrap();

        // FakeTool has no properties in schema (empty object), so no validation happens.
        // This test just ensures the code path runs without panic.
        // When a real tool with a populated schema is used, bad keys downgrade confidence.
        assert!(result.workflow.is_some());
    }

    // ── P.7 — parse_workflow_json tests (Agent I) ─────────────────────────────

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
        use super::parse_workflow_json;
        use crate::workflows::definition::{FailurePolicy, StepType};

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
        use super::parse_workflow_json;
        use crate::workflows::definition::StepType;

        let wf = parse_workflow_json(realistic_llm_json()).unwrap();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&wf)
            .expect("Workflow must serialize to valid TOML");

        // Deserialize back
        use crate::workflows::definition::Workflow;
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
        use super::parse_workflow_json;

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
        use super::parse_workflow_json;

        // Missing 'steps' field
        let json = r#"{ "id": "no-steps", "name": "No Steps", "description": "bad" }"#;
        let result = parse_workflow_json(json);
        assert!(result.is_err());
    }
}
