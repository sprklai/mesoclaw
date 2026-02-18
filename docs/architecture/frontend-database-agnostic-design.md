# Archived: Frontend Database-Agnostic Design

Status: Archived on 2026-02-18.

This document describes a legacy multi-database architecture (external DB providers, SSH options, and database-type-specific UI) that is no longer implemented in the current codebase.

Current direction:
- Prompt-template based skill system
- SQLite-backed app data in `src-tauri/src/database/`
- AI provider management via `src-tauri/src/commands/ai_providers.rs`

Use these instead:
- `docs/implementation-plan.md`
- `docs/index.md`
- `src-tauri/src/commands/skills.rs`
- `src-tauri/src/prompts/mod.rs`
