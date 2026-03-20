use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub schedule: Option<String>,
    pub steps: Vec<WorkflowStep>,
    #[serde(default = "now_rfc3339")]
    pub created_at: String,
    #[serde(default = "now_rfc3339")]
    pub updated_at: String,
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
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
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
}
