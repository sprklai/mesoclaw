# Observability — MesoClaw

Production observability is implemented via the [`tracing`](https://docs.rs/tracing) ecosystem.
All spans are written to a **rolling daily log file** on the user's machine.

---

## Log File Location

| Platform | Path |
|---|---|
| macOS | `~/Library/Logs/com.sprklai.mesoclaw/mesoclaw.YYYY-MM-DD.log` |
| Linux | `~/.local/share/com.sprklai.mesoclaw/mesoclaw.YYYY-MM-DD.log` |
| Windows | `%APPDATA%\com.sprklai.mesoclaw\mesoclaw.YYYY-MM-DD.log` |

Rolling files are kept automatically by `tracing-appender`. Each line is a structured plain-text event.

---

## Controlling Verbosity

Set the `RUST_LOG` environment variable before launching the app:

```bash
# Default (info and above)
RUST_LOG=info ./MesoClaw

# Debug everything
RUST_LOG=debug ./MesoClaw

# Debug only the agent, info elsewhere
RUST_LOG=info,mesoclaw::agent=debug ./MesoClaw

# Trace a specific module
RUST_LOG=mesoclaw::channels=trace ./MesoClaw
```

If `RUST_LOG` is unset, the level defaults to `info`.

---

## Instrumented Spans

Every instrumented function opens a span in the log. Span duration is recorded automatically.

### Agent

| Span | Fields | File |
|---|---|---|
| `agent.run` | `model`, `user_msg_len` | `src-tauri/src/agent/loop_.rs` |
| `agent.run_with_history` | `model`, `max_iterations`, `history_len` | `src-tauri/src/agent/loop_.rs` |
| `agent.tool` | `tool`, `call_id` | `src-tauri/src/agent/loop_.rs` |
| `command.agent.start` | `session_id` (deferred), `msg_len` | `src-tauri/src/agent/agent_commands.rs` |
| `command.agent.cancel` | `session_id` | `src-tauri/src/agent/agent_commands.rs` |

### Channels

| Span | Fields | File |
|---|---|---|
| `channel.register` | `channel` (name) | `src-tauri/src/channels/manager.rs` |
| `channel.unregister` | `name` | `src-tauri/src/channels/manager.rs` |
| `channel.send` | `channel_name`, `recipient`, `msg_len` | `src-tauri/src/channels/manager.rs` |
| `channel.health_all` | — | `src-tauri/src/channels/manager.rs` |
| `channel.start_all` | `buffer` | `src-tauri/src/channels/manager.rs` |

### Scheduler

| Span | Fields | File |
|---|---|---|
| `command.scheduler.list_jobs` | — | `src-tauri/src/scheduler/commands.rs` |
| `command.scheduler.create_job` | `name` | `src-tauri/src/scheduler/commands.rs` |
| `command.scheduler.toggle_job` | `job_id`, `enabled` | `src-tauri/src/scheduler/commands.rs` |
| `command.scheduler.delete_job` | `job_id` | `src-tauri/src/scheduler/commands.rs` |
| `command.scheduler.job_history` | `job_id` | `src-tauri/src/scheduler/commands.rs` |

### Gateway (HTTP daemon)

| Span | Fields | File |
|---|---|---|
| `gateway.create_session` | `channel` | `src-tauri/src/gateway/routes.rs` |
| `gateway.module_health` | `id` | `src-tauri/src/gateway/routes.rs` |
| `gateway.start_module` | `id` | `src-tauri/src/gateway/routes.rs` |
| `gateway.stop_module` | `id` | `src-tauri/src/gateway/routes.rs` |

### Sidecar Modules

| Span | Fields | File |
|---|---|---|
| `sidecar.start` | `id` (module id) | `src-tauri/src/modules/sidecar_service.rs` |
| `sidecar.stop` | `id` | `src-tauri/src/modules/sidecar_service.rs` |
| `sidecar.execute` | `id` | `src-tauri/src/modules/sidecar_service.rs` |
| `mcp.start` | `module` | `src-tauri/src/modules/mcp_client.rs` |
| `mcp.call_tool` | `module`, `tool` | `src-tauri/src/modules/mcp_client.rs` |

### Chat & Boot

| Span | Fields | File |
|---|---|---|
| `command.stream_chat` | `provider`, `model`, `session` | `src-tauri/src/commands/streaming_chat.rs` |
| `boot.run` | — | `src-tauri/src/services/boot.rs` |

---

## Reading Logs

**Tail the current day's log:**
```bash
tail -f ~/.local/share/com.sprklai.mesoclaw/mesoclaw.$(date +%Y-%m-%d).log
```

**Filter to agent spans only:**
```bash
grep "agent\." ~/.local/share/com.sprklai.mesoclaw/mesoclaw.$(date +%Y-%m-%d).log
```

**Find slow tool calls (look for `close` events with long duration):**
```bash
grep "agent.tool" ~/.local/share/com.sprklai.mesoclaw/mesoclaw.$(date +%Y-%m-%d).log
```

**macOS path:**
```bash
tail -f ~/Library/Logs/com.sprklai.mesoclaw/mesoclaw.$(date +%Y-%m-%d).log
```

---

## Adding New Spans

All existing `log::info!()` / `log::warn!()` calls are automatically forwarded into the tracing pipeline via `tracing_log::LogTracer` (configured in `src-tauri/src/plugins/logging.rs`).

To instrument a new async function:

```rust
#[tracing::instrument(
    name = "subsystem.operation",
    skip(self, large_arg),          // skip non-Debug or large args
    fields(key = %some_field)       // add structured context
)]
pub async fn my_fn(&self, large_arg: Vec<u8>, key: &str) -> Result<(), String> {
    // For fields computed inside the fn, use deferred recording:
    tracing::Span::current().record("dynamic_field", computed_value.as_str());
    // ...
}
```

---

## Stack

| Crate | Version | Role |
|---|---|---|
| `tracing` | 0.1 | Span/event macros (`#[instrument]`, `tracing::info!`) |
| `tracing-subscriber` | 0.3 | Subscriber registry + `EnvFilter` + `fmt` layer |
| `tracing-appender` | 0.2 | Non-blocking rolling file writer |
| `tracing-log` | 0.2 | Bridge: `log::` macros → tracing pipeline |
