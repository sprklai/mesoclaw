# Cross-Model Code Audit: Codex Audits, Claude Judges & Fixes

This command runs a dual-agent audit: Codex CLI reviews code with fresh eyes, then Claude independently reads the actual code, judges each finding's validity, designs its own fixes where warranted, and applies only what passes rigorous verification.

**Codex proposes, Claude disposes.** Claude is the architect — it reads every referenced file, forms its own opinion, and never applies Codex's suggested fixes verbatim. A wrong fix is worse than no fix.

**It runs automatically with NO user prompts unless findings need user decisions.**

## Arguments

```
$ARGUMENTS
```

Parse the arguments string for these options:

| Arg | Values | Default | Description |
|-----|--------|---------|-------------|
| SCOPE | `uncommitted`, `branch:NAME`, `commit:SHA`, `files:p1,p2`, `full` | `uncommitted` | What code to review |
| --focus | `security`, `performance`, `logic`, `error-handling`, `concurrency`, `race-conditions`, `api`, `frontend`, `architecture`, `all` | `all` | Narrow the audit focus |
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
- **performance**: "Focus on: unnecessary allocations and large clones where references or Cow suffice, blocking calls in async context (spawn_blocking missing), N+1 queries, missing caching opportunities, inefficient string building (repeated format! vs push_str), hot-path Vec/HashMap pre-allocation, unnecessary Arc/Mutex overhead, excessive serialization/deserialization, unbounded collection growth, redundant database round-trips, missing connection pooling, slow startup paths, large response payloads, iterator vs collect tradeoffs, lock contention in concurrent paths."
- **logic**: "Focus on: off-by-one errors, incorrect error handling flow, unreachable code, logic inversions, missing edge cases, incorrect state machine transitions, silent data truncation, integer overflow/underflow, incorrect boolean logic, missing None/Err handling on Option/Result chains."
- **error-handling**: "Focus on: swallowed errors, unwrap/expect on fallible operations, incorrect error propagation, missing error context, catch-all error handlers that hide bugs."
- **concurrency**: "Focus on: data races, deadlock potential, incorrect lock ordering, holding locks across await points, missing synchronization, channel misuse, unbounded channel/queue growth, task cancellation safety (drop during await), missing JoinHandle collection (fire-and-forget spawns that silently fail)."
- **race-conditions**: "Focus on: TOCTOU (time-of-check-to-time-of-use) bugs, check-then-act without atomic operations, concurrent read-modify-write without synchronization, shared mutable state accessed from multiple tasks without locks, event ordering assumptions (WebSocket messages arriving out of order, broadcast receivers missing messages), database read-then-write without transactions, file system races (check existence then create/read), DashMap entry API misuse (get then insert instead of entry), tokio::select! cancellation leaving state inconsistent, double-free or double-init in startup/shutdown sequences, missing fencing in pub/sub patterns."
- **api**: "Focus on: API contract violations, missing request validation, incorrect HTTP status codes, inconsistent error response format, missing CORS handling, undocumented endpoints."
- **frontend**: "Focus on: Svelte 5 reactivity issues, missing error states, accessibility problems, broken dark mode, stale state, memory leaks from unsubscribed stores."
- **architecture**: "Focus on: trait/abstraction boundaries, module coupling and cohesion, dependency direction violations (binary crates importing business logic, circular deps), single-responsibility violations, leaky abstractions, god structs/modules, misplaced logic (business logic outside zenii-core, presentation logic in core), unused or vestigial abstractions, inconsistent patterns across similar modules, feature-flag boundary correctness, config sprawl. This project follows a strict shared-core architecture: ALL business logic in zenii-core, binary crates are thin shells. Check for violations."
- **all**: "Review all aspects: correctness, security, performance, race conditions, error handling, concurrency, API design, architecture, code quality, test coverage gaps."

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

Categories: SECURITY, PERFORMANCE, LOGIC, ERROR-HANDLING, CONCURRENCY, RACE-CONDITION, API, ARCHITECTURE, CODE-QUALITY, TEST-GAP, STALE-CODE, CONVENTION

If you find no issues, output exactly: NO_ISSUES_FOUND
```

### Step 2: Execute Codex

Based on SCOPE, run one of these commands (15-minute timeout). Capture ALL output.

**uncommitted:**
```bash
timeout 900 codex review --uncommitted "FULL_PROMPT" 2>&1
```

**branch:NAME:**
```bash
timeout 900 codex review --base NAME "FULL_PROMPT" 2>&1
```

**commit:SHA:**
```bash
timeout 900 codex review --commit SHA "FULL_PROMPT" 2>&1
```

**files:p1,p2,...:**
```bash
timeout 900 codex exec "Review these files: p1, p2, ... FULL_PROMPT" --sandbox read-only --ephemeral 2>&1
```

**full:**
```bash
timeout 900 codex exec "Full codebase audit of this Rust+Svelte project. FULL_PROMPT" --sandbox read-only --ephemeral 2>&1
```

If the command times out, continue with whatever partial output was captured and print a warning: "Codex timed out after 15 minutes. Working with partial output."

If the output is empty or clearly garbled, print the raw output and say: "Could not parse Codex output. Raw output shown above. Proceed manually." STOP.

### Step 3: Parse findings

Read the Codex output and extract each finding into a structured list:
- category, title, files, severity, description, suggested_fix, effort

If output contains `NO_ISSUES_FOUND` or no findings are parseable:
Print: "Clean audit — Codex found no issues." STOP.

If `--dry-run` flag was set:
Print all parsed findings in a formatted table and STOP. Do not validate or fix anything.

### Step 4: Validate each finding (Claude's job — the critical gate)

Claude is the architect here, not a rubber stamp. Codex has context poverty — it sees code fragments without understanding project conventions, architectural decisions, or cross-module interactions. Treat every Codex finding as a **hypothesis to verify**, not a fact to act on.

**Default stance: skeptical.** Most Codex findings will be either false positives or technically correct but wrong for this project. Only promote a finding to "Apply" when you have HIGH confidence it's both real AND the right fix.

For EACH finding from Codex, do the following:

#### 4a. Read the actual code (mandatory — no shortcuts)

Use the Read tool to read the file(s) referenced in the finding. Read **at least 50-80 lines** around the referenced location — enough to understand the full function/method, its callers, and its error handling context. If the finding involves cross-module interaction, read the other module too.

**Do NOT skip this step.** Do NOT rely on Codex's description of what the code does. Read it yourself.

#### 4b. Independent assessment (form your own opinion BEFORE comparing to Codex)

Before evaluating Codex's specific claim, form your own understanding of the code:
1. What does this code do? What's its purpose in the larger system?
2. What invariants does it maintain? What assumptions does it rely on?
3. Is there anything you'd flag independently, looking at this code fresh?

Then evaluate Codex's claim against your independent reading:

#### 4c. Rigorous truthfulness check

Run through ALL of these checks — a finding must pass every one to be considered real:

1. **Existence check**: Does the code Codex references actually exist at that location? (Codex frequently hallucinates file paths, line numbers, and function names)
2. **Accuracy check**: Does the code actually do what Codex claims? Read it literally — is Codex misreading the logic, conflating two code paths, or missing context?
3. **Context check**: Is this already handled elsewhere that Codex couldn't see? (e.g., validation at API boundary, error handling in caller, config-driven behavior)
4. **Convention check**: Does this conflict with a CLAUDE.md convention or an intentional project decision? (e.g., Codex might flag `spawn_blocking` as unnecessary, not knowing our SQLite rule)
5. **Severity check**: Even if real, does this actually matter? Is it in a hot path? Is it reachable by users? Could it cause data loss or security issues, or is it cosmetic?
6. **Fix quality check**: Is Codex's suggested fix actually correct? Would it introduce new problems? Does it align with project patterns? (Often the finding is real but the suggested fix is wrong or over-engineered)

#### 4d. Design your own fix (never copy Codex's fix blindly)

If a finding passes the truthfulness check:
- **Design your own fix** based on your understanding of the codebase and CLAUDE.md conventions
- Codex's suggested fix is a hint, not a prescription — it often suggests patterns that violate project conventions or are overly complex
- The fix must be minimal, follow existing patterns in the codebase, and not introduce new abstractions or dependencies
- If the right fix is non-obvious or touches multiple modules, classify as "Defer" or "User Decision" instead of guessing

#### 4e. Classify the finding

| Disposition | Criteria | Action |
|-------------|----------|--------|
| **Apply** | Issue is real (passed ALL checks in 4c), fix is clear AND minimal, low risk, aligns with CLAUDE.md. Claude has designed a specific fix and is confident it won't break anything. | Will be fixed with Claude's fix (not Codex's) |
| **Skip** | Failed any check in 4c. Be specific about which check failed and why. | Won't fix — explain what Codex got wrong |
| **Defer** | Passed truthfulness checks but: fix touches >2 files, requires architectural discussion, has non-obvious side effects, or Claude isn't confident in the right fix | Track for later — explain what needs to be decided first |
| **User Decision** | Issue is real but there are legitimate trade-offs (performance vs. readability, strictness vs. ergonomics, etc.) that depend on user priorities | Present options with Claude's recommendation |

**Bias toward Skip and Defer over Apply.** A wrong fix is worse than no fix. When in doubt, defer.

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
> Claude's independent read: {what Claude found reading the actual code — confirm or nuance the issue}
> Checks passed: {which of the 6 checks in 4c this passed and key evidence}
> Claude's fix (not Codex's): {specific fix Claude designed, explaining how it differs from Codex's suggestion if at all}

### Findings Skipped

For each:
> **[CATEGORY] Title** — file:line
> Codex says: {description}
> Check failed: {which specific check (existence/accuracy/context/convention/severity/fix-quality) failed}
> Evidence: {what Claude actually found in the code that contradicts Codex's claim}

### Findings Deferred

For each:
> **[CATEGORY] Title** (severity) — file:line
> Codex says: {description}
> Claude confirms: {what's real about the finding}
> Deferred because: {why Claude can't confidently fix this now — multi-file impact, needs design discussion, uncertain side effects}
> Suggested next step: {specific action — TODO comment, plan file, or conversation topic}

### Findings Needing User Decision

For each:
> **[CATEGORY] Title** (severity) — file:line
> Codex says: {description}
> Claude's analysis: {what Claude found reading the code independently}
> Option A: {description, pros, cons}
> Option B: {description, pros, cons}
> Claude recommends: {which option and why, but deferring to user}
```

### Step 6: User approval gate

**All findings require user approval before any code is modified.** Present the full report and wait for the user to review it.

#### 6a. Handle "User Decision" findings first

If there are any "User Decision" findings:
- Ask the user for their disposition on each one (apply, skip, or defer)
- Wait for their response before proceeding

#### 6b. Ask permission to apply fixes

After "User Decision" items are resolved, present the final list of all findings marked "Apply" (both from Step 4 and any approved "User Decision" items). Ask the user:

```
Ready to apply N fixes. Proceed? (yes / pick specific ones / no)
```

- **yes**: Apply all "Apply" findings
- **pick**: Let the user specify which findings to apply by number/title
- **no**: Skip all fixes, jump to Step 9

**The `--fix` flag skips this gate** — auto-apply all "Apply" findings without asking. Still ask for "User Decision" items.

If there are NO "Apply" or approved "User Decision" findings, print "No fixes to apply." and skip to Step 9.

### Step 7: Apply fixes (Claude's fixes, not Codex's)

**CRITICAL**: Apply the fix Claude designed in Step 4d, NOT the fix Codex suggested. Codex's suggestions often violate project conventions, introduce unnecessary abstractions, or miss cross-module implications. Claude's fix was designed with full codebase context.

For each approved fix:

1. **Re-read the target code** before editing — the code may have changed from earlier fixes in this batch
2. Edit using the Edit tool, following CLAUDE.md conventions. Keep changes minimal — if a fix starts growing beyond ~10 lines, reconsider whether it should be "Defer" instead
3. After every 3 fixes, run `cargo check --workspace` to catch breakage early
4. If a fix breaks compilation:
   - Immediately revert that specific edit
   - Note it in the report as "Reverted — broke compilation: {error}"
   - Reconsider: was the fix actually correct? Log the lesson.
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
| Codex timeout (>15min) | Continue with partial output + warning |
| Unparseable output | Print raw output, suggest manual review, STOP |
| Fix breaks compilation | Revert that fix, note in report, continue |
| Fix breaks tests | Revert that fix, note in report, continue |
| Frontend-only changes | Skip Rust verification |
| Rust-only changes | Skip bun verification |
| Zero findings | "Clean audit" message, STOP |
