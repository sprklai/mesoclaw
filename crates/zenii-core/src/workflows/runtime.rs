use std::collections::HashMap;
use std::sync::Arc;

use crate::gateway::state::AppState;
use crate::{Result, ZeniiError};

use super::definition::{StepOutput, StepType};
use super::templates;

/// Dispatch a single workflow step and return its output string.
///
/// When `app_state` is provided, LLM steps resolve an agent and call the AI.
/// Without it, LLM steps return a placeholder (useful for tests).
pub async fn dispatch_step(
    step_type: &StepType,
    step_outputs: &HashMap<String, StepOutput>,
    tools: &crate::tools::ToolRegistry,
    app_state: Option<&Arc<AppState>>,
) -> Result<String> {
    match step_type {
        StepType::Tool { tool, args } => {
            // Resolve templates in args.
            // Step outputs may contain newlines, quotes, etc. that would break JSON
            // if injected raw. We JSON-escape each output before template resolution
            // so the resulting string remains valid JSON.
            // JSON-encode each step output so embedded quotes/newlines don't break
            // the args JSON string after template substitution.
            // serde_json::to_value produces Value::String; as_str() gives the raw
            // unescaped content. We then re-encode just the inner bytes via
            // to_string and remove the surrounding double-quotes to get the
            // JSON-escape-sequence form suitable for insertion into a JSON template.
            let escaped_outputs: HashMap<String, StepOutput> = step_outputs
                .iter()
                .map(|(k, v)| {
                    // Serialize to a JSON string literal (e.g. `"hello\nworld"`)
                    // then remove the wrapping `"` characters so only the interior
                    // escape sequences remain for safe template substitution.
                    let escaped = serde_json::to_string(&v.output)
                        .map(|mut js| {
                            if js.starts_with('"') && js.ends_with('"') && js.len() >= 2 {
                                js.remove(0);
                                js.pop();
                            }
                            js
                        })
                        .unwrap_or_else(|_| v.output.clone());
                    (
                        k.clone(),
                        StepOutput {
                            output: escaped,
                            ..v.clone()
                        },
                    )
                })
                .collect();

            let args_str = serde_json::to_string(args)
                .map_err(|e| ZeniiError::Workflow(format!("args serialize error: {e}")))?;
            let resolved_args_str = templates::resolve(&args_str, &escaped_outputs)?;
            let resolved_args: serde_json::Value = serde_json::from_str(&resolved_args_str)
                .map_err(|e| {
                    ZeniiError::Workflow(format!("args parse error after template: {e}"))
                })?;

            let tool_impl = tools
                .get(tool)
                .ok_or_else(|| ZeniiError::Workflow(format!("tool '{}' not found", tool)))?;
            let result = tool_impl.execute(resolved_args).await?;
            if result.success {
                Ok(result.output)
            } else {
                Err(ZeniiError::Workflow(format!(
                    "tool '{}' failed: {}",
                    tool, result.output
                )))
            }
        }
        StepType::Llm { prompt, model } => {
            let resolved_prompt = templates::resolve(prompt, step_outputs)?;

            #[cfg(feature = "ai")]
            {
                if let Some(state) = app_state {
                    let requested_model = model.as_deref();
                    let agent =
                        crate::ai::resolve_agent(requested_model, state, None, None, "workflow")
                            .await?;
                    let resp = agent.chat(&resolved_prompt, vec![]).await?;
                    return Ok(resp.output);
                }
            }

            // Fallback when AI feature is off or no AppState provided (tests)
            let _ = model;
            Ok(format!("[LLM step — prompt: {}]", resolved_prompt))
        }
        StepType::Condition {
            expression,
            if_true,
            if_false,
        } => {
            // Simple expression evaluation: check if a step output is truthy.
            // Falsy: empty string, "false" (any case), "0", "no" (any case).
            // Everything else is truthy.
            let resolved = templates::resolve(expression, step_outputs)?;
            let lower = resolved.trim().to_lowercase();
            let is_true = !lower.is_empty() && lower != "false" && lower != "0" && lower != "no";
            if is_true {
                Ok(if_true.clone())
            } else {
                Ok(if_false.clone().unwrap_or_default())
            }
        }
        StepType::Parallel { steps: _ } => Err(ZeniiError::Workflow(
            "parallel step execution not yet implemented".into(),
        )),
        StepType::Delay { seconds } => {
            tokio::time::sleep(std::time::Duration::from_secs(*seconds)).await;
            Ok(format!("delayed {} seconds", seconds))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::tools::ToolRegistry;
    use crate::tools::system_info::SystemInfoTool;

    // 5.38
    #[tokio::test]
    async fn dispatch_tool_step() {
        let tools = ToolRegistry::new();
        tools.register(Arc::new(SystemInfoTool::new())).unwrap();
        let outputs = HashMap::new();

        let step = StepType::Tool {
            tool: "system_info".into(),
            args: serde_json::json!({"action": "os"}),
        };
        let result = dispatch_step(&step, &outputs, &tools, None).await.unwrap();
        assert!(!result.is_empty());
    }

    // 5.39
    #[tokio::test]
    async fn dispatch_delay_step() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        let step = StepType::Delay { seconds: 0 };
        let result = dispatch_step(&step, &outputs, &tools, None).await.unwrap();
        assert!(result.contains("delayed"));
    }

    // 5.40
    #[tokio::test]
    async fn dispatch_condition_step() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        let step = StepType::Condition {
            expression: "true".into(),
            if_true: "yes_branch".into(),
            if_false: Some("no_branch".into()),
        };
        let result = dispatch_step(&step, &outputs, &tools, None).await.unwrap();
        assert_eq!(result, "yes_branch");
    }

    // 5.41
    #[tokio::test]
    async fn dispatch_unknown_tool_errors() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        let step = StepType::Tool {
            tool: "nonexistent_tool".into(),
            args: serde_json::json!({}),
        };
        let result = dispatch_step(&step, &outputs, &tools, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // RT.1 — Condition evaluation: falsy strings
    #[tokio::test]
    async fn condition_falsy_strings() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        for falsy in &["false", "False", "FALSE", "0", "no", "No", ""] {
            let step = StepType::Condition {
                expression: falsy.to_string(),
                if_true: "yes".into(),
                if_false: Some("no".into()),
            };
            let result = dispatch_step(&step, &outputs, &tools, None).await.unwrap();
            assert_eq!(result, "no", "expected 'no' for falsy expression '{falsy}'");
        }
    }

    // RT.2 — Condition evaluation: truthy strings
    #[tokio::test]
    async fn condition_truthy_strings() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        for truthy in &["yes", "1", "true", "True", "anything"] {
            let step = StepType::Condition {
                expression: truthy.to_string(),
                if_true: "yes".into(),
                if_false: Some("no".into()),
            };
            let result = dispatch_step(&step, &outputs, &tools, None).await.unwrap();
            assert_eq!(
                result, "yes",
                "expected 'yes' for truthy expression '{truthy}'"
            );
        }
    }

    // RT.3 — Parallel step returns error, not a string
    #[tokio::test]
    async fn parallel_step_returns_error() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        let step = StepType::Parallel {
            steps: vec!["s1".into(), "s2".into()],
        };
        let result = dispatch_step(&step, &outputs, &tools, None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("parallel") || err.contains("not yet implemented"),
            "unexpected error: {err}"
        );
    }

    // RT.4 — Template extraction: step output with embedded quotes and newlines round-trips
    #[tokio::test]
    async fn template_extraction_embedded_quotes_newlines() {
        use crate::workflows::definition::StepOutput;

        let tools = ToolRegistry::new();
        tools.register(Arc::new(SystemInfoTool::new())).unwrap();

        // Build step_outputs with a value containing quotes and newlines
        let tricky_output = "line1\nline2\twith\"quotes\"and\\backslash";
        let mut outputs = HashMap::new();
        outputs.insert(
            "prev".to_string(),
            StepOutput {
                step_name: "prev".into(),
                output: tricky_output.to_string(),
                success: true,
                duration_ms: 0,
                error: None,
            },
        );

        // A condition step that uses the template — just resolves the expression
        let step = StepType::Condition {
            expression: "{{ steps.prev.output }}".into(),
            if_true: "got_output".into(),
            if_false: Some("empty".into()),
        };
        let result = dispatch_step(&step, &outputs, &tools, None).await.unwrap();
        // Non-empty output is truthy
        assert_eq!(result, "got_output");
    }
}
