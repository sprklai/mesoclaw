# MesoClaw Testing and Debugging Guide

This guide covers tools and techniques for testing and debugging the MesoClaw desktop application during development.

---

## Table of Contents

- [CrabNebula DevTools](#crabnebula-devtools)
- [WebView DevTools](#webview-devtools)
- [Rust Backend Debugging](#rust-backend-debugging)
- [Frontend Debugging](#frontend-debugging)
- [Logging](#logging)
- [Common Debugging Scenarios](#common-debugging-scenarios)

---

## CrabNebula DevTools

[CrabNebula DevTools](https://v2.tauri.app/develop/debug/crabnebula-devtools/) is a free instrumentation tool for Tauri applications that provides real-time visualization of:

- **Log events** - Including logs from dependencies
- **Command performance** - Track Tauri command call performance
- **API usage** - Overall Tauri API usage metrics
- **Events and Commands** - Special interface for Tauri events with payload, responses, and execution spans

### Setup

CrabNebula DevTools is already configured in this project. The plugin is:

1. Added as a development dependency in `src-tauri/Cargo.toml`
2. Initialized in debug builds only in `src-tauri/src/lib.rs`

### Usage

1. Start the app in development mode:
   ```bash
   bun run tauri dev
   ```

2. When the app starts, DevTools will print a message to the terminal with a URL like:
   ```
   DevTools: http://localhost:3000
   ```

3. Open that URL in your browser to access the DevTools dashboard.

4. Interact with your app - all logs, commands, and events will appear in real-time.

### Features

| Feature | Description |
|---------|-------------|
| **Logs Panel** | View all log statements from Rust backend, filtered by level |
| **Commands Panel** | See all Tauri IPC commands with timing, arguments, and responses |
| **Events Panel** | Track frontend-backend event emissions |
| **Performance** | Identify slow commands and bottlenecks |

### Troubleshooting

- **No DevTools URL appears**: Ensure you're running a debug build (`bun run tauri dev`), not a release build
- **Connection refused**: Check if another process is using the DevTools port
- **Missing logs**: Verify logging is initialized via `plugins::logging::init()`

---

## WebView DevTools

For debugging the frontend (React/TypeScript) layer, you can access the WebView DevTools:

### Accessing DevTools

1. Run the app in development mode:
   ```bash
   bun run tauri dev
   ```

2. **On macOS/Linux**: Right-click anywhere in the app window and select "Inspect Element"

3. **Alternative**: Add a keyboard shortcut or menu item to open DevTools programmatically

### Useful for

- Inspecting React component hierarchy
- Debugging CSS styling issues
- Monitoring network requests
- Viewing console.log output from frontend code
- Checking localStorage and sessionStorage

---

## Rust Backend Debugging

### VS Code + rust-analyzer

1. Install the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension
2. Install [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) for debugging

### Launch Configuration

Create or update `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Tauri Development Debug",
      "cargo": {
        "args": [
          "build",
          "--manifest-path=./src-tauri/Cargo.toml",
          "--no-default-features",
          "--features=desktop"
        ],
        "filter": {
          "name": "mesoclaw-desktop",
          "kind": "bin"
        }
      },
      "preLaunchTask": "ui:dev",
      "executable": "${workspaceFolder}/src-tauri/target/debug/mesoclaw-desktop",
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Setting Breakpoints

1. Open a Rust file in VS Code
2. Click in the left margin to set a breakpoint (red dot appears)
3. Run the debug configuration
4. Execution will pause at the breakpoint

### Debug Console

While paused at a breakpoint:
- Inspect variable values in the "Variables" panel
- Use the "Debug Console" to evaluate expressions
- Step through code with F10 (step over) / F11 (step into)

---

## Frontend Debugging

### React DevTools

Install the [React Developer Tools](https://react.dev/learn/react-developer-tools) browser extension.

When running `bun run dev` (Vite dev server), you can use React DevTools in your browser to inspect:
- Component hierarchy
- Props and state
- React performance profiling

### Frontend Logs

Frontend `console.log` statements appear in:
1. WebView DevTools (right-click → Inspect)
2. Terminal output when running `bun run tauri dev`

### Zustand State Debugging

For debugging Zustand stores, you can add logging middleware:

```typescript
// In your store file
const useStore = create(
  devtools(
    (set) => ({
      // your store implementation
    }),
    { name: 'StoreName' }
  )
);
```

---

## Logging

### Backend Logging (Rust)

The app uses the `tracing` crate for structured logging. Log levels:

| Level | Use Case |
|-------|----------|
| `error!` | Errors requiring immediate attention |
| `warn!` | Potential issues or degraded functionality |
| `info!` | Important state changes or milestones |
| `debug!` | Detailed diagnostic information |
| `trace!` | Very verbose debugging |

#### Log File Location

Logs are stored in the Tauri app log directory:
- **macOS**: `~/Library/Logs/com.sprklai.mesoclaw/`
- **Linux**: `~/.local/share/mesoclaw/logs/`
- **Windows**: `%APPDATA%\com.sprklai.mesoclaw\logs\`

#### Viewing Logs

1. **In-app**: Navigate to the Logs page (sidebar → Logs)
2. **Via terminal**: Logs appear in stdout when running `bun run tauri dev`
3. **Via CrabNebula DevTools**: Real-time log viewer in browser

### Frontend Logging

```typescript
// Use console methods with appropriate levels
console.log('Info message');
console.warn('Warning message');
console.error('Error message', error);
```

---

## Common Debugging Scenarios

### Tauri Command Not Found

**Symptom**: `Error: command not found` in frontend

**Debug Steps**:
1. Verify the command is registered in `src-tauri/src/lib.rs`:
   ```rust
   .invoke_handler(tauri::generate_handler![
       commands::your_command,
       // ...
   ])
   ```
2. Check command naming - must have `_command` suffix
3. Rebuild after adding new commands: `cargo build`

### IPC Communication Issues

**Symptom**: Frontend not receiving data from backend

**Debug Steps**:
1. Use CrabNebula DevTools to see command calls and responses
2. Add logging to the command:
   ```rust
   #[tauri::command]
   pub async fn my_command() -> Result<String, String> {
       log::debug!("my_command called");
       // ...
   }
   ```
3. Check frontend invocation:
   ```typescript
   try {
     const result = await invoke<string>("my_command");
     console.log("Result:", result);
   } catch (error) {
     console.error("Command failed:", error);
   }
   ```

### Event Not Received

**Symptom**: Frontend event listener not triggering

**Debug Steps**:
1. Verify event emission in Rust:
   ```rust
   app_handle.emit("event-name", &payload)?;
   log::debug!("Emitted event-name with payload: {:?}", payload);
   ```
2. Check frontend listener:
   ```typescript
   import { listen } from '@tauri-apps/api/event';

   const unlisten = await listen('event-name', (event) => {
     console.log('Received:', event.payload);
   });
   ```
3. Ensure listener is set up before event is emitted

### Performance Issues

**Symptom**: App feels slow or unresponsive

**Debug Steps**:
1. Use CrabNebula DevTools to identify slow commands
2. Check for blocking operations in Rust - should use async/await
3. Profile React components with React DevTools Profiler
4. Check for memory leaks in browser DevTools → Memory tab

### API Key Storage Issues

**Symptom**: API keys not persisting or keyring errors

**Debug Steps**:
1. Check OS keyring is accessible:
   - **macOS**: Keychain Access app
   - **Linux**: gnome-keyring or kwallet
   - **Windows**: Credential Manager
2. Look for keyring errors in logs:
   ```bash
   grep -i keyring ~/.local/share/mesoclaw/logs/*.log
   ```
3. Try clearing and re-saving the API key in Settings

---

## Testing During Development

### Running Tests

```bash
# Frontend tests
bun run test

# Backend tests
cd src-tauri && cargo test --lib

# Run specific test
cargo test --lib test_name

# Run with output
cargo test --lib -- --nocapture
```

### Manual Testing Checklist

See [user_testing.md](./user_testing.md) for comprehensive manual test cases.

---

## Additional Resources

- [Tauri Debugging Guide](https://v2.tauri.app/develop/debug/)
- [CrabNebula DevTools Documentation](https://v2.tauri.app/develop/debug/crabnebula-devtools/)
- [VS Code LLDB Extension](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
- [tracing crate documentation](https://docs.rs/tracing/)
