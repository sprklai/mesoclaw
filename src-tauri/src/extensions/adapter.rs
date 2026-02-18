//! `WasmToolAdapter` — bridges a compiled WASM module into the `ToolRegistry`.
//!
//! # How it works
//!
//! 1. At construction time, the caller provides a compiled [`WasmModule`] plus
//!    metadata (`name`, `description`, JSON Schema for args).
//! 2. The adapter implements [`Tool`] so it can be registered alongside native
//!    Rust tools without any change to the agent loop.
//! 3. On each `execute()` call the adapter:
//!    - Validates and extracts typed arguments from the JSON `Value`
//!    - Dispatches to the appropriate `WasmModule::call_*` method
//!    - Returns a [`ToolResult`]
//!
//! # Current limitations (spike scope)
//!
//! - Only `(i32, i32) -> i32` functions are supported directly.
//! - Full JSON marshal protocol (host ↔ guest via linear memory) is **Phase 2**.
//!   See `runtime.rs` for the guest-side protocol specification.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tools::{Tool, ToolResult};

use super::runtime::WasmModule;

// ─── WasmCallProtocol ────────────────────────────────────────────────────────

/// Describes the calling convention the host uses when invoking the WASM tool.
#[derive(Debug, Clone)]
pub enum WasmCallProtocol {
    /// Call `fn_name(a: i32, b: i32) -> i32`.
    /// Arguments are extracted from `args["a"]` and `args["b"]` (defaults to 0).
    BinaryI32 { fn_name: String },
    /// Call `fn_name() -> i32` (no input args).
    NullaryI32 { fn_name: String },
}

// ─── WasmToolAdapter ─────────────────────────────────────────────────────────

/// Wraps a compiled WASM module as a [`Tool`] that the [`ToolRegistry`] can
/// call without knowing it is backed by WASM.
pub struct WasmToolAdapter {
    name: String,
    description: String,
    schema: Value,
    module: Arc<WasmModule>,
    protocol: WasmCallProtocol,
}

impl WasmToolAdapter {
    /// Construct an adapter for a `(i32, i32) -> i32` WASM tool.
    pub fn binary_i32(
        name: impl Into<String>,
        description: impl Into<String>,
        module: Arc<WasmModule>,
        fn_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "a": { "type": "integer", "description": "First operand" },
                    "b": { "type": "integer", "description": "Second operand" }
                },
                "required": ["a", "b"]
            }),
            module,
            protocol: WasmCallProtocol::BinaryI32 {
                fn_name: fn_name.into(),
            },
        }
    }

    /// Construct an adapter for a `() -> i32` WASM tool.
    pub fn nullary_i32(
        name: impl Into<String>,
        description: impl Into<String>,
        module: Arc<WasmModule>,
        fn_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            schema: json!({ "type": "object", "properties": {} }),
            module,
            protocol: WasmCallProtocol::NullaryI32 {
                fn_name: fn_name.into(),
            },
        }
    }
}

#[async_trait]
impl Tool for WasmToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        match &self.protocol {
            WasmCallProtocol::BinaryI32 { fn_name } => {
                let a = args["a"].as_i64().unwrap_or(0) as i32;
                let b = args["b"].as_i64().unwrap_or(0) as i32;
                let result = self.module.call_binary_i32(fn_name, a, b)?;
                Ok(ToolResult::ok(result.to_string()))
            }
            WasmCallProtocol::NullaryI32 { fn_name } => {
                let result = self.module.call_nullary_i32(fn_name)?;
                Ok(ToolResult::ok(result.to_string()))
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::extensions::runtime::WasmRuntime;
    use crate::tools::Tool;

    /// Minimal WAT module that exports an `add(i32, i32) -> i32` function.
    /// This is the proof-of-concept tool for the spike.
    const ADD_WAT: &str = r#"
        (module
            (func $add (export "add") (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add
            )
        )
    "#;

    /// WAT module that exports a `magic() -> i32` constant.
    const MAGIC_WAT: &str = r#"
        (module
            (func $magic (export "magic") (result i32)
                i32.const 42
            )
        )
    "#;

    fn compile_add() -> Arc<WasmModule> {
        let runtime = WasmRuntime::new().expect("runtime init");
        let bytes = wat::parse_str(ADD_WAT).expect("WAT parse");
        Arc::new(runtime.compile(&bytes).expect("compile"))
    }

    fn compile_magic() -> Arc<WasmModule> {
        let runtime = WasmRuntime::new().expect("runtime init");
        let bytes = wat::parse_str(MAGIC_WAT).expect("WAT parse");
        Arc::new(runtime.compile(&bytes).expect("compile"))
    }

    // ── Adapter construction ─────────────────────────────────────────────────

    #[test]
    fn binary_i32_adapter_has_correct_name() {
        let module = compile_add();
        let adapter = WasmToolAdapter::binary_i32("wasm_add", "Add two numbers", module, "add");
        assert_eq!(adapter.name(), "wasm_add");
        assert_eq!(adapter.description(), "Add two numbers");
    }

    #[test]
    fn binary_i32_adapter_schema_has_a_and_b() {
        let module = compile_add();
        let adapter = WasmToolAdapter::binary_i32("wasm_add", "desc", module, "add");
        let schema = adapter.parameters_schema();
        assert!(schema["properties"]["a"].is_object());
        assert!(schema["properties"]["b"].is_object());
    }

    // ── Runtime execution ────────────────────────────────────────────────────

    #[tokio::test]
    async fn binary_i32_adapter_executes_correctly() {
        let module = compile_add();
        let adapter = WasmToolAdapter::binary_i32("wasm_add", "Add two numbers", module, "add");
        let result = adapter.execute(json!({"a": 3, "b": 4})).await.expect("execute");
        assert!(result.success);
        assert_eq!(result.output, "7");
    }

    #[tokio::test]
    async fn binary_i32_adapter_defaults_missing_args_to_zero() {
        let module = compile_add();
        let adapter = WasmToolAdapter::binary_i32("wasm_add", "Add", module, "add");
        let result = adapter.execute(json!({"a": 5})).await.expect("execute");
        // b defaults to 0, so result = 5
        assert_eq!(result.output, "5");
    }

    #[tokio::test]
    async fn binary_i32_adapter_handles_negative_numbers() {
        let module = compile_add();
        let adapter = WasmToolAdapter::binary_i32("wasm_add", "Add", module, "add");
        let result = adapter.execute(json!({"a": -10, "b": 4})).await.expect("execute");
        assert_eq!(result.output, "-6");
    }

    #[tokio::test]
    async fn nullary_i32_adapter_returns_constant() {
        let module = compile_magic();
        let adapter = WasmToolAdapter::nullary_i32("wasm_magic", "Returns 42", module, "magic");
        let result = adapter.execute(json!({})).await.expect("execute");
        assert!(result.success);
        assert_eq!(result.output, "42");
    }

    // ── Idempotency ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn adapter_can_be_called_multiple_times() {
        let module = compile_add();
        let adapter = WasmToolAdapter::binary_i32("wasm_add", "Add", module, "add");
        // Each call uses a fresh Store, so calls are independent.
        for i in 0..5_i64 {
            let result = adapter.execute(json!({"a": i, "b": 1})).await.expect("execute");
            assert_eq!(result.output, (i + 1).to_string());
        }
    }

    // ── Compile validation ──────────────────────────────────────────────────

    #[test]
    fn invalid_wasm_bytes_fail_to_compile() {
        let runtime = WasmRuntime::new().expect("runtime init");
        let result = runtime.compile(b"not-wasm");
        assert!(result.is_err(), "invalid bytes should fail compilation");
    }

    #[tokio::test]
    async fn missing_export_returns_error() {
        let module = compile_add();
        let adapter =
            WasmToolAdapter::binary_i32("wasm_add", "Add", module, "nonexistent_fn");
        let result = adapter.execute(json!({"a": 1, "b": 2})).await;
        assert!(result.is_err(), "missing export should return Err");
    }
}
