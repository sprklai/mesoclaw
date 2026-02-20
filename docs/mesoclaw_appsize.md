# MesoClaw Application Size Analysis

## Executive Summary

This document analyzes what contributes to the 70MB+ application bundle size for MesoClaw and provides actionable recommendations to reduce it.

**Key Finding:** The size comes from THREE major sources:
1. **Frontend Assets** (~15MB) - Large PNG images and JavaScript bundles
2. **Rust Binary** (~20-40MB) - Tauri + heavy dependencies (Tauri, SQLite, async frameworks)
3. **Runtime Dependencies** - OS webview, shared libraries

---

## Current Size Breakdown (Measured)

### Rust Binary (Actual Measurements)

| Build Type | Size | Date |
|------------|------|------|
| **Release** | **31MB** | Feb 20, 2025 |
| Debug | 205MB | Feb 20, 2025 |

**The release binary is 31MB** - already optimized with:
- `opt-level = "z"` - Optimize for size
- `lto = true` - Link-time optimization
- `strip = true` - Remove debug symbols
- `panic = "abort"` - Remove panic unwinding
- `codegen-units = 1` - Better optimization

### Frontend (dist/): ~15MB

| Component | Size | Notes |
|-----------|------|-------|
| `mesoclaw.png` | 6.6MB | **OVERSIZED** - Application icon |
| `mesoclaw-lobster.png` | 6.6MB | **OVERSIZED** - Alternative icon |
| `dist/assets/*.js` | 1.3MB | Total JavaScript bundles |
| `dist/assets/*.css` | 96KB | Styles |
| Other | ~300KB | HTML, config |

**Issue:** Two PNG files alone account for **13.2MB (88%)** of frontend assets.

### Total Estimated Bundle Size

| Component | Size |
|-----------|------|
| Rust Binary | 31MB |
| Frontend Assets | 15MB |
| **Subtotal** | **46MB** |
| WebView/WebKit Runtime | ~20-30MB (system) |
| **Total Application** | **~66-76MB** |

### Major Rust Dependencies

The Cargo.toml includes many dependencies that add to binary size:

#### Heavy Dependencies (By Category)

1. **Tauri Ecosystem** (~10-15MB compressed)
   - `tauri` (core framework)
   - `tauri-plugin-*` (8+ plugins: notification, shell, store, updater, etc.)

2. **Database Layer** (~3-5MB)
   - `diesel` (ORM)
   - `libsqlite3-sys` with `bundled-full` features
   - `rusqlite` with `bundled-full` features
   - **Duplicate:** Both diesel and rusqlite link SQLite!

3. **Async/Web Framework** (~5-8MB)
   - `tokio` with many features
   - `axum` + `tower-http` (gateway server)
   - `hyper` (HTTP)
   - `reqwest` (HTTP client)

4. **Channel Integrations** (~8-12MB, mostly optional)
   - `teloxide` (Telegram)
   - `serenity` (Discord)
   - `matrix-sdk` (Matrix)
   - `slack-morphism` (Slack)

5. **Other Heavy Crates**
   - `regex` (~1MB)
   - `tera` (template engine, ~2MB)
   - `chrono` (datetime)
   - `serde` + `serde_json`
   - `notify` (file watching)

---

## Root Causes

### 1. Oversized Images (Immediate Win)
- **13.2MB** in two PNG files that should be <500KB total

### 2. Duplicate SQLite
Both `libsqlite3-sys` and `rusqlite` are bundled, adding 2-3MB twice

### 3. Unconditional Channel Integrations
Telegram, Discord, Slack, Matrix are compiled in even if not used

### 4. WASM Runtime (Optional but present)
`wasmtime` adds ~15MB when enabled (commented in Cargo.toml)

### 5. Full SQLite Features
`bundled-full` includes features like FTS5, GeoJSON, session extension

### 6. Axum/Gateway Server
Compiled in by default for all users, even CLI-only users

---

## Recommendations

### Priority 1: Quick Wins (Save ~13MB)

1. **Optimize Images** `src-tauri/` or `public/`
   ```bash
   # Convert PNG to optimized versions
   pngquant --quality=80-95 mesoclaw.png
   # Or use smaller formats
   ```
   - Target: <200KB per icon
   - **Savings: ~12MB**

2. **Remove Duplicate SQLite**
   ```toml
   # In src-tauri/Cargo.toml, remove one:
   # Keep: diesel + libsqlite3-sys (for ORM)
   # Remove: rusqlite (if diesel can handle all cases)
   # OR use rusqlite directly without diesel
   ```
   - **Savings: 2-3MB**

### Priority 2: Feature Flags (Save ~8-15MB)

3. **Make Channels Truly Optional**
   ```toml
   # Current default features:
   default = ["core", "cli", "desktop", "gateway",
              "channels-telegram", "channels-discord",
              "channels-slack"]

   # Recommended:
   default = ["core", "cli", "desktop"]
   # Users add channels via features
   ```
   - Users who don't use Discord save ~3MB
   - **Savings: Varies, up to 10MB if all disabled**

4. **Gateway Server Should Be Optional**
   ```toml
   # Remove gateway from default features
   # Only enable when explicitly building with gateway
   ```
   - **Savings: 3-5MB**

### Priority 3: Dependency Optimization (Save ~3-5MB)

5. **Use System SQLite on Linux**
   ```toml
   [target.'cfg(target_os = "linux")'.dependencies]
   libsqlite3-sys = { version = "0.30", features = [] }  # No bundled
   ```
   - **Savings: 1-2MB on Linux**

6. **Reduce Tokio Features**
   ```toml
   # Current: lots of features
   # Audit and remove unused ones:
   tokio = { version = "1", features = ["sync", "rt-multi-thread"] }
   # Remove: net, macros, process, io-util if unused
   ```
   - **Savings: 1-2MB**

7. **Replace Heavy Crates**
   - `tera` → `askama` (compile-time templates, ~500KB smaller)
   - `regex` → use `regex-lite` for simple patterns
   - **Savings: ~1-2MB**

### Priority 4: Advanced Optimizations

8. **Upstream Stripping**
   ```toml
   [profile.release]
   strip = true  # Already enabled
   # Additional:
   # Consider using `upx` for further compression
   ```

9. **Dynamic Linking (Linux)**
   ```toml
   # Link against system libwebkit2gtk instead of bundling
   # Requires changes to packaging config
   ```

10. **Split Binaries**
   - Build `mesoclaw-cli` and `mesoclaw-desktop` separately
   - Each can have different feature sets
   - CLI users don't need Tauri overhead

---

## Implementation Plan

### Phase 1: Immediate (1-2 hours)
1. Optimize/convert the two PNG images
2. Remove duplicate SQLite dependency
3. Test that everything still works

### Phase 2: Feature Flags (2-4 hours)
1. Move channels out of default features
2. Make gateway optional
3. Update CI/CD to build variants
4. Document feature combinations

### Phase 3: Dependency Audit (4-8 hours)
1. Audit tokio features used in code
2. Replace tera with askama
3. Test each change independently

### Phase 4: Advanced (1-2 days)
1. Implement dynamic linking for Linux
2. Create separate CLI and desktop builds
3. Add UPX compression to release pipeline

---

## Expected Results

| Optimization | Estimated Savings | Effort |
|--------------|-------------------|--------|
| Image optimization | ~12MB | Low |
| Remove duplicate SQLite | ~2MB | Low |
| Make channels optional | ~8MB (if all unused) | Medium |
| Make gateway optional | ~4MB | Medium |
| Tokio feature audit | ~1MB | Medium |
| Replace tera | ~1MB | Medium |
| **Total (Quick Wins)** | **~14MB** | **Low** |
| **Total (All)** | **~28MB** | **High** |

**Realistic target:** Reduce from ~70MB to ~45MB (Quick Wins + Phase 2)

---

## Measurement Commands

```bash
# Measure binary size
ls -lh src-tauri/target/release/mesoclaw-desktop

# Measure frontend assets
du -sh dist/

# Analyze Rust dependencies
cargo tree --features default | wc -l
cargo tree --features default --format "{p}" | sort | uniq

# Find largest crate contributions
cargo install cargo-bloat
cargo bloat --release --crates

# Check bundle size
bun run tauri build
find src-tauri/target/release/bundle -type f -exec ls -lh {} \; | sort -k5 -h
```

---

## Resources

- [Tauri Binary Size Guide](https://v2.tauri.app/distribute/binary-size/)
- [Cargo-Bloat](https://github.com/RazrFalcon/cargo-bloat) - Analyze what's in your binary
- [UPX Compressor](https://upx.github.io/) - Further compress executables
- [WASM Benchmarks](https://github.com/bytecodealliance/wasmtime#benchmarks) - WASM runtime is heavy

---

*Last Updated: 2026-02-20*
