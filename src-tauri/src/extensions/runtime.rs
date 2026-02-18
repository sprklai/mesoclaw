//! WASM runtime wrapper — compiles and caches WASM modules.
//!
//! # Guest protocol
//!
//! The spike uses a **direct typed-call** protocol for simplicity:
//! each WASM module exports a named function with a concrete Rust-compatible
//! signature (`(i32, i32) -> i32` for the demo).
//!
//! A production implementation would use the JSON marshal protocol instead:
//!
//! ```text
//! Guest exports:
//!   alloc(size: u32) -> u32          — allocate a buffer in guest memory
//!   dealloc(ptr: u32, size: u32)     — release a previously allocated buffer
//!   execute(ptr: u32, len: u32) -> u64  — packed (result_ptr << 32 | result_len)
//!   memory                           — the shared linear memory export
//! ```
//!
//! This makes it possible to pass arbitrary JSON from host to guest and back
//! without any unsafe code on the Rust side.

use std::sync::Arc;

use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

// ─── WasmRuntime ─────────────────────────────────────────────────────────────

/// Shared WASM engine.  One instance per application; thread-safe.
pub struct WasmRuntime {
    engine: Arc<Engine>,
}

impl WasmRuntime {
    /// Create a new runtime with the default engine configuration.
    pub fn new() -> Result<Self, String> {
        let engine = Engine::default();
        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    /// Compile WASM bytes into a [`WasmModule`] ready for instantiation.
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<WasmModule, String> {
        let module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| format!("WASM compile error: {e}"))?;
        Ok(WasmModule {
            engine: Arc::clone(&self.engine),
            module,
        })
    }
}

// ─── WasmModule ──────────────────────────────────────────────────────────────

/// A compiled WASM module ready to be instantiated and called.
///
/// Each call to `call_*` creates a fresh `Store` (isolated state) so that
/// modules cannot share mutable state between invocations.
pub struct WasmModule {
    pub(crate) engine: Arc<Engine>,
    pub(crate) module: Module,
}

impl WasmModule {
    /// Instantiate the module and look up a typed export function.
    fn instantiate<Params, Results>(
        &self,
        store: &mut Store<()>,
        fn_name: &str,
    ) -> Result<TypedFunc<Params, Results>, String>
    where
        Params: wasmtime::WasmParams,
        Results: wasmtime::WasmResults,
    {
        let instance = Instance::new(&mut *store, &self.module, &[])
            .map_err(|e| format!("WASM instantiate error: {e}"))?;
        instance
            .get_typed_func::<Params, Results>(store, fn_name)
            .map_err(|e| format!("export `{fn_name}` not found: {e}"))
    }

    /// Call an exported `(i32, i32) -> i32` function.
    ///
    /// Used by the demo add-tool and any WASM module exporting a simple
    /// binary integer operation.
    pub fn call_binary_i32(
        &self,
        fn_name: &str,
        a: i32,
        b: i32,
    ) -> Result<i32, String> {
        let mut store = Store::new(&self.engine, ());
        let func = self.instantiate::<(i32, i32), i32>(&mut store, fn_name)?;
        func.call(&mut store, (a, b))
            .map_err(|e| format!("WASM call error: {e}"))
    }

    /// Call an exported `() -> i32` function.
    pub fn call_nullary_i32(&self, fn_name: &str) -> Result<i32, String> {
        let mut store = Store::new(&self.engine, ());
        let func = self.instantiate::<(), i32>(&mut store, fn_name)?;
        func.call(&mut store, ())
            .map_err(|e| format!("WASM call error: {e}"))
    }
}
