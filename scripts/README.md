# Scripts & Commands

## Shell Scripts

### `build.sh` -- Cross-Platform Build Script

Builds MesoClaw binaries for any supported platform, with optional Tauri desktop bundling.

```bash
# Build native debug binaries
./scripts/build.sh --target native

# Build release binaries for Linux x86
./scripts/build.sh --target linux-x86 --release

# Build Tauri desktop app with .deb and .AppImage bundles
./scripts/build.sh --tauri --bundle deb,appimage --release

# Build only the daemon with all features
./scripts/build.sh --target native --release --crates mesoclaw-daemon --all-features

# List all available targets
./scripts/build.sh --list-targets
```

**Targets**: `native`, `linux-x86`, `linux-arm64`, `linux-armv7`, `linux-musl`, `macos-x86`, `macos-arm`, `macos-universal`, `windows`, `all`

---

### `docker-build.sh` -- Docker-Based Cross-Compilation

Cross-compiles binaries using Docker containers. Useful for building Linux/Windows targets from any host OS.

```bash
# Cross-compile for Linux ARM64
./scripts/docker-build.sh --target linux-arm64

# Cross-compile for Windows from Linux
./scripts/docker-build.sh --target windows --profile ci-release

# Build specific crates only
./scripts/docker-build.sh --target linux-x86 --crates "mesoclaw-cli mesoclaw-daemon"
```

**Targets**: `linux-x86`, `linux-arm64`, `linux-armv7`, `windows`

---

### `version-bump.sh` -- Semantic Version Synchronization

Bumps the version across all 7 project files and updates the CHANGELOG header.

```bash
# Patch bump: 0.1.0 -> 0.1.1
./scripts/version-bump.sh patch

# Minor bump: 0.1.0 -> 0.2.0
./scripts/version-bump.sh minor

# Major bump: 0.1.0 -> 1.0.0
./scripts/version-bump.sh major

# Set to a specific version
./scripts/version-bump.sh set 2.0.0
```

**Files updated**: `Cargo.toml`, `tauri.conf.json`, `package.json`, `environment.ts`, `identity/types.rs`, `IDENTITY.md`, `CHANGELOG.md`

---

### `release.sh` -- Legacy Release Script

Syncs a specific version across project files and creates a git tag. Superseded by the `/release-tag` Claude command which provides a more complete workflow.

```bash
./scripts/release.sh 1.2.0
./scripts/release.sh --dry-run 1.2.0
```

---

### `quality-check.sh` -- Local Quality Gate

Runs all quality checks locally: formatting, linting, tests, and banned pattern detection. Exits 0 only if everything passes.

```bash
./scripts/quality-check.sh
```

**Checks**: `cargo fmt --check`, `cargo clippy`, `cargo test`, banned patterns (e.g., `std::sync::Mutex` in async code, `println!` in library code)

---

### `smoke-test.sh` -- Binary Smoke Tests

Validates a compiled binary: checks architecture, file size, and basic execution.

```bash
# Basic smoke test
./scripts/smoke-test.sh target/release/mesoclaw-daemon

# With architecture and size constraints
./scripts/smoke-test.sh target/release/mesoclaw-daemon --expected-arch x86-64 --max-size 50
```

---

## Claude Code Commands

These are slash commands you can run inside Claude Code sessions.

### `/ship` -- Format, Scan, Commit & Push

Automated shipping pipeline: formats code, checks i18n, runs clippy, scans for leaked secrets, commits, and pushes to `main`. Blocks if secrets are detected.

```
/ship
```

**Flow**: format -> i18n check -> clippy -> secret scan -> commit -> push

---

### `/release-tag` -- Full Release Workflow

Orchestrates the entire release: version bump, CHANGELOG update, Cargo.lock generation, secret scan, commit, tag, and push to trigger GitHub Actions release builds for all platforms.

```
/release-tag patch
/release-tag minor --message "Added channel integrations"
/release-tag major --dry-run
```

**Arguments**:
- `patch | minor | major` (required) -- semantic version bump type
- `--message "..."` (optional) -- custom CHANGELOG notes; auto-generates from commits if omitted
- `--dry-run` (optional) -- preview the release plan without executing

**Flow**: clean check -> version bump -> CHANGELOG -> Cargo.lock -> secret scan -> commit -> tag -> push -> GitHub Actions builds all platforms
