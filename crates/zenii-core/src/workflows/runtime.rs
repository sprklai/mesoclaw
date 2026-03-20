use std::collections::HashMap;

use crate::{Result, ZeniiError};

use super::definition::{StepOutput, StepType};
use super::templates;

/// Dispatch a single workflow step and return its output string.
pub async fn dispatch_step(
    step_type: &StepType,
    step_outputs: &HashMap<String, StepOutput>,
    tools: &crate::tools::ToolRegistry,
) -> Result<String> {
    match step_type {
        StepType::Tool { tool, args } => {
            // Resolve templates in args
            let args_str = serde_json::to_string(args)
                .map_err(|e| ZeniiError::Workflow(format!("args serialize error: {e}")))?;
            let resolved_args_str = templates::resolve(&args_str, step_outputs)?;
            let resolved_args: serde_json::Value =
                serde_json::from_str(&resolved_args_str).map_err(|e| {
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
        StepType::Llm { prompt, model: _ } => {
            // Resolve template in prompt
            let resolved_prompt = templates::resolve(prompt, step_outputs)?;
            // LLM execution requires an agent — return the resolved prompt for now
            // Full LLM step execution would use resolve_agent() similar to chat
            Ok(format!("[LLM step — prompt: {}]", resolved_prompt))
        }
        StepType::Condition {
            expression,
            if_true,
            if_false,
        } => {
            // Simple expression evaluation: check if a step output contains "true" or is non-empty
            let resolved = templates::resolve(expression, step_outputs)?;
            let is_true = !resolved.is_empty() && resolved != "false" && resolved != "0";
            if is_true {
                Ok(if_true.clone())
            } else {
                Ok(if_false.clone().unwrap_or_default())
            }
        }
        StepType::Parallel { steps } => {
            // Parallel is a meta-step — executor handles actual parallelism
            Ok(format!("parallel: [{}]", steps.join(", ")))
        }
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
    use crate::tools::system_info::SystemInfoTool;
    use crate::tools::ToolRegistry;

    // 5.38
    #[tokio::test]
    async fn dispatch_tool_step() {
        let tools = ToolRegistry::new();
        tools
            .register(Arc::new(SystemInfoTool::new()))
            .unwrap();
        let outputs = HashMap::new();

        let step = StepType::Tool {
            tool: "system_info".into(),
            args: serde_json::json!({"action": "os"}),
        };
        let result = dispatch_step(&step, &outputs, &tools).await.unwrap();
        assert!(!result.is_empty());
    }

    // 5.39
    #[tokio::test]
    async fn dispatch_delay_step() {
        let tools = ToolRegistry::new();
        let outputs = HashMap::new();

        let step = StepType::Delay { seconds: 0 };
        let result = dispatch_step(&step, &outputs, &tools).await.unwrap();
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
        let result = dispatch_step(&step, &outputs, &tools).await.unwrap();
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
        let result = dispatch_step(&step, &outputs, &tools).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
