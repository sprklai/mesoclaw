# Release Tag: Version Bump, Commit, Tag & Push

This command orchestrates the full release workflow: version bump across all files, CHANGELOG update, Cargo.lock generation, secret scan, commit, tag, and push to trigger GitHub Actions release builds.

**It runs fully automatically with NO user prompts unless secrets are found or --dry-run is specified.**

## Arguments

```
$ARGUMENTS
```

Parse the arguments string for:
- **Bump type** (REQUIRED): `patch`, `minor`, or `major`
- **--message "..."** (optional): Custom CHANGELOG/release notes. If omitted, auto-generate from commits since last tag.
- **--dry-run** (optional): Show what would happen without executing any changes.

If no bump type is provided, STOP and print usage:
```
Usage: /release-tag <patch|minor|major> [--message "Release notes"] [--dry-run]

Examples:
  /release-tag patch
  /release-tag minor --message "Added channel integrations"
  /release-tag major --dry-run
```

## Instructions

Follow these steps strictly and sequentially. Stop immediately if any step fails. Do NOT ask the user for confirmation between steps (except for secret detection) — proceed automatically.

### Step 1: Verify clean state

Run `git status --porcelain` to check for uncommitted changes.

If there ARE uncommitted changes, STOP and tell the user:
```
RELEASE BLOCKED: Uncommitted changes detected.

Please commit or stash your changes before releasing:
  git stash        # to stash changes
  git commit -am "..." # to commit changes
```

Also run `git branch --show-current` to confirm we are on the `main` branch.
If NOT on main, STOP and tell the user: "You are on branch X, not main. Switch to main before releasing."

### Step 2: Determine versions

Run the version-bump script in a subshell or read the current version from `Cargo.toml` to determine:
- **Current version**: from `grep -m1 '^version = "' Cargo.toml`
- **New version**: computed from the bump type

If `--dry-run` is set, print the plan and STOP after this step:
```
DRY RUN — Release plan:

  Version:   <current> -> <new>
  Tag:       app-v<new>
  Bump type: <patch|minor|major>
  CHANGELOG: <custom message or "auto-generated from commits">

  Steps that would execute:
  1. Run scripts/version-bump.sh <bump-type> (updates 7 files + CHANGELOG header)
  2. Update CHANGELOG body with release notes
  3. Run cargo generate-lockfile
  4. Secret scan all staged changes
  5. Commit: "release: v<new>"
  6. Tag: app-v<new>
  7. Push commit + tag to origin/main
  8. GitHub Actions builds: Linux (deb/rpm/AppImage), macOS (universal DMG), Windows (msi/exe), Embedded (arm64/armv7/musl)

No changes were made.
```

### Step 3: Bump version

Run the version-bump script:
```bash
./scripts/version-bump.sh <patch|minor|major>
```

This updates 7 files (Cargo.toml, tauri.conf.json, package.json, environment.ts, identity/types.rs, IDENTITY.md, CHANGELOG.md header) and verifies them.

If the script exits with a non-zero code, STOP and report the error.

### Step 4: Update CHANGELOG body

Read the new version that was set (from Cargo.toml after the bump).

**If `--message` was provided:**
- Open `CHANGELOG.md` and insert the custom message text under the new version header that was just created by version-bump.sh.
- Format it as a bullet list if it isn't already.

**If `--message` was NOT provided:**
- Find the last git tag: `git describe --tags --abbrev=0 2>/dev/null`
- Generate commit log: `git log --oneline <last-tag>..HEAD` (or `git log --oneline` if no previous tag exists)
- Insert the commit summaries as bullet points under the new version header in CHANGELOG.md.
- Group by prefix if possible (feat:, fix:, chore:, etc.), otherwise list as-is.

### Step 5: Generate Cargo.lock

Run:
```bash
cargo generate-lockfile
```

This ensures the lockfile reflects the updated version.

### Step 6: Secret scan (BLOCKING)

This is the critical security gate. Scan ALL changes for leaked secrets.

#### 6a. Stage all changes
```bash
git add -A
```

Then run `git diff --cached --no-color` to get the full diff of staged changes.

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
SECRET DETECTED - RELEASE BLOCKED

File: <filepath>:<line>
Type: <secret type>
Match: <first 8 chars>...<last 4 chars> (masked)

ACTION REQUIRED:
1. Remove the secret from the file
2. Use environment variables instead
3. If this was already committed, rotate the key immediately
```

- STOP. Do NOT commit. Do NOT tag. Do NOT push. This is non-negotiable.
- This is the ONLY case where user input is required before proceeding.

#### 6e. If scan is clean:
Print: "Secret scan passed - no leaked credentials detected."
Proceed immediately to Step 7 without asking for confirmation.

### Step 7: Commit

Create the release commit:
```bash
git commit -m "$(cat <<'EOF'
release: v<VERSION>

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

Replace `<VERSION>` with the actual new version number.

### Step 8: Tag

Create the release tag:
```bash
git tag app-v<VERSION>
```

Replace `<VERSION>` with the actual new version number.

### Step 9: Push

Push both the commit and the tag:
```bash
git push origin main && git push origin app-v<VERSION>
```

This triggers `.github/workflows/release.yml` which builds all platform targets automatically:
- Linux: `.deb`, `.rpm`, `.AppImage` + standalone binaries
- macOS: `.dmg` (universal) + universal binaries
- Windows: `.msi`, `.exe` (NSIS) + standalone binaries
- Embedded: ARM64, ARMv7, musl standalone binaries
- All: `SHA256SUMS.txt` checksums

### Step 10: Summary

Print a summary:
```
Release complete:

  Version:    <old> -> <new>
  Tag:        app-v<new>
  Commit:     <short hash>
  Secret scan: PASSED

  GitHub Actions will now build release artifacts for all platforms.
  Monitor: https://github.com/sprklai/mesoclawv2/actions

  Artifacts (when complete):
    Linux:    .deb, .rpm, .AppImage + mesoclaw + mesoclaw-daemon
    macOS:    .dmg (universal) + mesoclaw + mesoclaw-daemon
    Windows:  .msi, .exe (NSIS) + mesoclaw.exe + mesoclaw-daemon.exe
    Embedded: mesoclaw + mesoclaw-daemon (arm64, armv7, musl)
    Checksums: SHA256SUMS.txt
```
