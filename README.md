# Mesoclaw

Desktop-first AI agent built with Tauri 2 (Rust backend + React frontend), with a gateway-centric architecture, modular sidecar/tool system, and phased implementation plan.

## Status

This repository is actively transitioning from legacy `tauri-ai-boilerplate` internals to the Mesoclaw architecture documented under `docs/`.

Use the docs below as the source of truth for architecture and roadmap.

## Quick Start

### Prerequisites

- Bun `>= 1.3`
- Rust (stable toolchain)
- Tauri v2 toolchain requirements for your OS

### Install and run

```bash
bun install
bun run tauri:dev
```

### Useful scripts

```bash
# Frontend
bun run dev
bun run build
bun run test
bun run lint

# Backend
bun run cargo:check
bun run cargo:build
bun run cargo:build:release
```

## Documentation Index

### Core Product/Architecture

- PRD: `docs/product-requirements.md`
- Architecture diagram: `docs/architecture-diagram.md`
- Implementation plan: `docs/implementation-plan.md`
- Test plan: `docs/test-plan.md`
- User journey: `docs/user-journey.md`
- Gap analysis: `docs/mesoclaw-gap-analysis.md`
- Ecosystem analysis: `docs/claw-ecosystem-analysis.md`

### Detailed Design Plans

- CLI/Gateway/Agent orchestration: `docs/plans/2026-02-16-cli-gateway-agents-design.md`
- Sidecar modularity architecture: `docs/plans/2026-02-16-sidecar-modularity-design.md`
- Documentation reconciliation draft: `docs/plans/2026-02-16-doc-reconciliation-draft.md`

### Tauri Plugin Baseline

- Plugin baseline and gaps: `docs/architecture/tauri-plugin-baseline.md`

## Current Direction (Summary)

1. Gateway-first control plane (`127.0.0.1` HTTP + WebSocket), with Tauri IPC reserved for native desktop integrations.
2. Two entry points (CLI + desktop) sharing one core backend.
3. Sidecar modules for extensibility (native/container/MCP paths).
4. Strong security posture (policy-gated tools, scoped capabilities, auditable actions).

## Branding and Naming

Product naming is centralized in `branding.config.json`.

### Rename the Product

```bash
# 1) Edit branding values
$EDITOR branding.config.json

# 2) Sync branding across frontend/backend metadata and generated constants
bun run branding:sync

# 3) (Optional) verify Rust backend still compiles
bun run cargo:check
```

This sync updates key frontend/backend identity values (package names, Tauri app identity, window titles, keychain/credential service names, splash/title files, and generated app identity constants).

### Typical fields to update

- `productName` (visible app name)
- `slug` (filesystem/config naming base)
- `reverseDomain` (bundle identifier prefix)
- `mainWindowTitle`, `htmlTitle`, `splashTitle`, `splashSubtitle`
- `keychainService`, `credentialsService`
- `openRouterHttpReferer`, `openRouterTitle`
- `skillsConfigDirName`

Relevant references:

- Architecture and roadmap: `docs/architecture-diagram.md`, `docs/implementation-plan.md`
- Product definition: `docs/product-requirements.md`
- Test expectations: `docs/test-plan.md`
