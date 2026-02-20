# Releasing MesoClaw

This guide covers version management, code signing setup, the CI/CD release pipeline, and manual build instructions for the MesoClaw desktop application (Tauri 2).

---

## Table of Contents

1. [Version Management](#version-management)
2. [Code Signing Setup](#code-signing-setup)
   - [macOS](#macos)
   - [Windows](#windows)
   - [Linux](#linux)
3. [CI/CD Release Pipeline](#cicd-release-pipeline)
4. [GitHub Secrets Reference](#github-secrets-reference)
5. [Manual Build Instructions](#manual-build-instructions)
6. [Troubleshooting](#troubleshooting)

---

## Version Management

The `scripts/release.sh` script keeps the version synchronized across three files:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

### Commands

```bash
# Check that all three files are in sync
./scripts/release.sh status

# Bump the patch component (0.1.0 -> 0.1.1)
./scripts/release.sh patch

# Bump the minor component (0.1.0 -> 0.2.0)
./scripts/release.sh minor

# Bump the major component (0.1.0 -> 1.0.0)
./scripts/release.sh major

# Set an explicit version
./scripts/release.sh set 1.2.3

# Sync all files to the current package.json version (no commit)
./scripts/release.sh sync
```

### What the Script Does

When you run `patch`, `minor`, or `major`, the script:

1. Reads the current version from `package.json`.
2. Increments it according to the bump type.
3. Writes the new version to all three config files.
4. Creates a git commit with the message `chore(release): vX.Y.Z` and a changelog of commits since the last tag.
5. Creates an annotated git tag `app-vX.Y.Z`.
6. Pushes the commit and tag to the current branch.
7. Offers to push to the `release` branch, which triggers the GitHub Actions workflow.

### Triggering CI from the Release Branch

The GitHub Actions release workflow is triggered manually via `workflow_dispatch`. After running the release script:

- If you are already on the `release` branch, CI starts immediately after push.
- If you are on another branch (e.g., `main` or `master`), the script will ask whether to push to `release`:

```bash
git push origin master:release
```

Alternatively, trigger the workflow manually from the GitHub Actions tab using the "Run workflow" button.

---

## Code Signing Setup

### macOS

macOS code signing requires an Apple Developer account enrolled in the Apple Developer Program (paid membership).

#### Certificate Setup

1. Open Xcode or Keychain Access and create a **Developer ID Application** certificate.
2. Export the certificate as a `.p12` file with a password.
3. Base64-encode the `.p12` for use as a GitHub secret:

```bash
base64 -i DeveloperID.p12 | pbcopy
```

4. Store the result as `APPLE_CERTIFICATE` in GitHub Secrets.
5. Store the `.p12` password as `APPLE_CERTIFICATE_PASSWORD`.

#### Notarization Setup

Apple notarization submits the signed app to Apple's servers for malware scanning. Without it, Gatekeeper will block the app on other machines.

You need an **app-specific password** for your Apple ID:

1. Go to [appleid.apple.com](https://appleid.apple.com) → Security → App-Specific Passwords.
2. Generate a password with a label like `mesoclaw-notarize`.
3. Store the following secrets in GitHub:

| Secret | Value |
|--------|-------|
| `APPLE_ID` | Your Apple ID email (e.g., `dev@example.com`) |
| `APPLE_ID_PASSWORD` | The app-specific password you generated |
| `APPLE_TEAM_ID` | Your 10-character Apple Team ID (e.g., `6BQSGY2B74`) |

The Team ID appears in the Apple Developer portal under Membership Details.

#### Bundle Configuration

The signing identity and entitlements are configured in `src-tauri/tauri.conf.json`:

```json
"macOS": {
  "signingIdentity": "Developer ID Application: NSR Technologies Inc. (6BQSGY2B74)",
  "entitlements": "./entitlements.plist",
  "minimumSystemVersion": "11.0",
  "providerShortName": "6BQSGY2B74",
  "hardenedRuntime": true
}
```

The `entitlements.plist` at `src-tauri/entitlements.plist` grants the following permissions (required for Hardened Runtime):

- `com.apple.security.cs.allow-jit` - JIT compilation (needed by WebKit)
- `com.apple.security.cs.allow-unsigned-executable-memory`
- `com.apple.security.cs.disable-library-validation`
- `com.apple.security.network.client` / `.server` - Network access
- `com.apple.security.files.user-selected.read-write` - File access

#### Local macOS Signing (Development)

To sign locally without CI:

```bash
# Verify your certificate is installed
security find-identity -v -p codesigning

# Build with signing (credentials read from Keychain automatically)
bun run tauri build --target aarch64-apple-darwin
```

Notarization during local builds requires the same environment variables listed in the GitHub Secrets section, set in your shell.

---

### Windows

#### Option A: Azure Trusted Signing (Recommended for CI)

Azure Trusted Signing provides cloud-based signing without managing a local certificate file. It is the recommended approach for Windows code signing in CI/CD pipelines.

1. Create an Azure account and set up a Trusted Signing account.
2. Create a certificate profile (Organization Validated or Extended Validation).
3. Install the `azure-codesign` action or use `signtool` with Azure credentials.

Required environment variables for CI:

| Variable | Description |
|----------|-------------|
| `AZURE_TENANT_ID` | Azure Active Directory tenant ID |
| `AZURE_CLIENT_ID` | Service principal / app registration client ID |
| `AZURE_CLIENT_SECRET` | Service principal secret |
| `AZURE_ENDPOINT` | Trusted Signing endpoint URL |
| `AZURE_CODE_SIGNING_NAME` | Trusted Signing account name |
| `AZURE_CERT_PROFILE_NAME` | Certificate profile name |

Note: Azure Trusted Signing is not currently wired into the `release.yml` workflow. Add a signing step before the Tauri build action for the Windows matrix entry if needed.

#### Option B: Local PFX Certificate

For a self-managed certificate (e.g., from DigiCert or Sectigo):

1. Obtain a Code Signing certificate as a `.pfx` file.
2. Base64-encode it:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("cert.pfx")) | Set-Clipboard
```

3. Store as `WINDOWS_CERTIFICATE` in GitHub Secrets.
4. Store the PFX password as `WINDOWS_CERTIFICATE_PASSWORD`.

The Tauri action can use these environment variables directly when the `tauri.conf.json` Windows bundle section is configured to point to the certificate.

#### No Signing (Development Builds)

Windows builds without a certificate produce unsigned installers. Users will see a SmartScreen warning on first launch. This is acceptable for internal testing but not for public distribution.

---

### Linux

Linux packages (AppImage, `.deb`, `.rpm`) do not require code signing. The CI workflow builds:

- **Ubuntu 22.04**: `.deb` package
- **Ubuntu 24.04**: AppImage and `.rpm` package

No additional configuration is needed for Linux releases.

Optional: If you distribute via a repository (e.g., apt or RPM repo), sign the repository index with a GPG key. This is separate from the Tauri build process.

---

## CI/CD Release Pipeline

The release workflow is defined in `.github/workflows/release.yml`.

### Trigger

The workflow is triggered manually via `workflow_dispatch` with one input:

- `draft` (boolean, default `true`): When checked, the GitHub Release is created as a draft. Uncheck to publish immediately.

To run: go to **Actions** tab in GitHub, select **Release**, click **Run workflow**.

### Jobs

#### Job 1: `create-release`

Runs on `ubuntu-latest`.

1. Reads the version from `src-tauri/tauri.conf.json`.
2. Creates a GitHub Release (draft by default) tagged `vX.Y.Z` using `softprops/action-gh-release`.
3. Outputs the release ID and version to downstream jobs.

#### Job 2: `build`

Runs in parallel across six matrix entries after `create-release` succeeds.

| Matrix Entry | Runner | Output |
|---|---|---|
| macOS (Apple Silicon) | `macos-latest` | `.dmg` for `aarch64-apple-darwin` |
| macOS (Intel) | `macos-latest` | `.dmg` for `x86_64-apple-darwin` |
| macOS (Universal) | `macos-latest` | Universal `.dmg` |
| Windows (x64) | `windows-latest` | `.msi` / `.exe` for `x86_64-pc-windows-msvc` |
| Linux Ubuntu 22.04 | `ubuntu-22.04` | `.deb` |
| Linux Ubuntu 24.04 | `ubuntu-24.04` | AppImage + `.rpm` |

Each build job:

1. Installs Rust stable with the required targets.
2. Sets up Rust build caching via `swatinem/rust-cache`.
3. Installs Bun and runs `bun install --frozen-lockfile`.
4. On Linux, installs required system packages (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, etc.).
5. Runs `tauri-apps/tauri-action` which builds the frontend, compiles the Rust backend, bundles the app, signs it (macOS), and uploads artifacts to the GitHub Release.

### Artifact Upload

The `tauri-action` automatically uploads built artifacts to the GitHub Release created in job 1. No manual upload step is needed.

---

## GitHub Secrets Reference

Configure these secrets in your repository under **Settings > Secrets and variables > Actions**.

### Required for All Releases

| Secret | Description |
|--------|-------------|
| `GITHUB_TOKEN` | Auto-provided by GitHub Actions. No setup needed. |

### Required for macOS Signing and Notarization

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` Developer ID Application certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_ID_PASSWORD` | App-specific password for the Apple ID |
| `APPLE_TEAM_ID` | 10-character Apple Developer Team ID (e.g., `6BQSGY2B74`) |

### Required for Tauri Updater Signing

The Tauri updater uses a separate key pair to sign update payloads. This is distinct from OS-level code signing.

| Secret | Description |
|--------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key generated by `tauri signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the private key (can be empty) |

Generate the key pair locally:

```bash
# Install Tauri CLI if not already installed
cargo install tauri-cli

# Generate signing keys
cargo tauri signer generate -w ~/.tauri/mesoclaw.key
```

Copy the private key content to the `TAURI_SIGNING_PRIVATE_KEY` secret. Store the public key in `tauri.conf.json` under `plugins.updater.pubkey` if you enable the updater plugin.

---

## Manual Build Instructions

### Prerequisites

- Rust stable toolchain (`rustup toolchain install stable`)
- Bun (`curl -fsSL https://bun.sh/install | bash`)
- On Linux: system packages listed in the CI workflow
- On macOS: Xcode Command Line Tools (`xcode-select --install`)

### Build for the Current Platform

```bash
# From the project root
bun install
bun run tauri build
```

Output artifacts are placed in `src-tauri/target/release/bundle/`.

### Build for a Specific macOS Target

```bash
# Apple Silicon
rustup target add aarch64-apple-darwin
bun run tauri build --target aarch64-apple-darwin

# Intel
rustup target add x86_64-apple-darwin
bun run tauri build --target x86_64-apple-darwin

# Universal binary
rustup target add aarch64-apple-darwin x86_64-apple-darwin
bun run tauri build --target universal-apple-darwin
```

### Build Specific Linux Bundles

```bash
bun run tauri build --bundles deb
bun run tauri build --bundles appimage
bun run tauri build --bundles rpm
bun run tauri build --bundles appimage,rpm
```

### Build Without Signing (Skip Signing)

To produce unsigned binaries for local testing, unset the signing environment variables before building or omit `signingIdentity` from `tauri.conf.json` temporarily.

---

## Troubleshooting

### macOS: "The application cannot be opened because its integrity cannot be verified"

The app is unsigned or notarization failed. Check that:

- `APPLE_CERTIFICATE` is a valid base64-encoded `.p12`.
- `APPLE_ID_PASSWORD` is an app-specific password (not your Apple ID password).
- `APPLE_TEAM_ID` matches the Team ID in the `signingIdentity` field.
- The certificate has not expired (`security find-identity -v -p codesigning`).

### macOS: Notarization Timeout

Apple's notarization service can take several minutes. The `tauri-action` waits for the notarization response. If it times out in CI, re-run the workflow. For persistent failures, check the Apple Developer status page.

### Windows: SmartScreen Warning on Launch

The `.exe` or `.msi` is unsigned. Add a valid code signing certificate. SmartScreen warnings disappear once the file has sufficient reputation, but an EV certificate bypasses this immediately.

### Version Files Out of Sync

If `scripts/release.sh status` shows mismatched versions:

```bash
./scripts/release.sh sync
```

This updates `Cargo.toml` and `tauri.conf.json` to match `package.json` without creating a commit.

### Cargo.lock Not Committed

The `Cargo.lock` file should be committed for reproducible builds. If the CI build fails with dependency resolution errors, ensure `Cargo.lock` is tracked in git:

```bash
git add Cargo.lock
git commit -m "chore: commit Cargo.lock"
```

### Linux Build Missing System Dependencies

If the build fails on Linux with missing headers, install the required packages:

```bash
sudo apt-get update && sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libasound2-dev
```

### Tauri Updater Key Mismatch

If the updater plugin rejects an update signature, verify that `TAURI_SIGNING_PRIVATE_KEY` in CI matches the public key stored in `tauri.conf.json`. Regenerate the key pair and update both if they have drifted.
