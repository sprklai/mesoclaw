# Format, Update Docs, Scan Secrets, Commit & Push to Main

This command formats the code, updates documentation to reflect changes, scans for leaked secrets, commits, and pushes to the remote main branch.
**It will REFUSE to proceed if any secrets or tokens are detected.**
**It runs fully automatically with NO user prompts unless secrets are found.**

## Instructions

Follow these steps strictly and sequentially. Stop immediately if any step fails. Do NOT ask the user for confirmation between steps — proceed automatically unless a secret is detected.

### Step 1: Verify branch

Run `git branch --show-current` to confirm we are on the `main` branch.
If NOT on main, STOP and tell the user: "You are on branch X, not main. Switch to main first or confirm you want to push from this branch."

### Step 2: Format the code

Run the following formatters:
```
cargo fmt --all
```
If a `web/` directory exists with a `package.json`, also run:
```
cd web && bun run format 2>/dev/null || npx prettier --write . 2>/dev/null; cd -
```

### Step 3: Check i18n

If a `web/` directory exists with `project.inlang/settings.json`, run:
```
cd web && bun run check
```
This runs `svelte-kit sync && svelte-check` which validates paraglide i18n message keys, types, and compiled output. If there are errors (missing message keys, type mismatches, stale paraglide output), fix them before proceeding. Do NOT ask the user — just fix and re-run.

### Step 4: Run lints

Run `cargo clippy --workspace` to catch lint issues. If there are warnings or errors, fix them before proceeding. Do NOT ask the user — just fix and re-run.

### Step 5: Update documentation

Review the staged and unstaged changes (use `git diff` and `git diff --cached`) to understand what has changed since the last commit. Then update the following documentation files **only if the code changes warrant it** — do not make gratuitous edits:

1. **`README.md`** — Update if there are new features, removed features, changed commands, new dependencies, or altered project structure.
2. **`CHANGELOG.md`** — Add entries for any user-facing changes (features, fixes, breaking changes). Follow the existing format and group under the current unreleased version section. Create a new section if needed.
3. **`website-data.json`** — Sync derived metrics with the current codebase state:
   - `version`: read from `grep -m1 '^version = "' Cargo.toml | sed 's/.*"\(.*\)".*/\1/'`
   - `gatewayEndpoints`: count via `grep -c '\.route(' crates/zenii-core/src/gateway/routes.rs`
   - `agentTools`: count via `grep -c 'registry\.register\b' crates/zenii-core/src/boot.rs`
   - `backendTests`: count via `cargo test --workspace 2>&1 | awk '/test result/{sum+=$4} END{print sum}'`
   - Other fields (`binarySize`, `debRpmSize`, `startupSeconds`, `llmProviders`, `messagingChannels`, `crates`, `securityLayers`, `platformTargets`, `compileFlags`) — update manually only if the code changes warrant it.
   - Read the current `website-data.json`, apply only the fields that changed, write back.
   - After writing `website-data.json`, read `backendTests` from it and update the test count badge in `README.md`: replace `tests-<N>-blue` with `tests-<backendTests>-blue` and `alt="<N> tests"` with `alt="<backendTests> tests"`.
4. **`docs/` directory** — Update relevant docs if the changes affect:
   - `docs/architecture.md` — new modules, traits, data flows, or structural changes
   - `docs/phases.md` — phase status changes or deliverable updates
   - `docs/processes.md` — process flow changes
   - `docs/cli-reference.md` — new or changed CLI commands/flags
   - Any other doc that describes behavior that has changed

**Rules:**
- Only update files where the code changes actually affect documented behavior. If nothing changed that impacts docs, skip this step entirely.
- Keep changes minimal and accurate — reflect what actually changed, don't embellish.
- Do NOT create new documentation files — only update existing ones.
- Do NOT ask the user — just make the updates and proceed.

### Step 6: Secret scan (BLOCKING)

This is the critical security gate. Scan ALL staged and unstaged tracked files for leaked secrets.

#### 6a. Stage all formatted changes first
Run `git add -A` to stage everything, then immediately run `git diff --cached --no-color` to get the full diff.

#### 6b. Scan the diff output for these patterns (MUST check ALL):

| Type | Pattern |
|------|---------|
| OpenAI API Key | `sk-[a-zA-Z0-9]{20}` |
| Anthropic API Key | `sk-ant-` |
| AWS Access Key | `AKIA[0-9A-Z]{16}` |
| GitHub Token | `ghp_[a-zA-Z0-9]{36}` or `gho_` or `ghu_` or `ghs_` or `ghr_` |
| Google API Key | `AIza[0-9A-Za-z\-_]{35}` |
| Slack Token | `xox[baprs]-` |
| Discord Bot Token | `[MN][a-zA-Z\d]{23}\.[a-zA-Z\d]{6}\.[a-zA-Z\d]{38}` |
| Telegram Bot Token | `\d{9,10}:[a-zA-Z0-9_-]{35}` |
| Stripe Key | `sk_live_` or `pk_live_` |
| Private Key | `-----BEGIN .* PRIVATE KEY-----` |
| JWT | `eyJ[a-zA-Z0-9_-]*\.eyJ` |
| Generic API Key | `(?i)(api[_-]?key\|apikey)\s*[=:]\s*['"]?[a-zA-Z0-9_\-]{20,}` |
| Password Assignment | `(?i)(password\|passwd\|pwd)\s*[=:]\s*['"]?[^'"\s]{8,}` |
| Generic Secret/Token | `(?i)(secret\|token)\s*[=:]\s*['"]?[a-zA-Z0-9_\-]{20,}` |
| Auth Header | `(?i)authorization\s*:\s*(bearer\|basic)\s+[a-zA-Z0-9_\-\.]+` |
| Connection String | `(?i)(mongodb\|postgres\|mysql\|redis)://[^\s'"]+:[^\s'"]+@` |
| Bot ID / Channel Token | `(?i)(bot[_-]?id\|bot[_-]?token\|channel[_-]?token\|chat[_-]?id)\s*[=:]\s*['"]?[a-zA-Z0-9_\-:]{8,}` |

#### 6c. Also scan ALL tracked files (not just diff) for the same patterns:
Use grep across the entire repo (excluding `.git/`, `target/`, `node_modules/`, `*.lock` files).

#### 6d. If ANY secret is detected:
- Run `git reset HEAD` to unstage everything
- Output the alert in this EXACT format:

```
SECRET DETECTED - PUSH BLOCKED

File: <filepath>:<line>
Type: <secret type>
Match: <first 8 chars>...<last 4 chars> (masked)

ACTION REQUIRED:
1. Remove the secret from the file
2. Use environment variables instead
3. If this was already committed, rotate the key immediately
```

- STOP. Do NOT commit. Do NOT push. This is non-negotiable.
- This is the ONLY case where user input is required before proceeding.

#### 6e. If scan is clean:
Print: "Secret scan passed - no leaked credentials detected."
Proceed immediately to Step 7 without asking for confirmation.

### Step 7: Commit

- Run `git diff --cached --stat` for a summary
- Create a commit with a descriptive message based on the actual changes. Do NOT ask the user for a commit message — generate one automatically:

```
git commit -m "$(cat <<'EOF'
<descriptive message based on changes>

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

### Step 8: Push to remote main

- First run `git pull --rebase origin main` to sync
- Then run `git push origin main`
- Do NOT ask for confirmation — just push.

### Step 9: Summary

Print a summary:
```
Ship complete:
- Formatted: cargo fmt + prettier
- i18n check: PASSED
- Linted: cargo clippy
- Docs updated: <list files updated, or "none needed"> (including website-data.json if metrics changed)
- Secret scan: PASSED
- Committed: <commit hash> <commit message>
- Pushed to: origin/main
```
