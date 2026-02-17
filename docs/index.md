# MesoClaw Documentation Index

This index organizes all documentation for the MesoClaw project, providing a structured guide to architecture, planning, features, and implementation.

## 🚀 Quick Start (New Contributors)

**Start here if you're new to the project:**

1. **[README](../README.md)** - Project overview, features, quick start commands
2. **[CLAUDE.md](../CLAUDE.md)** - High-level orientation and common development tasks
3. **[.claude/CLAUDE.md](../.claude/CLAUDE.md)** - Comprehensive project standards and code conventions

## 📋 Where to Start & Track Work

### For Implementation Work

**Finding Active Work:**
1. Check **[Implementation Plan](./implementation-plan.md)** - Master roadmap with phase status
2. Review **[Active Plans](#active-implementation-plans)** below - Current bite-sized task lists
3. Look at **Task Management** - Use `TaskList`, `TaskCreate`, `TaskUpdate` tools in Claude Code

**Workflow:**
```
Design → Plan → Implement → Test → Commit → Review
   ↓       ↓        ↓         ↓       ↓        ↓
Design  Bite-sized  TDD     Unit/    Git    Code
 Doc     Tasks    Approach  E2E           Review
```

**Task Tracking:**
- **In Claude Code Session**: Use Task tools (`/tasks` to list)
- **In Plans**: Each implementation plan has numbered tasks with checkboxes
- **In Git**: Commits reference task/plan numbers

### Implementation Sequence

**Master sequence is defined in the [Implementation Plan](./implementation-plan.md):**

1. **Phase 0:** Slim Down + Responsive
2. **Phase 1:** Foundation
3. **Phase 2:** Core Infrastructure
4. **Phase 3:** Agent Intelligence
5. **Phase 4:** Proactive Behavior
6. **Phase 5:** Config & DX
7. **Phase 6:** Extensions & UI
8. **Phase 7:** Channels & Mobile
9. **Phase 8:** CI/CD & Community

**Cross-cutting track:** i18n ✅ **COMPLETE** (2026-02-17)
- Plan: [i18n Implementation Plan](./plans/2026-02-16-i18n-implementation.md) — all 12 tasks done
- Design: [i18n Design](./plans/2026-02-16-i18n-design.md)

### Testing Sequence

**Execution order comes from [Test Plan](./test-plan.md):**

1. Unit tests (module/function level)
2. Task tests (per implementation task)
3. Phase/Wave integration tests
4. Manual tests (workflow validation)
5. E2E scenarios (release-level confidence)

**Specialized test guidance:**
- Accessibility: [Accessibility Improvements](./features/accessibility/ACCESSIBILITY_IMPROVEMENTS.md)
- Security: [Secure Storage](./security/SECURE_STORAGE.md)

**Test Plan Reference:** [Test Plan](./test-plan.md)

---

## 📚 Architecture Documentation

### Core Architecture
- **[Architecture Diagram](./architecture-diagram.md)** - Complete system architecture overview with diagrams
- **[Frontend Database-Agnostic Design](./architecture/frontend-database-agnostic-design.md)** - Frontend abstraction layer
- **[AI Multi-Provider Design](./architecture/AI_MULTI_PROVIDER_DESIGN.md)** - AI provider integration architecture
- **[Tauri Plugin Baseline](./architecture/tauri-plugin-baseline.md)** - Tauri plugin architecture and patterns

### Ecosystem Analysis
- **[Claw Ecosystem Analysis](./claw-ecosystem-analysis.md)** - Analysis of Claw family products (ZeroClaw, MicroClaw, etc.)
- **[Moltis/MicroClaw Analysis](./moltis-microclaw-analysis.md)** - Comparison with related products
- **[Mesoclaw Gap Analysis](./mesoclaw-gap-analysis.md)** - Feature gap analysis and roadmap

---

## 🎯 Implementation Plans

### Active Implementation Plans

**Current Focus:**

- **[Implementation Plan — Phase 0: Slim Down + Responsive](./implementation-plan.md#phase-0-slim-down-technical-debt-reduction)** 🔥 **START HERE**
  - 8 tasks: Clippy lints, provider consolidation, prompt templates, Diesel→rusqlite, responsive layout, two-binary structure, CLI shell, feature flags
  - Three parallel streams: Backend (0.1→0.2→0.3→0.4), Frontend (0.5), Architecture (0.6→0.7→0.8)
  - Status: In progress
  - Design: [Implementation Plan](./implementation-plan.md)

**Recently Completed:**

- **[i18n Implementation Plan](./plans/2026-02-16-i18n-implementation.md)** ✅ **COMPLETE** (2026-02-17)
  - Frontend internationalization with react-i18next — all 12 tasks done, 24/24 tests passing
  - Design: [i18n Design](./plans/2026-02-16-i18n-design.md)

**Recent Plans:**

- [MesoClaw Refactoring Plan](./plans/2026-02-16-mesoclaw-refactoring-plan.md) - Bite-sized refactoring tasks
- [MesoClaw Refactoring Design](./plans/2026-02-16-mesoclaw-refactoring-design.md) - Documentation and code cleanup
- [CLI + Gateway + Agents Design](./plans/2026-02-16-cli-gateway-agents-design.md) - CLI-first architecture
- [Sidecar Modularity Design](./plans/2026-02-16-sidecar-modularity-design.md) - Sidecar architecture for modularity
- [Doc Reconciliation Draft](./plans/2026-02-16-doc-reconciliation-draft.md) - Documentation cleanup

### Master Roadmap

- **[Implementation Plan](./implementation-plan.md)** - Phase-based master roadmap
- **[Test Plan](./test-plan.md)** - Comprehensive testing strategy

---

## 📖 Product & Requirements

- **[Product Requirements](./product-requirements.md)** - Complete PRD with functional requirements
- **[User Journey](./user-journey.md)** - User experience flows and scenarios
- **[Generated Diagrams](./generated-diagrams.md)** - Auto-generated architecture diagrams

---

## ⚡ Features

### AI & Chat
- **[Chat Functionality](./features/CHAT_FUNCTIONALITY.md)** - Chat interface implementation guide
- **[Skill System](./features/SKILL_SYSTEM.md)** - AI skill system architecture and usage

### Accessibility
- **[Accessibility Improvements](./features/accessibility/ACCESSIBILITY_IMPROVEMENTS.md)** - Accessibility enhancements
- **[Keyboard Navigation](./features/accessibility/KEYBOARD_NAVIGATION.md)** - Keyboard shortcuts and navigation patterns

---

## 🔒 Security

- **[Secure Storage](./security/SECURE_STORAGE.md)** - Secure credential storage design
- **[Secure Storage Quickstart](./security/SECURE_STORAGE_QUICKSTART.md)** - Quick reference guide
- **[Keychain Migration](./security/KEYCHAIN_MIGRATION.md)** - Migration guide for keychain storage

---

## 🎨 UI/UX

- **[UI/UX Improvements](./ux/UI_UX_IMPROVEMENTS.md)** - Interface improvements and enhancements
- **[Splash Screen Fix](./ui-fixes/SPLASH_SCREEN_FIX.md)** - Splash screen implementation
- **[Splash Screen Position Fix](./ui-fixes/SPLASH_SCREEN_POSITION_FIX.md)** - Splash screen positioning

---

## 🛠️ Build & Optimization

- **[Build Optimizations](./BUILD_OPTIMIZATIONS.md)** - Build performance improvements and optimization strategies

---

## 🔄 Workflow Guide

### Starting a New Feature

```
1. Design Phase
   ├─ Read: Product Requirements → User Journey
   ├─ Read: Related Architecture Docs
   ├─ Create: Design document (docs/plans/YYYY-MM-DD-{feature}-design.md)
   └─ Get: Design approval

2. Planning Phase
   ├─ Create: Implementation plan (docs/plans/YYYY-MM-DD-{feature}-implementation.md)
   ├─ Break down: Into bite-sized tasks (2-5 min each)
   └─ Define: Success criteria

3. Implementation Phase (TDD Approach)
   ├─ For each task:
   │  ├─ Write: Failing test
   │  ├─ Run: Verify test fails
   │  ├─ Implement: Minimal code to pass
   │  ├─ Run: Verify test passes
   │  └─ Commit: With descriptive message
   └─ Review: Code quality, security, accessibility

4. Testing Phase
   ├─ Unit tests: Individual components
   ├─ Integration tests: Feature integration
   ├─ E2E tests: User workflows
   ├─ Accessibility: Keyboard + screen reader
   └─ Security: Credential handling

5. Documentation Phase
   ├─ Update: Feature documentation
   ├─ Update: API documentation
   ├─ Update: This index (if needed)
   └─ Update: CHANGELOG
```

### Task Management in Claude Code

**Creating tasks:**
```
Use TaskCreate tool with:
- subject: Brief task title
- description: Detailed requirements
- activeForm: Present continuous (e.g., "Implementing feature")
```

**Tracking tasks:**
```
TaskList - View all tasks with status
TaskGet - Get task details
TaskUpdate - Update status (pending → in_progress → completed)
```

**Task workflow:**
```
pending → in_progress → completed
   ↓          ↓             ↓
 Created   Working on    Verified
           the task      & Done
```

---

## 📊 Current Project Status

Status is tracked in the master plans, not duplicated here:

- Roadmap source of truth: [Implementation Plan](./implementation-plan.md)
- Validation source of truth: [Test Plan](./test-plan.md)
- Cross-cutting active track: [i18n Implementation Plan](./plans/2026-02-16-i18n-implementation.md)

Use phase checkpoints in `docs/implementation-plan.md` and wave/manual test sections in `docs/test-plan.md` for current progress.

---

## 🎯 Quick Links by Role

### For Developers
1. Start: [CLAUDE.md](../CLAUDE.md)
2. Architecture: [Architecture Diagram](./architecture-diagram.md)
3. Active Work: [Phase 0: Slim Down + Responsive](./implementation-plan.md#phase-0-slim-down-technical-debt-reduction)
4. Testing: [Test Plan](./test-plan.md)
5. Standards: [.claude/CLAUDE.md](../.claude/CLAUDE.md)

### For Designers
1. User Flows: [User Journey](./user-journey.md)
2. UI/UX: [UI/UX Improvements](./ux/UI_UX_IMPROVEMENTS.md)
3. Accessibility: [Accessibility Improvements](./features/accessibility/ACCESSIBILITY_IMPROVEMENTS.md)

### For Security Reviewers
1. Credentials: [Secure Storage](./security/SECURE_STORAGE.md)
2. Architecture: [AI Multi-Provider Design](./architecture/AI_MULTI_PROVIDER_DESIGN.md)

### For Product Managers
1. Requirements: [Product Requirements](./product-requirements.md)
2. Roadmap: [Implementation Plan](./implementation-plan.md)
3. Status: Phase checkpoints in [Implementation Plan](./implementation-plan.md)

---

## 📝 Documentation Standards

**Creating new documentation:**

1. **Architecture Docs** → `docs/architecture/`
   - High-level design, diagrams, trade-offs
   - Format: Markdown with Mermaid diagrams

2. **Implementation Plans** → `docs/plans/YYYY-MM-DD-{feature}-implementation.md`
   - Bite-sized tasks (2-5 min each)
   - Exact file paths, complete code snippets
   - TDD approach: test → fail → implement → pass → commit

3. **Design Docs** → `docs/plans/YYYY-MM-DD-{feature}-design.md`
   - Requirements, approaches, decision rationale
   - Architecture overview, file structure

4. **Feature Docs** → `docs/features/`
   - User-facing functionality, usage examples
   - API references, code examples

5. **Security Docs** → `docs/security/`
   - Threat models, mitigation strategies
   - Secure coding guidelines

---

**Last Updated:** 2026-02-17
**Maintained By:** MesoClaw Development Team

**Need help?** Start with [README](../README.md) → [CLAUDE.md](../CLAUDE.md) → This index
