use std::collections::HashMap;

use crate::{Result, ZeniiError};

use super::definition::StepOutput;

/// Resolve template placeholders in a string using completed step outputs.
/// Supports: `{{steps.step_name.output}}`, `{{steps.step_name.success}}`, `{{steps.step_name.error}}`
pub fn resolve(template: &str, step_outputs: &HashMap<String, StepOutput>) -> Result<String> {
    let mut env = minijinja::Environment::new();

    // Build a "steps" context object
    let mut steps: HashMap<String, minijinja::Value> = HashMap::new();
    for (name, output) in step_outputs {
        let mut step_map = std::collections::BTreeMap::new();
        step_map.insert(
            "output".to_string(),
            minijinja::Value::from(output.output.clone()),
        );
        step_map.insert(
            "success".to_string(),
            minijinja::Value::from(output.success),
        );
        step_map.insert(
            "error".to_string(),
            minijinja::Value::from(output.error.clone().unwrap_or_default()),
        );
        steps.insert(name.clone(), minijinja::Value::from(step_map));
    }

    env.add_template("__inline", template)
        .map_err(|e| ZeniiError::Workflow(format!("template parse error: {e}")))?;

    let tmpl = env
        .get_template("__inline")
        .map_err(|e| ZeniiError::Workflow(format!("template not found: {e}")))?;

    let ctx = minijinja::context! { steps => steps };
    tmpl.render(ctx)
        .map_err(|e| ZeniiError::Workflow(format!("template render error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_output(name: &str, output: &str, success: bool) -> (String, StepOutput) {
        (
            name.to_string(),
            StepOutput {
                step_name: name.to_string(),
                output: output.to_string(),
                success,
                duration_ms: 0,
                error: None,
            },
        )
    }

    // 5.14
    #[test]
    fn resolve_simple_template() {
        let mut outputs = HashMap::new();
        outputs.insert("step1".to_string(), {
            let (_, o) = make_output("step1", "hello", true);
            o
        });
        let result = resolve("Result: {{ steps.step1.output }}", &outputs).unwrap();
        assert_eq!(result, "Result: hello");
    }

    // 5.15
    #[test]
    fn resolve_missing_step() {
        let outputs = HashMap::new();
        // Accessing a field on a missing step returns a template render error
        let result = resolve("{{ steps.missing.output }}", &outputs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("template render error"));
    }

    // 5.16
    #[test]
    fn resolve_no_templates() {
        let outputs = HashMap::new();
        let result = resolve("no placeholders", &outputs).unwrap();
        assert_eq!(result, "no placeholders");
    }

    // 5.17
    #[test]
    fn resolve_multiple_templates() {
        let mut outputs = HashMap::new();
        let (k1, o1) = make_output("s1", "alpha", true);
        let (k2, o2) = make_output("s2", "beta", true);
        outputs.insert(k1, o1);
        outputs.insert(k2, o2);
        let result =
            resolve("A: {{ steps.s1.output }}, B: {{ steps.s2.output }}", &outputs).unwrap();
        assert_eq!(result, "A: alpha, B: beta");
    }

    // 5.18
    #[test]
    fn resolve_nested_access() {
        let mut outputs = HashMap::new();
        let (k, o) = make_output("s1", "data", true);
        outputs.insert(k, o);
        let result = resolve("Success: {{ steps.s1.success }}", &outputs).unwrap();
        assert_eq!(result, "Success: true");
    }
}
