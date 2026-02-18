//! WASM Extension System — spike implementation (Phase 6.8).
//!
//! Enables user-authored tools compiled to WebAssembly to run inside the
//! MesoClaw agent loop with the same interface as native Rust tools.
//!
//! # Feature flag
//!
//! This entire module requires the `wasm-ext` Cargo feature:
//!
//! ```sh
//! cargo build --features wasm-ext
//! cargo test  --features wasm-ext -- extensions
//! ```
//!
//! It is **off by default** and adds approximately 15 MB to the binary.
//!
//! # Architecture
//!
//! ```text
//!  ┌─────────────────────────┐
//!  │    Agent Loop           │
//!  │  ToolRegistry::call()   │
//!  └────────────┬────────────┘
//!               │  Arc<dyn Tool>
//!  ┌────────────▼────────────┐
//!  │  WasmToolAdapter        │  ← implements Tool; registered like any native tool
//!  │  (adapter.rs)           │
//!  └────────────┬────────────┘
//!               │  WasmModule::call_*()
//!  ┌────────────▼────────────┐
//!  │  WasmModule             │  ← compiled module, fresh Store per call
//!  │  (runtime.rs)           │
//!  └────────────┬────────────┘
//!               │  wasmtime Engine + Module + Instance
//!  ┌────────────▼────────────┐
//!  │  WASM sandbox           │  ← no syscalls, no memory outside module
//!  │  (.wasm binary)         │
//!  └─────────────────────────┘
//! ```
//!
//! # Go / No-Go criteria
//!
//! | Criterion            | Result      | Notes                                         |
//! |----------------------|-------------|-----------------------------------------------|
//! | Binary size impact   | ⚠ High      | `wasmtime` adds ~15 MB; consider `wasm3` (~300 KB) |
//! | Startup cost         | ✅ Low      | JIT compile at load time (~50 ms for small modules) |
//! | Sandbox guarantees   | ✅ Strong   | WASM isolates memory; no arbitrary syscalls   |
//! | JSON marshal DX      | ⚠ Medium   | Memory protocol adds boilerplate; wasm-bindgen helps |
//! | Multi-language tools | ✅ Yes      | Rust, AssemblyScript, C/C++, Zig compile to WASM |
//! | Agent loop changes   | ✅ None     | Adapter pattern is fully transparent          |
//!
//! **Recommendation**: Proceed if binary size is acceptable (desktop app).
//! Switch to `wasm3` crate for CLI/mobile targets.  Ship a Rust WASM SDK
//! (proc-macros + alloc helpers) to reduce host-protocol boilerplate before
//! making extensions user-facing.

pub mod adapter;
pub mod runtime;

pub use adapter::{WasmCallProtocol, WasmToolAdapter};
pub use runtime::{WasmModule, WasmRuntime};
