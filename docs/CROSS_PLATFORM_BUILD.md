# Cross-Platform Build Guide

This guide covers building MesoClaw for multiple platforms and architectures using the provided build scripts.

## Quick Start

```bash
# Interactive mode (select targets from menu)
./scripts/cross-platform-build.sh

# Build default targets (Linux x86_64, Windows x86_64, macOS Universal)
./scripts/cross-platform-build.sh --default

# Build all available targets for current platform
./scripts/cross-platform-build.sh --all
```

## Available Scripts

| Script | Purpose | Platform |
|--------|---------|----------|
| `scripts/cross-platform-build.sh` | Main interactive build script | Any |
| `scripts/docker-build.sh` | Docker-based cross-compilation | Linux/macOS |
| `scripts/release.sh` | Version bump and release preparation | Any |

## Supported Targets

### Linux

| Target Name | Rust Triple | Architecture | Bundle Types |
|-------------|-------------|---------------|--------------|
| `linux-x86_64` | `x86_64-unknown-linux-gnu` | 64-bit Intel/AMD | AppImage, deb, rpm |
| `linux-i686` | `i686-unknown-linux-gnu` | 32-bit | AppImage, deb, rpm |
| `linux-aarch64` | `aarch64-unknown-linux-gnu` | ARM64 | AppImage, deb, rpm |
| `linux-armv7` | `armv7-unknown-linux-gnueabihf` | ARMv7 | AppImage, deb, rpm |

### Windows

| Target Name | Rust Triple | Architecture | Bundle Types |
|-------------|-------------|---------------|--------------|
| `windows-x86_64` | `x86_64-pc-windows-msvc` | 64-bit MSVC | MSI, NSIS |
| `windows-i686` | `i686-pc-windows-msvc` | 32-bit MSVC | MSI, NSIS |
| `windows-x86_64-gnu` | `x86_64-pc-windows-gnu` | 64-bit MinGW | NSIS |

### macOS

| Target Name | Rust Triple | Architecture | Bundle Types |
|-------------|-------------|---------------|--------------|
| `macos-x86_64` | `x86_64-apple-darwin` | Intel | DMG, App |
| `macos-aarch64` | `aarch64-apple-darwin` | Apple Silicon | DMG, App |
| `macos-universal` | `universal-apple-darwin` | Universal | DMG, App |

## Usage Examples

### Interactive Mode

Run the script without arguments for an interactive menu:

```bash
./scripts/cross-platform-build.sh
```

You'll see a menu like this:

```
╔════════════════════════════════════════════════════════════╗
║  Tauri 2.x Cross-Platform Build Script                      ║
╚════════════════════════════════════════════════════════════╝

Select target platforms:

  Linux
    1) x86_64 (64-bit Intel/AMD)
    2) i686 (32-bit)
    3) aarch64 (ARM64)
    4) armv7 (ARM32)
    5) All Linux targets

  Windows
    6) x86_64 (64-bit MSVC)
    7) i686 (32-bit MSVC)
    8) x86_64-gnu (64-bit MinGW)
    9) All Windows targets

  macOS
    a) x86_64 (Intel)
    b) aarch64 (Apple Silicon)
    c) universal (Intel + Silicon)
    d) All macOS targets

  Presets
    L) Linux + Windows + macOS (All)
    D) Default (Linux x86_64, Windows x86_64, macOS universal)
```

### Command-Line Mode

Build specific targets:

```bash
# Linux x86_64 only
./scripts/cross-platform-build.sh --targets linux-x86_64

# Multiple specific targets
./scripts/cross-platform-build.sh --targets linux-x86_64,windows-x86_64,macos-universal

# All Linux targets
./scripts/cross-platform-build.sh --linux

# All Windows targets
./scripts/cross-platform-build.sh --windows

# All macOS targets
./scripts/cross-platform-build.sh --macos
```

### Environment Variables

You can also use environment variables:

```bash
# Set targets via environment
TARGETS="linux-x86_64,windows-x86_64" ./scripts/cross-platform-build.sh

# Clean build
CLEAN_BUILD=1 ./scripts/cross-platform-build.sh --default

# Skip frontend build (use existing dist/)
SKIP_FRONTEND=1 ./scripts/cross-platform-build.sh

# Verbose output
VERBOSE=1 ./scripts/cross-platform-build.sh
```

### Build Options

| Option | Short | Description |
|--------|-------|-------------|
| `--targets TARGETS` | `-t` | Comma-separated list of targets |
| `--all` | | Build for all available targets |
| `--linux` | | Build for all Linux targets |
| `--windows` | | Build for all Windows targets |
| `--macos` | | Build for all macOS targets |
| `--default` | | Build default targets |
| `--clean` | | Clean build artifacts before building |
| `--verbose` | `-v` | Enable verbose output |
| `--skip-frontend` | | Skip frontend build |
| `--help` | `-h` | Show help message |

## Cross-Platform Compilation

### Building Windows from Linux

Use Docker for cross-compilation:

```bash
# Build Docker image once
./scripts/docker-build.sh --build-image

# Build Windows binaries
./scripts/docker-build.sh --windows

# Or use docker directly
docker run --rm -v $(pwd):/app -w /app tauri-cross-compile \
    ./scripts/cross-platform-build.sh --windows
```

### Building for Multiple Architectures

```bash
# All Linux targets (x86_64, i686, aarch64, armv7)
./scripts/cross-platform-build.sh --linux

# Docker-based cross-compilation for ARM
./scripts/docker-build.sh --platforms linux
```

## Prerequisites

### Linux

```bash
# Basic dependencies
sudo apt-get install \
    cargo rustc bun nodejs \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libasound2-dev

# Cross-compilation toolchains
sudo apt-get install \
    gcc-multilib \
    g++-multilib \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    gcc-arm-linux-gnueabihf \
    g++-arm-linux-gnueabihf

# Windows cross-compilation (MinGW)
sudo apt-get install mingw-w64 nsis
```

### macOS

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Bun
curl -fsSL https://bun.sh/install | bash

# Xcode command line tools (for macOS builds)
xcode-select --install
```

### Windows

```bash
# Install Rust from https://rustup.rs
# Install Bun from https://bun.sh
# Install Visual Studio Build Tools for MSVC builds
```

## CI/CD Integration

The project includes GitHub Actions workflows for automated builds:

- `.github/workflows/release.yml` - Release builds for all platforms
- `.github/workflows/ci.yml` - Continuous integration

The CI automatically builds for:
- macOS (Apple Silicon, Intel, Universal)
- Windows (x64)
- Linux (Ubuntu 22.04 deb, Ubuntu 24.04 AppImage + RPM)

## Output Artifacts

Build artifacts are placed in:

```
src-tauri/target/
├── release/
│   └── bundle/
│       ├── appimage/      # Linux .AppImage files
│       ├── deb/           # Linux .deb packages
│       ├── rpm/           # Linux .rpm packages
│       ├── msi/           # Windows .msi installers
│       ├── nsis/          # Windows .exe installers
│       ├── macos/         # macOS .dmg files
│       └── dmg/           # Alternative macOS .dmg location
└── universal-apple-darwin/
    └── release/           # macOS universal binary
```

## Troubleshooting

### Missing Rust Targets

If you see errors about missing targets, install them:

```bash
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-msvc
rustup target add aarch64-apple-darwin
# etc.
```

### Linux Cross-Compilation Issues

Ensure you have the correct cross-toolchain:

```bash
# For 32-bit
sudo apt-get install gcc-multilib g++-multilib

# For ARM64
sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu

# For ARMv7
sudo apt-get install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf
```

### Windows Cross-Compilation from Linux

Use the Docker-based build:

```bash
./scripts/docker-build.sh --windows
```

### macOS Codesigning

For release builds, you need:

1. Apple Developer certificate
2. Set in `src-tauri/tauri.conf.json`:
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)",
     "hardenedRuntime": true
   }
   ```

### Build Failures

```bash
# Clean build
./scripts/cross-platform-build.sh --clean

# Check Rust version
rustc --version

# Update Rust
rustup update

# Clear cargo cache
cargo clean
```

## Advanced: GitHub Actions for All Platforms

For full cross-platform builds including macOS, use GitHub Actions:

```bash
# Push to trigger CI
git push origin main

# Or manually trigger from GitHub Actions tab
# Workflow: .github/workflows/release.yml
```

The release workflow builds:
- macOS (Apple Silicon, Intel, Universal) with code signing
- Windows (x64) with code signing
- Linux (deb, AppImage, rpm)

See `.github/workflows/release.yml` for configuration.
