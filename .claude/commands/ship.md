# Format, Scan Secrets, Commit & Push to Main

This command formats the code, scans for leaked secrets, commits, and pushes to the remote main branch.
**It will REFUSE to proceed if any secrets or tokens are detected.**

## Instructions

Follow these steps strictly and sequentially. Stop immediately if any step fails.

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

### Step 3: Run lints

Run `cargo clippy --workspace` to catch lint issues. If there are warnings or errors, fix them before proceeding.

### Step 4: Secret scan (BLOCKING)

This is the critical security gate. Scan ALL staged and unstaged tracked files for leaked secrets.

#### 4a. Stage all formatted changes first
Run `git add -A` to stage everything, then immediately run `git diff --cached --no-color` to get the full diff.

#### 4b. Scan the diff output for these patterns (MUST check ALL):

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

#### 4c. Also scan ALL tracked files (not just diff) for the same patterns:
Use grep across the entire repo (excluding `.git/`, `target/`, `node_modules/`, `*.lock` files).

#### 4d. If ANY secret is detected:
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

#### 4e. If scan is clean:
Print: "Secret scan passed - no leaked credentials detected."

### Step 5: Commit

- Run `git status` to review what will be committed
- Run `git diff --cached --stat` for a summary
- Create a commit with a descriptive message based on the actual changes:

```
git commit -m "$(cat <<'EOF'
<descriptive message based on changes>

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

### Step 6: Push to remote main

- First run `git pull --rebase origin main` to sync
- Then run `git push origin main`
- Report the result to the user

### Step 7: Summary

Print a summary:
```
Ship complete:
- Formatted: cargo fmt + prettier
- Linted: cargo clippy
- Secret scan: PASSED
- Committed: <commit hash> <commit message>
- Pushed to: origin/main
```
