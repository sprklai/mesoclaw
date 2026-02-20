# Draft: Documentation Reconciliation Plan

> Purpose: Resolve cross-document inconsistencies across architecture, phases, requirements traceability, and tests.
> Scope: `docs/architecture-diagram.md`, `docs/mesoclaw-gap-analysis.md`, `docs/product-requirements.md`, `docs/implementation-plan.md`, `docs/test-plan.md`, `docs/user-journey.md`, and existing design docs in `docs/plans/`.

---

## 1. Objectives

1. Establish one source of truth for runtime architecture: **gateway-first (HTTP/WebSocket), CLI-first, GUI as thin client**.
2. Align phase ordering for channels/Telegram and release mapping.
3. Close requirement traceability gaps (`FR`/`P` IDs to implementation tasks and tests).
4. Remove contradictory dependency and feature-flag definitions.
5. Add completion criteria so doc consistency is testable in review.

---

## 2. Source of Truth Decisions (Draft)

1. **Architecture**: Use `docs/plans/2026-02-16-cli-gateway-agents-design.md` as canonical for transport and process model.
2. **Module system and feature flags**: Use `docs/plans/2026-02-16-sidecar-modularity-design.md` as canonical, then sync test/implementation docs.
3. **PRD vs execution**: `docs/product-requirements.md` remains product truth; missing implementation/test items must be added or explicitly deferred.

---

## 3. Priority Fix Order

### P0: Resolve architectural contradictions

1. Update `docs/architecture-diagram.md`

- Replace IPC-first frontend/backend interaction sections with gateway-first equivalents.
- Remove/replace references to frontend `invoke()` for agent/provider/memory/scheduler operations.
- Keep Tauri IPC explicitly for window/tray/native integrations only.
- Update module tree/examples that still imply `commands/*` as primary control surface.

2. Update `docs/user-journey.md`

- Stage 2 flow should use gateway API language, not direct Tauri IPC command flow.
- Keep user-facing behavior unchanged; only transport model references change.

3. Update `docs/mesoclaw-gap-analysis.md`

- Either:
  - Option A: Mark as **superseded by** `docs/plans/2026-02-16-cli-gateway-agents-design.md` for architecture sections, or
  - Option B: Fully rewrite architecture overview to gateway-first.
- Remove contradictory claims that “gateway is Tauri IPC bridge” where design now requires HTTP/WS gateway.

### P1: Align phases and release mapping

4. Update `docs/implementation-plan.md` and `docs/product-requirements.md`

- Unify where P1.7 (Channel Trait) and P7.1 (Telegram) land.
- If adopting revised order from CLI/Gateway design:
  - Place Telegram under proactive/channels phase (earlier than current Phase 7), or
  - Update PRD release table to match current implementation order.

5. Update `docs/product-requirements.md` release plan

- Reconcile CI/CD and signing timing with `docs/implementation-plan.md` (currently Phase 8).
- Reconcile v1.1/v1.2 wording with actual phase placement of mobile and signing tasks.

### P2: Close requirement and testing gaps

6. Add missing implementation tasks for PRD-linked IDs

- Add explicit tasks (or explicit defer notes) for:
  - `P3.18` (memory hygiene)
  - `P3.20` (WASM extension system)

7. Expand `docs/test-plan.md` task-level sections

- Add dedicated task test sections for implementation tasks currently covered only by wave tests:
  - 4.2, 4.3
  - 5.1, 5.2, 5.3, 5.4, 5.5
  - 6.1, 6.2, 6.3, 6.4, 6.5, 6.6
  - 7.2, 7.3, 7.4
  - 8.1, 8.2, 8.3, 8.4, 8.5

### P3: Normalize definitions

8. Standardize feature flags across docs

- Choose one naming scheme: `sidecars`/`containers`/`mcp-client` vs `modules`.
- Update `docs/test-plan.md` feature-flag tests to match chosen names/defaults.

9. Standardize dependency versions

- Create one canonical dependency table in `docs/implementation-plan.md` appendix (or separate file).
- Sync all docs to the same `rusqlite`, `toml`, and related versions.
- Add “version policy” line: “use latest validated versions from lockfile when implementation begins.”

10. Security path policy harmonization

- Decide `/tmp` policy:
  - If blocked globally, update sidecar examples to use project-local temp paths.
  - If allowed with constraints, update gap/security docs to reflect exception.

---

## 4. File-by-File Edit Checklist

### `docs/architecture-diagram.md`

- [ ] Transport model sections updated to gateway-first.
- [ ] Data-flow diagrams updated for HTTP/WS path.
- [ ] Frontend library references (`invoke.ts`) replaced/renamed where needed.
- [ ] “axum optional” wording aligned with gateway-first decision.

### `docs/mesoclaw-gap-analysis.md`

- [ ] Architecture section no longer contradicts gateway-first design.
- [ ] P1.7/channel timing language aligned with implementation plan.
- [ ] Security blocked-path list aligned with chosen `/tmp` policy.

### `docs/product-requirements.md`

- [ ] `P3.18`/`P3.20` status reconciled (tasked or explicitly deferred).
- [ ] Release table aligned with current phase execution and CI/signing/mobile sequencing.

### `docs/implementation-plan.md`

- [ ] Channel/Telegram phase placement aligned with PRD + design docs.
- [ ] Add tasks or defer notes for `P3.18` and `P3.20`.
- [ ] Optional: add dependency/version appendix as canonical reference.

### `docs/test-plan.md`

- [ ] Task-level test sections added for all missing implementation tasks.
- [ ] Feature-flag test cases updated to final feature naming/defaults.

### `docs/user-journey.md`

- [ ] Stage 2 and channel interaction references aligned to gateway transport.

---

## 5. Acceptance Criteria

1. No architecture statements conflict between:

- `docs/plans/2026-02-16-cli-gateway-agents-design.md`
- `docs/architecture-diagram.md`
- `docs/user-journey.md`
- `docs/mesoclaw-gap-analysis.md`

2. Every PRD planned ID maps to one of:

- an implementation task, or
- an explicit deferred/out-of-scope note with rationale.

3. Every implementation task has either:

- a dedicated task-level test section in `docs/test-plan.md`, or
- an explicit “covered by wave test only” note.

4. Feature flags and dependency versions are internally consistent across all docs.

---

## 6. Suggested Execution Sequence (Single PR)

1. Architecture docs (`architecture-diagram`, `user-journey`, `gap-analysis`)
2. Phase/release reconciliation (`product-requirements`, `implementation-plan`)
3. Traceability closure (`implementation-plan`, `test-plan`)
4. Normalization pass (feature flags, dependencies, security path policy)
5. Final consistency review using simple grep checks for old terms (`invoke(` references in backend-control context, conflicting feature names)

---

## 7. Open Decisions to Confirm

1. Should `docs/mesoclaw-gap-analysis.md` be maintained as active architecture guidance, or kept as historical analysis with a superseded banner?
2. Final placement of Telegram (`Phase 4` vs `Phase 7`) based on team capacity and risk tolerance.
3. `/tmp` policy for security vs sidecar ergonomics.
