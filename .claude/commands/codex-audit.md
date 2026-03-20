# Cross-Model Code Audit: Codex Audits, Claude Validates & Fixes

This command runs a dual-agent audit: Codex CLI reviews code with fresh eyes, Claude validates each finding against actual code and CLAUDE.md conventions, then applies approved fixes with verification.

**It runs automatically with NO user prompts unless findings need user decisions.**

## Arguments

```
$ARGUMENTS
```

Parse the arguments string for these options:

| Arg | Values | Default | Description |
|-----|--------|---------|-------------|
| SCOPE | `uncommitted`, `branch:NAME`, `commit:SHA`, `files:p1,p2`, `full` | `uncommitted` | What code to review |
| --focus | `security`, `performance`, `logic`, `error-handling`, `concurrency`, `api`, `frontend`, `architecture`, `all` | `all` | Narrow the audit focus |
| --fix | flag | off | Auto-apply "Apply" findings without asking |
| --dry-run | flag | off | Show Codex findings only, no validation or fixes |

If `$ARGUMENTS` is empty, use defaults: scope=`uncommitted`, focus=`all`, no flags.

---

## Instructions

Follow these steps strictly and sequentially. Stop immediately if any pre-flight check fails.

### Step 0: Pre-flight checks

#### 0a. Check Codex is installed

Run `which codex`. If it fails, print:
```
Codex CLI not found. Install it:
  npm install -g @openai/codex
Then authenticate:
  codex login
```
STOP.

#### 0b. Check Codex auth

Run with 15-second timeout:
```bash
timeout 15 codex exec "echo ok" --sandbox read-only --ephemeral 2>&1
```
If this fails or returns an auth error, print:
```
Codex authentication failed. Run: codex login
```
STOP.

#### 0c. Validate scope

Based on the parsed SCOPE:

- **uncommitted**: Run `git status --porcelain`. If empty, print "Nothing to audit — no uncommitted changes." and STOP.
- **branch:NAME**: Run `git rev-parse --verify NAME 2>/dev/null`. If fails, print "Branch NAME does not exist." and STOP.
- **commit:SHA**: Run `git rev-parse --verify SHA 2>/dev/null`. If fails, print "Commit SHA not found." and STOP.
- **files:p1,p2,...**: Check each file exists. If any missing, print which files are missing and STOP.
- **full**: No validation needed.

Print: "Pre-flight passed. Starting audit..."

### Step 1: Build Codex prompt

Construct the prompt with three sections:

#### 1a. Project context injection

Include this verbatim in the prompt:
```
PROJECT CONVENTIONS (from CLAUDE.md — respect these when evaluating code):
- Rust 2024 edition, tokio async runtime
- Error handling: ZeniiError enum with thiserror, never Result<T, String> or .map_err(|e| e.to_string())
- Async: tokio::sync primitives only, never std::sync::Mutex in async paths
- No block_on() — use tokio::spawn or .await
- All SQLite ops via spawn_blocking (rusqlite is sync)
- Logging: tracing macros only (info!, warn!, error!, debug!), never println!
- Security: parameterized SQL only, never log credentials, zeroize for sensitive data
- No magic numbers: tunables belong in AppConfig
- Naming: snake_case (Rust), camelCase (TypeScript/Svelte)
- Frontend: max 1 $effect per Svelte component, WS for real-time, no polling
- Feature flags for optional modules (channels, scheduler)
- All public functions should have unit tests

ARCHITECTURE RULES:
- Workspace: 5 binary crates (desktop, mobile, cli, tui, daemon) + 1 shared core (zenii-core)
- ALL business logic lives in zenii-core. Binary crates are thin shells (<100 lines each)
- Zero business logic in binary crates — everything in zenii-core
- No code duplication — if used twice, extract to zenii-core
- Gateway: axum HTTP+WS server at 127.0.0.1:18981, all clients communicate through it
- Dependency direction: binaries -> zenii-core -> external crates (never reversed)
- Feature gates for optional modules: channels, channels-telegram, channels-slack, channels-discord, scheduler
- Config: all tunables in AppConfig (schema.rs), no magic numbers in business logic
- Tools: ToolRegistry with DashMap backing, registered in boot.rs
- AI: rig-core based agent with provider registry (DB-backed, 6 built-in providers)
- DB: rusqlite + sqlite-vec, WAL mode, migrations in transactions, all ops via spawn_blocking
- Frontend: SvelteKit SPA (adapter-static) + Svelte 5 runes + shadcn-svelte
```

#### 1b. Focus-area instructions

Based on `--focus` value, append one of:

- **security**: "Focus on: SQL injection, command injection, credential exposure, unsafe unwrap on user input, missing input validation at API boundaries, XSS in frontend, insecure defaults, authentication/authorization gaps."
- **performance**: "Focus on: unnecessary allocations, missing async, blocking calls in async context, N+1 queries, missing caching opportunities, large clones where references suffice, inefficient string building."
- **logic**: "Focus on: off-by-one errors, incorrect error handling flow, unreachable code, logic inversions, missing edge cases, incorrect state machine transitions, race conditions."
- **error-handling**: "Focus on: swallowed errors, unwrap/expect on fallible operations, incorrect error propagation, missing error context, catch-all error handlers that hide bugs."
- **concurrency**: "Focus on: data races, deadlock potential, incorrect lock ordering, holding locks across await points, missing synchronization, channel misuse."
- **api**: "Focus on: API contract violations, missing request validation, incorrect HTTP status codes, inconsistent error response format, missing CORS handling, undocumented endpoints."
- **frontend**: "Focus on: Svelte 5 reactivity issues, missing error states, accessibility problems, broken dark mode, stale state, memory leaks from unsubscribed stores."
- **architecture**: "Focus on: trait/abstraction boundaries, module coupling and cohesion, dependency direction violations (binary crates importing business logic, circular deps), single-responsibility violations, leaky abstractions, god structs/modules, misplaced logic (business logic outside zenii-core, presentation logic in core), unused or vestigial abstractions, inconsistent patterns across similar modules, feature-flag boundary correctness, config sprawl. This project follows a strict shared-core architecture: ALL business logic in zenii-core, binary crates are thin shells. Check for violations."
- **all**: "Review all aspects: correctness, security, performance, error handling, API design, architecture, code quality, test coverage gaps."

#### 1c. Output format instructions

Append this to the prompt:
```
OUTPUT FORMAT — use this exact structure for each finding:

### [CATEGORY] Finding Title
- **File(s)**: path/to/file.rs:line
- **Severity**: critical | high | medium | low
- **Description**: What's wrong and why it matters
- **Suggested fix**: Specific code change or approach
- **Effort**: trivial | small | medium | large

Categories: SECURITY, PERFORMANCE, LOGIC, ERROR-HANDLING, CONCURRENCY, API, ARCHITECTURE, CODE-QUALITY, TEST-GAP, STALE-CODE, CONVENTION

If you find no issues, output exactly: NO_ISSUES_FOUND
```

### Step 2: Execute Codex

Based on SCOPE, run one of these commands (5-minute timeout). Capture ALL output.

**uncommitted:**
```bash
timeout 300 codex review --uncommitted "FULL_PROMPT" 2>&1
```

**branch:NAME:**
```bash
timeout 300 codex review --base NAME "FULL_PROMPT" 2>&1
```

**commit:SHA:**
```bash
timeout 300 codex review --commit SHA "FULL_PROMPT" 2>&1
```

**files:p1,p2,...:**
```bash
timeout 300 codex exec "Review these files: p1, p2, ... FULL_PROMPT" --sandbox read-only --ephemeral 2>&1
```

**full:**
```bash
timeout 300 codex exec "Full codebase audit of this Rust+Svelte project. FULL_PROMPT" --sandbox read-only --ephemeral 2>&1
```

If the command times out, continue with whatever partial output was captured and print a warning: "Codex timed out after 5 minutes. Working with partial output."

If the output is empty or clearly garbled, print the raw output and say: "Could not parse Codex output. Raw output shown above. Proceed manually." STOP.

### Step 3: Parse findings

Read the Codex output and extract each finding into a structured list:
- category, title, files, severity, description, suggested_fix, effort

If output contains `NO_ISSUES_FOUND` or no findings are parseable:
Print: "Clean audit — Codex found no issues." STOP.

If `--dry-run` flag was set:
Print all parsed findings in a formatted table and STOP. Do not validate or fix anything.

### Step 4: Validate each finding (Claude's job)

For EACH finding from Codex, do the following:

#### 4a. Read the actual code

Use the Read tool to read the file(s) referenced in the finding. Read enough context (30-50 lines around the referenced location) to understand the code.

#### 4b. Determine if the issue is real

Ask yourself:
1. Does the code actually have this problem? (Codex may have hallucinated or misread)
2. Is this already handled elsewhere that Codex couldn't see?
3. Does this conflict with a CLAUDE.md convention? (Codex doesn't know our conventions)
4. Is this a matter of preference vs. an actual bug?

#### 4c. Classify the finding

Assign one of four dispositions:

| Disposition | Criteria | Action |
|-------------|----------|--------|
| **Apply** | Issue is real, fix is clear, low risk, aligns with CLAUDE.md | Will be fixed |
| **Skip** | False positive, already handled, convention conflict, or Codex misunderstanding | Won't fix, explain why |
| **Defer** | Issue is real but fix is complex, risky, or needs design discussion | Track for later |
| **User Decision** | Issue is real but fix has trade-offs the user should weigh | Ask user |

Record your reasoning for each classification. Be specific — "false positive" is not enough; explain what Codex got wrong.

### Step 5: Present structured report

Print the report in this format:

```
## Codex Audit Report

**Scope**: {scope} | **Focus**: {focus} | **Findings**: {total}

| Metric | Count |
|--------|-------|
| Total findings | N |
| Apply | N |
| Skip (false positive) | N |
| Defer | N |
| User Decision | N |
| False positive rate | N% |

### Findings to Apply

For each:
> **[CATEGORY] Title** (severity) — file:line
> Codex says: {description}
> Claude validates: {why this is a real issue}
> Fix: {what will be done}

### Findings Skipped

For each:
> **[CATEGORY] Title** — file:line
> Codex says: {description}
> Skipped because: {specific reason — convention conflict, false positive, already handled}

### Findings Deferred

For each:
> **[CATEGORY] Title** (severity) — file:line
> Codex says: {description}
> Deferred because: {why it needs more thought}
> Suggested tracking: {TODO comment, issue, or plan}

### Findings Needing User Decision

For each:
> **[CATEGORY] Title** (severity) — file:line
> Codex says: {description}
> Trade-offs: {option A vs option B with pros/cons}
> Recommendation: {what Claude would do and why}
```

### Step 6: Handle user decisions

If there are any "User Decision" findings:
- Ask the user for their disposition on each one (apply, skip, or defer)
- Wait for their response before proceeding

If `--fix` flag was set:
- Auto-apply all "Apply" findings without asking
- Still ask for "User Decision" items

If there are NO "Apply" or approved "User Decision" findings, print "No fixes to apply." and skip to Step 9.

### Step 7: Apply fixes

For each approved fix:

1. Edit the code following CLAUDE.md conventions (use Edit tool, minimal changes)
2. After every 3 fixes, run `cargo check --workspace` to catch breakage early
3. If a fix breaks compilation:
   - Immediately revert that specific edit
   - Note it in the report as "Reverted — broke compilation"
   - Continue with remaining fixes

For frontend fixes (Svelte/TypeScript files):
- After all frontend fixes, run `cd web && bun run check` once
- Revert any fix that causes type errors

### Step 8: Full verification

Run the full verification suite:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```

If `web/` files were touched:
```bash
cd web && bun run check && bun run test
```

If ANY test fails that was passing before:
- Identify which fix caused the failure
- Revert that fix
- Re-run verification to confirm green
- Note the revert in the final report

### Step 9: Save learnings

If any findings revealed recurring patterns (same bug type appearing in multiple places), save the pattern to memory for future prevention:
- What the pattern is
- Where it tends to appear
- How to prevent it

Only save genuinely useful patterns — not one-off issues.

### Step 10: Final summary

Print the final summary:

```
## Audit Complete

| Metric | Count |
|--------|-------|
| Total findings | N |
| Applied | N |
| Skipped | N |
| Deferred | N |
| Reverted (broke build) | N |
| User decisions | N |

**Verification**: cargo check OK | cargo test OK | cargo clippy OK | bun check OK

### Disagreements (Claude overrode Codex)
{List each case where Claude skipped a Codex finding, with reasoning.
These are the highest-signal items — they reveal blind spots in both models.}

### Applied Fixes
{List each fix: file, what changed, why}
```

---

## Edge Case Reference

| Case | Handling |
|------|----------|
| Codex not installed | Print install instructions, STOP |
| Codex auth expired | Tell user to run `codex login`, STOP |
| No uncommitted changes | Print message, STOP |
| Branch doesn't exist | Error message, STOP |
| Codex timeout (>5min) | Continue with partial output + warning |
| Unparseable output | Print raw output, suggest manual review, STOP |
| Fix breaks compilation | Revert that fix, note in report, continue |
| Fix breaks tests | Revert that fix, note in report, continue |
| Frontend-only changes | Skip Rust verification |
| Rust-only changes | Skip bun verification |
| Zero findings | "Clean audit" message, STOP |
