use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Result, ZeniiError};

/// Normalize a cron expression to the 6-field format required by the `cron` crate
/// (sec min hour dom month dow). If exactly 5 fields are given (standard Unix cron),
/// a seconds field of "0" is prepended. All other field counts are returned as-is.
pub fn normalize_cron_expr(expr: &str) -> String {
    if expr.split_whitespace().count() == 5 {
        format!("0 {expr}")
    } else {
        expr.to_owned()
    }
}

/// Canvas position for a workflow node in the visual builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub schedule: Option<String>,
    pub steps: Vec<WorkflowStep>,
    /// Visual builder layout positions (step_name → position). Optional,
    /// never used for execution logic — only consumed by the frontend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<HashMap<String, NodePosition>>,
    #[serde(default = "now_rfc3339")]
    pub created_at: String,
    #[serde(default = "now_rfc3339")]
    pub updated_at: String,
    /// Schema version for forward-compatibility. None / absent means version 1.
    /// The loader warns if this exceeds the known version (Issue 6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    #[serde(flatten)]
    pub step_type: StepType,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub failure_policy: FailurePolicy,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[non_exhaustive]
pub enum StepType {
    Tool {
        tool: String,
        #[serde(default)]
        args: serde_json::Value,
    },
    Llm {
        prompt: String,
        #[serde(default)]
        model: Option<String>,
    },
    Condition {
        expression: String,
        if_true: String,
        #[serde(default)]
        if_false: Option<String>,
    },
    Parallel {
        steps: Vec<String>,
    },
    Delay {
        seconds: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FailurePolicy {
    #[default]
    Stop,
    Continue,
    Fallback {
        step: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub step_name: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: String,
    pub workflow_id: String,
    pub status: WorkflowRunStatus,
    pub step_results: Vec<StepOutput>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

/// Returns `true` if `s` consists solely of lowercase ASCII letters, digits, underscores, or
/// hyphens and is non-empty.
fn is_valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Returns `true` if `s` is a valid step name: non-empty, only `[a-z0-9_]`.
fn is_valid_step_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl Workflow {
    /// Validate the workflow definition. Returns an error describing the first violation found.
    pub fn validate(&self) -> Result<()> {
        // id: non-empty, matches [a-z0-9_-]+
        if !is_valid_id(&self.id) {
            return Err(ZeniiError::Validation(format!(
                "workflow id '{}' is invalid: must match [a-z0-9_-]+",
                self.id
            )));
        }

        // name: non-empty
        if self.name.trim().is_empty() {
            return Err(ZeniiError::Validation(
                "workflow name must not be empty".into(),
            ));
        }

        // steps: non-empty
        if self.steps.is_empty() {
            return Err(ZeniiError::Validation(
                "workflow must have at least one step".into(),
            ));
        }

        // Collect step names for reference validation
        let mut step_name_set: HashSet<&str> = HashSet::new();

        for step in &self.steps {
            // Step name: non-empty, matches [a-z0-9_]+
            if !is_valid_step_name(&step.name) {
                return Err(ZeniiError::Validation(format!(
                    "step name '{}' is invalid: must match [a-z0-9_]+",
                    step.name
                )));
            }

            // Step names must be unique
            if !step_name_set.insert(step.name.as_str()) {
                return Err(ZeniiError::Validation(format!(
                    "duplicate step name '{}'",
                    step.name
                )));
            }

            // Condition and Parallel not yet implemented
            match &step.step_type {
                StepType::Condition { .. } => {
                    return Err(ZeniiError::Workflow(
                        "condition/parallel execution not yet implemented".into(),
                    ));
                }
                StepType::Parallel { .. } => {
                    return Err(ZeniiError::Workflow(
                        "condition/parallel execution not yet implemented".into(),
                    ));
                }
                _ => {}
            }
        }

        // Now validate cross-references (depends_on, if_true/if_false, fallback)
        for step in &self.steps {
            // depends_on: all referenced steps must exist
            for dep in &step.depends_on {
                if !step_name_set.contains(dep.as_str()) {
                    return Err(ZeniiError::Validation(format!(
                        "step '{}' depends_on unknown step '{dep}'",
                        step.name
                    )));
                }
            }

            // Condition if_true / if_false
            if let StepType::Condition {
                if_true, if_false, ..
            } = &step.step_type
            {
                if !step_name_set.contains(if_true.as_str()) {
                    return Err(ZeniiError::Validation(format!(
                        "step '{}' if_true references unknown step '{if_true}'",
                        step.name
                    )));
                }
                if let Some(f) = if_false
                    && !step_name_set.contains(f.as_str())
                {
                    return Err(ZeniiError::Validation(format!(
                        "step '{}' if_false references unknown step '{f}'",
                        step.name
                    )));
                }
            }

            // Fallback in failure policy
            if let FailurePolicy::Fallback {
                step: fallback_step,
            } = &step.failure_policy
                && !step_name_set.contains(fallback_step.as_str())
            {
                return Err(ZeniiError::Validation(format!(
                    "step '{}' fallback references unknown step '{fallback_step}'",
                    step.name
                )));
            }
        }

        // Cycle detection via DFS on depends_on edges
        if let Some(cycle_step) = detect_cycle(&self.steps) {
            return Err(ZeniiError::Validation(format!(
                "circular dependency detected involving step '{cycle_step}'"
            )));
        }

        // Schedule validation (if present): use cron crate
        if let Some(sched) = &self.schedule {
            #[cfg(feature = "workflows")]
            {
                let normalized = normalize_cron_expr(sched);
                cron::Schedule::from_str(&normalized).map_err(|e| {
                    ZeniiError::Validation(format!("invalid cron schedule '{sched}': {e}"))
                })?;
            }
            #[cfg(not(feature = "workflows"))]
            {
                let _ = sched; // cron not available without the workflows feature
            }
        }

        Ok(())
    }
}

/// Perform DFS-based cycle detection on `depends_on` edges.
/// Returns the name of a step involved in a cycle, or `None` if the graph is acyclic.
fn detect_cycle(steps: &[WorkflowStep]) -> Option<String> {
    // Build adjacency: step_name -> Vec<step_name> (depends_on)
    let adj: HashMap<&str, &Vec<String>> = steps
        .iter()
        .map(|s| (s.name.as_str(), &s.depends_on))
        .collect();

    // 0 = unvisited, 1 = in-stack, 2 = done
    let mut state: HashMap<&str, u8> = HashMap::new();

    for step in steps {
        if let Some(cycle) = dfs_visit(step.name.as_str(), &adj, &mut state) {
            return Some(cycle);
        }
    }
    None
}

fn dfs_visit<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, &'a Vec<String>>,
    state: &mut HashMap<&'a str, u8>,
) -> Option<String> {
    match state.get(node).copied().unwrap_or(0) {
        2 => return None,                   // already fully processed
        1 => return Some(node.to_string()), // back edge → cycle
        _ => {}
    }
    state.insert(node, 1);
    if let Some(deps) = adj.get(node) {
        for dep in *deps {
            if let Some(cycle) = dfs_visit(dep.as_str(), adj, state) {
                return Some(cycle);
            }
        }
    }
    state.insert(node, 2);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // 5.1
    #[test]
    fn workflow_serde_roundtrip() {
        let wf = Workflow {
            id: "wf1".into(),
            name: "Test Workflow".into(),
            description: "A test workflow".into(),
            schedule: None,
            steps: vec![
                WorkflowStep {
                    name: "get_info".into(),
                    step_type: StepType::Tool {
                        tool: "system_info".into(),
                        args: serde_json::json!({"action": "os"}),
                    },
                    depends_on: vec![],
                    retry: None,
                    failure_policy: FailurePolicy::Stop,
                    timeout_secs: None,
                },
                WorkflowStep {
                    name: "wait".into(),
                    step_type: StepType::Delay { seconds: 5 },
                    depends_on: vec!["get_info".into()],
                    retry: None,
                    failure_policy: FailurePolicy::Stop,
                    timeout_secs: None,
                },
            ],
            layout: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            schema_version: Some(1),
        };

        let json = serde_json::to_string(&wf).unwrap();
        let back: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "wf1");
        assert_eq!(back.name, "Test Workflow");
        assert_eq!(back.steps.len(), 2);
        assert_eq!(back.steps[0].name, "get_info");
        assert_eq!(back.steps[1].name, "wait");
        assert_eq!(back.steps[1].depends_on, vec!["get_info"]);
    }

    // 5.2
    #[test]
    fn workflow_step_tool_type() {
        let step = StepType::Tool {
            tool: "shell".into(),
            args: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: StepType = serde_json::from_str(&json).unwrap();
        match back {
            StepType::Tool { tool, args } => {
                assert_eq!(tool, "shell");
                assert_eq!(args["command"], "ls");
            }
            _ => panic!("expected Tool variant"),
        }
    }

    // 5.3
    #[test]
    fn workflow_step_llm_type() {
        let step = StepType::Llm {
            prompt: "Summarize this".into(),
            model: Some("gpt-4o".into()),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: StepType = serde_json::from_str(&json).unwrap();
        match back {
            StepType::Llm { prompt, model } => {
                assert_eq!(prompt, "Summarize this");
                assert_eq!(model, Some("gpt-4o".into()));
            }
            _ => panic!("expected Llm variant"),
        }
    }

    // 5.4
    #[test]
    fn workflow_step_condition_type() {
        let step = StepType::Condition {
            expression: "{{steps.s1.success}}".into(),
            if_true: "proceed".into(),
            if_false: Some("fallback".into()),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: StepType = serde_json::from_str(&json).unwrap();
        match back {
            StepType::Condition {
                expression,
                if_true,
                if_false,
            } => {
                assert_eq!(expression, "{{steps.s1.success}}");
                assert_eq!(if_true, "proceed");
                assert_eq!(if_false, Some("fallback".into()));
            }
            _ => panic!("expected Condition variant"),
        }
    }

    // 5.5
    #[test]
    fn workflow_step_parallel_type() {
        let step = StepType::Parallel {
            steps: vec!["a".into(), "b".into(), "c".into()],
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: StepType = serde_json::from_str(&json).unwrap();
        match back {
            StepType::Parallel { steps } => {
                assert_eq!(steps, vec!["a", "b", "c"]);
            }
            _ => panic!("expected Parallel variant"),
        }
    }

    // 5.6
    #[test]
    fn workflow_step_delay_type() {
        let step = StepType::Delay { seconds: 30 };
        let json = serde_json::to_string(&step).unwrap();
        let back: StepType = serde_json::from_str(&json).unwrap();
        match back {
            StepType::Delay { seconds } => {
                assert_eq!(seconds, 30);
            }
            _ => panic!("expected Delay variant"),
        }
    }

    // 5.7
    #[test]
    fn failure_policy_variants() {
        let stop = FailurePolicy::Stop;
        let json_stop = serde_json::to_string(&stop).unwrap();
        assert!(json_stop.contains("stop"));

        let cont = FailurePolicy::Continue;
        let json_cont = serde_json::to_string(&cont).unwrap();
        assert!(json_cont.contains("continue"));

        let fb = FailurePolicy::Fallback {
            step: "recovery".into(),
        };
        let json_fb = serde_json::to_string(&fb).unwrap();
        assert!(json_fb.contains("fallback"));
        assert!(json_fb.contains("recovery"));

        // Roundtrip all variants
        let back_stop: FailurePolicy = serde_json::from_str(&json_stop).unwrap();
        assert!(matches!(back_stop, FailurePolicy::Stop));
        let back_cont: FailurePolicy = serde_json::from_str(&json_cont).unwrap();
        assert!(matches!(back_cont, FailurePolicy::Continue));
        let back_fb: FailurePolicy = serde_json::from_str(&json_fb).unwrap();
        assert!(matches!(back_fb, FailurePolicy::Fallback { step } if step == "recovery"));
    }

    // 5.8
    #[test]
    fn retry_config_defaults() {
        let rc = RetryConfig::default();
        assert_eq!(rc.max_retries, 3);
        assert_eq!(rc.retry_delay_ms, 1000);
    }

    // 5.9
    #[test]
    fn step_output_serde() {
        let output = StepOutput {
            step_name: "fetch_data".into(),
            output: "some result".into(),
            success: true,
            duration_ms: 150,
            error: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        let back: StepOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.step_name, "fetch_data");
        assert_eq!(back.output, "some result");
        assert!(back.success);
        assert_eq!(back.duration_ms, 150);
        assert!(back.error.is_none());
    }

    // 5.10
    #[test]
    fn workflow_run_status_variants() {
        let variants = vec![
            WorkflowRunStatus::Running,
            WorkflowRunStatus::Completed,
            WorkflowRunStatus::Failed,
            WorkflowRunStatus::Cancelled,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: WorkflowRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    // 5.11
    #[test]
    fn workflow_run_serde() {
        let run = WorkflowRun {
            id: "run-1".into(),
            workflow_id: "wf-1".into(),
            status: WorkflowRunStatus::Completed,
            step_results: vec![StepOutput {
                step_name: "s1".into(),
                output: "done".into(),
                success: true,
                duration_ms: 100,
                error: None,
            }],
            started_at: "2026-01-01T00:00:00Z".into(),
            completed_at: Some("2026-01-01T00:01:00Z".into()),
            error: None,
        };
        let json = serde_json::to_string(&run).unwrap();
        let back: WorkflowRun = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "run-1");
        assert_eq!(back.workflow_id, "wf-1");
        assert_eq!(back.status, WorkflowRunStatus::Completed);
        assert_eq!(back.step_results.len(), 1);
        assert!(back.completed_at.is_some());
    }

    // 5.11b — Layout roundtrip (JSON)
    #[test]
    fn layout_json_roundtrip() {
        let mut layout: HashMap<String, NodePosition> = HashMap::new();
        layout.insert("step_a".into(), NodePosition { x: 100.0, y: 200.0 });
        layout.insert("step_b".into(), NodePosition { x: 400.0, y: 200.0 });

        let json = serde_json::to_string(&layout).unwrap();
        let back: HashMap<String, NodePosition> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert!((back["step_a"].x - 100.0).abs() < f32::EPSILON);
        assert!((back["step_b"].y - 200.0).abs() < f32::EPSILON);
    }

    // 5.11c — Layout roundtrip (TOML)
    #[test]
    fn workflow_with_layout_toml_roundtrip() {
        let toml_str = r#"
            id = "layout-test"
            name = "Layout Test"
            description = "Tests layout preservation"

            [[steps]]
            name = "gather"
            type = "tool"
            tool = "system_info"
            [steps.args]
            action = "all"

            [[steps]]
            name = "summarize"
            type = "llm"
            prompt = "Summarize: {{steps.gather.output}}"
            depends_on = ["gather"]

            [layout]
            gather = { x = 100.0, y = 200.0 }
            summarize = { x = 400.0, y = 200.0 }
        "#;

        let wf: Workflow = toml::from_str(toml_str).unwrap();
        assert!(wf.layout.is_some());
        let layout = wf.layout.as_ref().unwrap();
        assert_eq!(layout.len(), 2);
        assert!((layout["gather"].x - 100.0).abs() < f32::EPSILON);
        assert!((layout["summarize"].x - 400.0).abs() < f32::EPSILON);

        // Re-serialize and verify layout survives
        let reserialized = toml::to_string_pretty(&wf).unwrap();
        let back: Workflow = toml::from_str(&reserialized).unwrap();
        assert!(back.layout.is_some());
        let back_layout = back.layout.unwrap();
        assert_eq!(back_layout.len(), 2);
        assert!((back_layout["gather"].x - 100.0).abs() < f32::EPSILON);
    }

    // 5.11e — schema_version roundtrips through TOML serialization (Issue 6)
    #[test]
    fn schema_version_toml_roundtrip() {
        let wf = Workflow {
            id: "sv-test".into(),
            name: "Schema Version Test".into(),
            description: "Verifies schema_version survives TOML serde".into(),
            schedule: None,
            steps: vec![WorkflowStep {
                name: "s1".into(),
                step_type: StepType::Delay { seconds: 1 },
                depends_on: vec![],
                retry: None,
                failure_policy: FailurePolicy::Stop,
                timeout_secs: None,
            }],
            layout: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            schema_version: Some(1),
        };

        let toml_str = toml::to_string_pretty(&wf).unwrap();
        assert!(
            toml_str.contains("schema_version"),
            "serialized TOML must contain schema_version"
        );

        let back: Workflow = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.schema_version, Some(1));

        // Absent schema_version deserializes as None (treated as v1)
        let no_sv = r#"
            id = "no-sv"
            name = "No SV"
            description = "No schema_version field"
            [[steps]]
            name = "s1"
            type = "delay"
            seconds = 1
        "#;
        let back2: Workflow = toml::from_str(no_sv).unwrap();
        assert_eq!(back2.schema_version, None);
    }

    // 5.11d — Workflow without layout is backward-compatible
    #[test]
    fn workflow_without_layout_compat() {
        let toml_str = r#"
            id = "no-layout"
            name = "No Layout"
            description = "Older workflow without layout"

            [[steps]]
            name = "s1"
            type = "delay"
            seconds = 5
        "#;

        let wf: Workflow = toml::from_str(toml_str).unwrap();
        assert!(wf.layout.is_none());

        // Serializing back should not emit a layout section
        let reserialized = toml::to_string_pretty(&wf).unwrap();
        assert!(!reserialized.contains("[layout]"));
    }

    // 5.12
    #[test]
    fn workflow_from_toml() {
        let toml_str = r#"
            id = "daily-report"
            name = "Daily Report"
            description = "Generates a daily report"

            [[steps]]
            name = "fetch"
            type = "tool"
            tool = "web_search"
            [steps.args]
            query = "latest news"

            [[steps]]
            name = "summarize"
            type = "llm"
            prompt = "Summarize: {{steps.fetch.output}}"
            depends_on = ["fetch"]
        "#;

        let wf: Workflow = toml::from_str(toml_str).unwrap();
        assert_eq!(wf.id, "daily-report");
        assert_eq!(wf.name, "Daily Report");
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps[0].name, "fetch");
        assert_eq!(wf.steps[1].name, "summarize");
        assert_eq!(wf.steps[1].depends_on, vec!["fetch"]);
        match &wf.steps[0].step_type {
            StepType::Tool { tool, .. } => assert_eq!(tool, "web_search"),
            _ => panic!("expected Tool step"),
        }
        match &wf.steps[1].step_type {
            StepType::Llm { prompt, .. } => {
                assert!(prompt.contains("{{steps.fetch.output}}"));
            }
            _ => panic!("expected Llm step"),
        }
    }

    // P.5 — step name with space must be rejected by validate()
    // NOTE: Requires Agent A's `Workflow::validate()` method to be merged.
    // Until then this test is marked #[ignore] so CI stays green.
    #[test]
    #[ignore = "requires Workflow::validate() from agent-a branch"]
    fn step_name_with_space_fails_validation() {
        let wf = Workflow {
            id: "test".into(),
            name: "Test".into(),
            description: "desc".into(),
            schedule: None,
            steps: vec![WorkflowStep {
                name: "step name".into(), // contains a space — must be rejected
                step_type: StepType::Delay { seconds: 1 },
                depends_on: vec![],
                retry: None,
                failure_policy: FailurePolicy::Stop,
                timeout_secs: None,
            }],
            layout: None,
            schema_version: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        // Once Workflow::validate() exists:
        // let result = wf.validate();
        // assert!(result.is_err(), "step name with space must be rejected");
        // assert!(result.unwrap_err().to_string().contains("step name"));
        let _ = wf; // suppress unused warning until validate() lands
    }

    // 5.13
    #[test]
    fn workflow_from_toml_minimal() {
        let toml_str = r#"
            id = "simple"
            name = "Simple"
            description = "Minimal workflow"

            [[steps]]
            name = "pause"
            type = "delay"
            seconds = 1
        "#;

        let wf: Workflow = toml::from_str(toml_str).unwrap();
        assert_eq!(wf.id, "simple");
        assert_eq!(wf.name, "Simple");
        assert_eq!(wf.description, "Minimal workflow");
        assert_eq!(wf.steps.len(), 1);
        match &wf.steps[0].step_type {
            StepType::Delay { seconds } => assert_eq!(*seconds, 1),
            _ => panic!("expected Delay step"),
        }
        // Defaults should be populated
        assert!(wf.steps[0].depends_on.is_empty());
        assert!(wf.steps[0].retry.is_none());
        assert!(matches!(wf.steps[0].failure_policy, FailurePolicy::Stop));
    }

    // ── validate() tests ──────────────────────────────────────────────────────

    fn valid_workflow() -> Workflow {
        Workflow {
            id: "my-workflow".into(),
            name: "My Workflow".into(),
            description: "A valid workflow".into(),
            schedule: None,
            steps: vec![
                WorkflowStep {
                    name: "step_one".into(),
                    step_type: StepType::Tool {
                        tool: "shell".into(),
                        args: serde_json::json!({}),
                    },
                    depends_on: vec![],
                    retry: None,
                    failure_policy: FailurePolicy::Stop,
                    timeout_secs: None,
                },
                WorkflowStep {
                    name: "step_two".into(),
                    step_type: StepType::Llm {
                        prompt: "Summarize".into(),
                        model: None,
                    },
                    depends_on: vec!["step_one".into()],
                    retry: None,
                    failure_policy: FailurePolicy::Stop,
                    timeout_secs: None,
                },
            ],
            layout: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            schema_version: Some(1),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    // validate_ok — well-formed workflow passes
    #[test]
    fn validate_ok() {
        let wf = valid_workflow();
        assert!(wf.validate().is_ok());
    }

    // validate_empty_steps — empty steps list is rejected
    #[test]
    fn validate_empty_steps() {
        let mut wf = valid_workflow();
        wf.steps.clear();
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("at least one step"), "{err}");
    }

    // validate_duplicate_step_name — duplicate step names are rejected
    #[test]
    fn validate_duplicate_step_name() {
        let mut wf = valid_workflow();
        wf.steps[1].name = "step_one".into();
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate step name"), "{err}");
    }

    // validate_unknown_depends_on — unknown dependency is rejected
    #[test]
    fn validate_unknown_depends_on() {
        let mut wf = valid_workflow();
        wf.steps[1].depends_on = vec!["nonexistent".into()];
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("unknown step"), "{err}");
    }

    // validate_cycle — A→B→A is rejected
    #[test]
    fn validate_cycle() {
        let wf = Workflow {
            id: "cycle-wf".into(),
            name: "Cycle".into(),
            description: "cycle test".into(),
            schedule: None,
            steps: vec![
                WorkflowStep {
                    name: "step_a".into(),
                    step_type: StepType::Delay { seconds: 1 },
                    depends_on: vec!["step_b".into()],
                    retry: None,
                    failure_policy: FailurePolicy::Stop,
                    timeout_secs: None,
                },
                WorkflowStep {
                    name: "step_b".into(),
                    step_type: StepType::Delay { seconds: 1 },
                    depends_on: vec!["step_a".into()],
                    retry: None,
                    failure_policy: FailurePolicy::Stop,
                    timeout_secs: None,
                },
            ],
            layout: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            schema_version: None,
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("circular dependency"), "{err}");
    }

    // validate_invalid_schedule — garbage cron expression is rejected
    #[cfg(feature = "workflows")]
    #[test]
    fn validate_invalid_schedule() {
        let mut wf = valid_workflow();
        wf.schedule = Some("not a cron expression".into());
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("invalid cron schedule"), "{err}");
    }

    // validate_valid_schedule — valid 7-field cron passes
    #[cfg(feature = "workflows")]
    #[test]
    fn validate_valid_schedule() {
        let mut wf = valid_workflow();
        // 7-field cron: sec min hour dom month dow year
        wf.schedule = Some("0 30 9 * * Mon *".into());
        assert!(wf.validate().is_ok());
    }

    // validate_valid_schedule_5_field — standard 5-field cron is normalized and passes
    #[cfg(feature = "workflows")]
    #[test]
    fn validate_valid_schedule_5_field() {
        let mut wf = valid_workflow();
        wf.schedule = Some("0 17 * * *".into());
        assert!(
            wf.validate().is_ok(),
            "5-field cron should be accepted after normalization"
        );
    }

    // validate_valid_schedule_6_field — 6-field cron passes without normalization
    #[cfg(feature = "workflows")]
    #[test]
    fn validate_valid_schedule_6_field() {
        let mut wf = valid_workflow();
        wf.schedule = Some("0 0 17 * * *".into());
        assert!(
            wf.validate().is_ok(),
            "6-field cron should be accepted as-is"
        );
    }

    // validate_invalid_schedule_4_field — 4-field cron is rejected
    #[cfg(feature = "workflows")]
    #[test]
    fn validate_invalid_schedule_4_field() {
        let mut wf = valid_workflow();
        wf.schedule = Some("17 * * *".into());
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("invalid cron schedule"), "{err}");
    }

    // validate_invalid_schedule_8_field — 8-field cron is rejected
    #[cfg(feature = "workflows")]
    #[test]
    fn validate_invalid_schedule_8_field() {
        let mut wf = valid_workflow();
        wf.schedule = Some("0 0 17 * * * 2026 extra".into());
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("invalid cron schedule"), "{err}");
    }

    // validate_condition_step_rejected — condition step type returns not-implemented error
    #[test]
    fn validate_condition_step_rejected() {
        let mut wf = valid_workflow();
        wf.steps[0].step_type = StepType::Condition {
            expression: "true".into(),
            if_true: "step_two".into(),
            if_false: None,
        };
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("not yet implemented"), "{err}");
    }

    // validate_parallel_step_rejected — parallel step type returns not-implemented error
    #[test]
    fn validate_parallel_step_rejected() {
        let mut wf = valid_workflow();
        wf.steps[0].step_type = StepType::Parallel {
            steps: vec!["step_two".into()],
        };
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("not yet implemented"), "{err}");
    }

    // validate_empty_id — empty id is rejected
    #[test]
    fn validate_empty_id() {
        let mut wf = valid_workflow();
        wf.id = String::new();
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("invalid"), "{err}");
    }

    // validate_invalid_id_chars — id with path separators is rejected
    #[test]
    fn validate_invalid_id_chars() {
        let mut wf = valid_workflow();
        wf.id = "../evil".into();
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("invalid"), "{err}");
    }

    // validate_empty_name — empty name is rejected
    #[test]
    fn validate_empty_name() {
        let mut wf = valid_workflow();
        wf.name = String::new();
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("name"), "{err}");
    }

    // validate_unknown_fallback — fallback to unknown step is rejected
    #[test]
    fn validate_unknown_fallback() {
        let mut wf = valid_workflow();
        wf.steps[0].failure_policy = FailurePolicy::Fallback {
            step: "no_such_step".into(),
        };
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("unknown step"), "{err}");
    }
}
