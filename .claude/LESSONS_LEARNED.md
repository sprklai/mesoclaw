# Lessons Learned & Best Practices

Documenting issues encountered during development and solutions to avoid repeating mistakes.

---

## Configuration Management

### MCP Server Duplication
**Issue:** Two Serena dashboards opened on launch.

**Root Cause:** Serena configured in TWO places simultaneously:
- `.mcp.json` (project-level MCP config)
- `~/.claude/settings.json` with `"serena@claude-plugins-official": true`

**Solution:** Choose ONE approach:
- Keep `.mcp.json` for explicit control → disable plugin
- Keep plugin for auto-updates → remove `.mcp.json` entry

**Best Practice:**
```
❌ Don't: Enable same MCP server in multiple locations
✅ Do: Use .mcp.json for project-specific servers, plugins for global tools
```

### Settings File Bloat
**Issue:** 243+ permission entries in global settings, including malformed entries like `"Bash(1)"`, `"Bash(done)"`, `"Bash(__NEW_LINE_...)"`.

**Root Cause:** Permissions added incrementally without cleanup; some entries were command output accidentally captured.

**Best Practice:**
```
❌ Don't: Let permissions grow unbounded
✅ Do: Periodically audit permissions with: grep -E '"Bash\([a-z0-9_]+\)"' settings.json
✅ Do: Use broader patterns: "Bash(cargo:*)" instead of individual commands
```

### Stale Cache Files
**Issue:** Multiple versions of same plugin cached (8 Serena versions, 2 temp_git_* dirs).

**Best Practice:**
```bash
# Periodically clean plugin cache
ls ~/.claude/plugins/cache/claude-plugins-official/*/

# Remove temp directories
rm -rf ~/.claude/plugins/cache/temp_git_*

# Clean old security warning states
rm -f ~/.claude/security_warnings_state_*.json
```

---

## Tauri + React Development

### IPC Command Naming
**Issue:** Inconsistent naming made commands hard to discover.

**Solution:** All Tauri commands use `*_command` suffix:
```rust
// ✅ Good
#[tauri::command]
pub async fn get_llm_provider_config_command() -> Result<Config, String>

// ❌ Bad - unclear if it's a Tauri command
pub async fn get_llm_provider_config() -> Result<Config, String>
```

### Async State in React
**Issue:** Race conditions when components unmount during async operations.

**Solution:** Use cleanup pattern:
```typescript
useEffect(() => {
  let cancelled = false;

  async function fetchData() {
    const result = await invoke("command");
    if (!cancelled) {
      setState(result);
    }
  }

  fetchData();
  return () => { cancelled = true; };
}, []);
```

### Zustand Store Hydration
**Issue:** Store state not persisting across app restarts.

**Solution:** Use `tauri-plugin-store` for persistence:
```typescript
// Don't rely on memory-only state
const useStore = create<State>()(
  persist(
    (set) => ({
      // state
    }),
    {
      name: 'store-name',
      storage: createJSONStorage(() => tauriStore),
    }
  )
);
```

---

## Rust Backend

### Error Handling Pattern
**Issue:** Unwrap calls causing panics in production.

**Solution:** All commands return `Result<T, String>`:
```rust
// ✅ Good - proper error propagation
#[tauri::command]
pub fn my_command() -> Result<String, String> {
    operation().map_err(|e| format!("Operation failed: {}", e))
}

// ❌ Bad - will panic
pub fn my_command() -> String {
    operation().unwrap()
}
```

### Rustls CryptoProvider Initialization
**Issue:** App panicked on startup with "CryptoProvider not installed".

**Root Cause:** Rustls 0.23 requires explicit crypto provider initialization.

**Solution:**
```rust
// In lib.rs or main.rs, before any TLS operations
rustls::crypto::ring::default_provider()
    .install_default()
    .expect("Failed to install rustls crypto provider");
```

### Database Migrations
**Issue:** Schema changes not applying in production.

**Best Practice:**
```bash
# Always test migrations with temporary database
DATABASE_URL="/tmp/test.db" diesel migration run

# Generate schema after migration
DATABASE_URL="/tmp/test.db" diesel print-schema > src-tauri/src/database/schema.rs
```

---

## TypeScript/Frontend

### Import Organization
**Issue:** Barrel files (index.ts re-exporting everything) causing circular dependencies and slow builds.

**Best Practice:**
```typescript
// ❌ Bad - barrel file
export * from "./components";
export * from "./utils";

// ✅ Good - direct imports
import { Button } from "@/components/ui/Button";
import { formatDate } from "@/lib/utils/format";
```

### Icon Management
**Issue:** Missing icon exports causing runtime errors.

**Solution:** Centralized icon management:
```typescript
// src/lib/icons/ui-icons.ts
export { Pause, ArrowDown, ArrowRight } from "lucide-react";

// src/lib/icons/index.ts
export * from "./ui-icons";

// Usage
import { Pause } from "@/lib/icons";
```

---

## Debugging & Troubleshooting

### Tauri Dev Tools
**Issue:** Hard to debug Rust backend issues.

**Best Practice:**
```bash
# Enable debug logging
RUST_LOG=debug bun run tauri dev

# Check logs
tail -f ~/.config/mesoclaw/logs/*.log

# Use Tauri devtools
# Right-click in app → Inspect Element
```

### Database Issues
**Issue:** SQLite database locked or corrupted.

**Best Practice:**
```bash
# Check database integrity
sqlite3 ~/.config/mesoclaw/mesoclaw.db "PRAGMA integrity_check;"

# Enable WAL mode for better concurrency
sqlite3 ~/.config/mesoclaw/mesoclaw.db "PRAGMA journal_mode=WAL;"
```

---

## Common Pitfalls to Avoid

| Pitfall | Solution |
|---------|----------|
| API keys in code | Use OS keyring via `tauri-plugin-keyring` |
| Hardcoded paths | Use Tauri's `app_config_dir()` API |
| Blocking main thread | Use `tokio::spawn()` for async work |
| Memory leaks in React | Clean up subscriptions in `useEffect` return |
| Unbounded state growth | Implement state pagination/cleanup |
| Missing error boundaries | Wrap components in error boundaries |

---

## Quick Reference Commands

```bash
# Format and lint
bunx ultracite fix && cargo fmt

# Type check
bun run typecheck && cargo check

# Run tests
bun test && cargo test --lib

# Build for production
bun run tauri build

# Clean Rust build artifacts
cargo clean

# Check for outdated dependencies
bun outdated && cargo outdated
```

---

## Contributing New Lessons

When you encounter and solve a significant issue:

1. Add it to the appropriate section above
2. Include: Issue → Root Cause → Solution → Best Practice
3. Keep entries concise and actionable
4. Update the Quick Reference if adding new commands
