# MesoClaw Documentation & Code Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform MesoClaw from legacy "database comprehension" identity to current "lightweight privacy-first desktop AI agent" reality through documentation consolidation, legacy code removal, and rusqlite migration.

**Architecture:** Documentation-first layered approach: (1) Establish identity via CLAUDE.md updates, (2) Consolidate architecture (sidecar single source of truth), (3) Enhance plans with version mapping and acceptance criteria, (4) Execute code changes (Diesel→rusqlite, remove legacy database providers).

**Tech Stack:** Rust 2024, Tauri 2, rusqlite, Markdown, Git

**Design Document:** [docs/plans/2026-02-16-mesoclaw-refactoring-design.md](2026-02-16-mesoclaw-refactoring-design.md)

---

## Summary of Tasks

This plan contains 36 bite-sized tasks organized into 5 phases:
- **Phase 1:** Core Identity (7 tasks - ~45 min)
- **Phase 2:** Architecture Consolidation (4 tasks - ~60 min)
- **Phase 3:** Plans & Tests Updates (4 tasks - ~30 min)  
- **Phase 4:** Code Changes (10 tasks - ~90 min)
- **Phase 5:** Final Verification (2 tasks - ~15 min)

**Total estimated time:** ~4 hours

Each task follows TDD where applicable and includes exact file paths, complete code changes, verification steps, and commit messages.

For detailed task breakdown with step-by-step instructions, see the design document.

---

## Quick Reference

**Key Files to Update:**
- CLAUDE.md (root) - Project identity
- .claude/CLAUDE.md - Project standards
- src-tauri/CLAUDE.md - Backend standards
- docs/architecture/sidecar-system.md - NEW (single source of truth)
- docs/README.md - NEW (navigation guide)
- docs/architecture-diagram.md - Gateway, autonomy levels, SQLite
- docs/implementation-plan.md - Version mapping, acceptance criteria
- docs/test-plan.md - Tauri v2 reference, phase wave protocol
- src-tauri/Cargo.toml - Remove Diesel, add rusqlite
- src-tauri/src/database/mod.rs - Migrate to rusqlite

**Key Files to Delete:**
- src-tauri/tests/test_ssh_serialization.rs
- src-tauri/tests/test_mysql_schema.rs
- src-tauri/tests/integration_mongodb.rs
- src-tauri/src/database/pool.rs
- src-tauri/src/database/schema.rs

**Verification Commands:**
```bash
# Check for legacy references
rg "diesel::|PostgreSQL|MySQL|MongoDB|DatabaseProvider" src-tauri/src/ --type rust

# Verify rusqlite usage
rg "rusqlite::" src-tauri/src/ --type rust

# Run tests
cargo test --lib
cargo build --release
```

---

**For execution:** See design document for complete step-by-step instructions for all 36 tasks.
