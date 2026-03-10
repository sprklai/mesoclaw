# MesoClaw TODO Tracker

> Auto-maintained list of all TODO, MOCK, FIX, and STUB items in the codebase.
> Last updated: 2026-03-09 (Phase 8.11 Autonomous Reasoning complete)

## Summary

| Type | Open | Done | Total |
|------|------|------|-------|
| TODO | 12 | 22 | 34 |
| STUB | 3 | 11 | 14 |
| MOCK | 0 | 0 | 0 |
| FIX | 0 | 0 | 0 |

---

## Pending Items

| Status | Type | File | Line | Description | Phase |
|--------|------|------|------|-------------|-------|
| [ ] | STUB | crates/mesoclaw-tui/src/main.rs | 2 | TUI binary — implement ratatui interface | Future Release (FR-1) |
| [ ] | TODO | tests/phase5_combined.md | M5.4 | Manual test: CLI chat streaming — requires configured API key to validate WS streaming | Phase 5 |
| [ ] | TODO | tests/phase5_combined.md | M5.5 | Manual test: CLI run single prompt — requires configured API key to validate POST /chat response | Phase 5 |
| [ ] | TODO | tests/phase4_agent_tools.md | 4.8-4.12 | Agent tool loop integration tests — single tool call, chained tools, max_retries, tool error, final response (requires mock LLM) | Future |
| [ ] | TODO | tests/phase4_agent_tools.md | M4.1 | Manual test: Websearch tool live test — requires API keys + chat | Future |
| [ ] | TODO | tests/phase4_agent_tools.md | M4.2 | Manual test: Sysinfo tool live test — ask "what system am I running on?" | Future |
| [ ] | TODO | tests/phase4_agent_tools.md | M4.3 | Manual test: Tool error recovery — disconnect network, verify graceful error | Future |
| [ ] | TODO | tests/phase8.3.2_web_search.md | M.WS.2-3 | Manual tests pending: multi-provider cascade live test, DuckDuckGo fallback live test | Phase 8.3.2 |
| [ ] | TODO | tests/phase8.8_channel_lifecycle.md | M.CR.1-8 | Manual tests — all require live bot tokens (Telegram, Slack, Discord) | Stage 8.8 |
| [ ] | TODO | tests/phase8.11_autonomous_reasoning.md | M18.1 | Manual test: Telegram desktop file discovery — requires live Telegram bot token | Phase 8.11 |
| [ ] | TODO | tests/phase8.11_autonomous_reasoning.md | M18.2 | Manual test: CLI downloads folder query — requires configured API key | Phase 8.11 |
| [ ] | TODO | tests/phase8.11_autonomous_reasoning.md | M18.3-5 | Manual tests: Continuation nudge, max limit, cross-interface — requires live LLM interaction | Phase 8.11 |

---

## Completed Items

| Status | Type | File | Description | Phase |
|--------|------|------|-------------|-------|
| [x] | TODO | crates/mesoclaw-core/src/config/schema.rs | Add Phase 2 config fields: memory scoring weights, rate limit window/max, embedding dim, embedding cache size | Phase 2 |
| [x] | TODO | crates/mesoclaw-daemon/src/main.rs | Start axum gateway server | Phase 3 |
| [x] | STUB | crates/mesoclaw-daemon/src/main.rs | Daemon fully wired: boot → gateway with graceful shutdown | Phase 3 |
| [x] | STUB | crates/mesoclaw-cli/src/main.rs | CLI binary — clap structure with 6 commands, MesoClient HTTP/WS wrapper, 10 tests | Phase 5 |
| [x] | STUB | crates/mesoclaw-desktop/src/lib.rs | Tauri 2.10 desktop shell — Builder with 5 plugins, tray, IPC, close-to-tray, embedded gateway | Phase 7 |
| [x] | STUB | crates/mesoclaw-core/src/tools/web_search.rs | WebSearchTool refactored — uses `websearch` crate with Tavily → Brave → DuckDuckGo cascade | Phase 8 |
| [x] | STUB | crates/mesoclaw-core/src/tools/process.rs | ProcessTool kill action implemented — sysinfo-based kill with Full autonomy gate | Stage 8.9 |
| [x] | TODO | tests/phase6_frontend.md | Manual test: Chat streaming end-to-end — WS connectivity verified | Phase 6 |
| [x] | TODO | tests/phase8.3_context.md | Core context unit tests implemented (ContextEngine, BootContext, tier injection, cache invalidation) | Stage 8.9 |
| [x] | TODO | tests/phase8.5_channel_router.md | 32 unit tests implemented and passing — integration tests and manual tests deferred | Phase 8.5 |
| [x] | TODO | tests/phase8.6_scheduler.md | 42 unit tests + 4 build verification tests — 52 tests + 6 CLI tests | Phase 8.6 |
| [x] | STUB | crates/mesoclaw-core/src/channels/router.rs | ChannelRouter orchestrator — end-to-end message pipeline with tool policy, formatting, session mapping | Stage 8.7 |
| [x] | STUB | crates/mesoclaw-core/src/channels/telegram/mod.rs | Telegram lifecycle hooks — on_agent_start/on_tool_use/on_agent_complete with typing refresh | Stage 8.8 |
| [x] | STUB | crates/mesoclaw-core/src/channels/slack/mod.rs | Slack lifecycle hooks — ephemeral messages for processing status | Stage 8.8 |
| [x] | TODO | tests/phase8.7_channel_router.md | Integration tests — ChannelRouter end-to-end, session persistence, tool policy filtering | Stage 8.7 |
| [x] | STUB | crates/mesoclaw-core/src/scheduler/tokio_scheduler.rs | Heartbeat/AgentTurn/SendViaChannel/Notify payloads implemented via PayloadExecutor | Stage 8.6.1 |
| [x] | TODO | crates/mesoclaw-core/src/scheduler/ | PayloadExecutor module — real payload execution for all 4 types | Stage 8.6.1 |
| [x] | TODO | crates/mesoclaw-core/src/gateway/handlers/ws.rs | WS /ws/notifications endpoint — push notifications to clients | Stage 8.6.1 |
| [x] | TODO | web/src/lib/stores/ | notifications.svelte.ts — WS notification store + svelte-sonner toasts | Stage 8.6.1 |
| [x] | TODO | crates/mesoclaw-desktop/ | Desktop OS notifications — tauri-plugin-notification with permission request | Stage 8.6.1 |
| [x] | TODO | crates/mesoclaw-core/src/tools/memory_tool.rs | MemoryTool — agent tool for store/recall/forget memory operations | Phase 8.10 |
| [x] | TODO | crates/mesoclaw-core/src/tools/config_tool.rs | ConfigTool — agent tool for get/update config with whitelist enforcement | Phase 8.10 |
| [x] | TODO | crates/mesoclaw-core/src/tools/channel_tool.rs | ChannelSendTool — agent tool for send/list/status channels (feature-gated) | Phase 8.10 |
| [x] | TODO | crates/mesoclaw-core/src/tools/scheduler_tool.rs | SchedulerTool — agent tool for create/list/delete/toggle/history jobs (feature-gated) | Phase 8.10 |
| [x] | TODO | crates/mesoclaw-core/src/boot.rs | Boot registration of 4 agent action tools (memory, config, channel_send, scheduler) | Phase 8.10 |
| [x] | TODO | crates/mesoclaw-core/src/ai/reasoning/ | ReasoningEngine + ContinuationStrategy — extensible reasoning with autonomous continuation | Phase 8.11 |
| [x] | TODO | crates/mesoclaw-core/src/ai/context.rs | BootContext extended with environment discovery (home, desktop, downloads, shell, username) | Phase 8.11 |
| [x] | TODO | crates/mesoclaw-core/src/ai/context.rs | Reasoning guidance section in compose_full() system prompt | Phase 8.11 |
| [x] | TODO | crates/mesoclaw-core/src/config/schema.rs | agent_max_continuations + agent_reasoning_guidance config fields | Phase 8.11 |
| [x] | TODO | crates/mesoclaw-core/src/gateway/ | All 4 call sites swapped to reasoning_engine.chat() (chat, ws, router, scheduler) | Phase 8.11 |
| [x] | TODO | crates/mesoclaw-core/src/boot.rs | ReasoningEngine initialization with ContinuationStrategy wired into Services + AppState | Phase 8.11 |

---

## Notes

- `mesoclaw-mobile` crate has no source files yet — deferred to Future Release (FR-2)
- `mesoclaw-tui` crate is a stub — deferred to Future Release (FR-1)
- Phase 1-7 completed: full Rust workspace + Svelte frontend + Tauri desktop
- Phase 8 fully complete (15.1, 15.2, 15.3, 15.3b, 15.5, 16, 8.6.1, 8.7, 8.8, 8.9, 8.10)
- Phase 8.11 automated tests complete: 20 new tests (868 total workspace), 0 clippy warnings
- Phase 8.11 manual tests (M18.1-M18.5) pending — require live API keys/bot tokens
- Stages 9-15 complete: build, CI/CD, quality, docs, licensing
- Per "No magic numbers" rule: all tunables in AppConfig (46+ fields including agent_max_continuations)
- Only remaining work: TUI (FR-1) and Mobile (FR-2) — explicitly deferred to future release
