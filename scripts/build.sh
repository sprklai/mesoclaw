#!/usr/bin/env bash
# MesoClaw -- Cross-Platform Build Script
# Usage: ./scripts/build.sh [OPTIONS]
#
# Options:
#   --target <TARGET>     Build target (native|linux-x86|linux-arm64|linux-armv7|linux-musl|
#                                       macos-x86|macos-arm|macos-universal|windows|all)
#   --release             Build in release mode (default: debug)
#   --profile <PROFILE>   Cargo profile (release|ci-release|release-fast) (overrides --release)
#   --crates <CRATES>     Space-separated list of crates to build (default: all binary crates)
#   --features <FEATURES> Comma-separated features to enable
#   --all-features        Enable all features
#   --list-targets        List available build targets and exit
#   --install-toolchain   Install the required Rust target toolchain
#   --tauri               Build Tauri desktop app (native platform only)
#   --bundle <FORMATS>    Comma-separated bundle formats (e.g., deb,appimage,dmg,msi,nsis)
#   --dev                 Start dev mode (Vite + Tauri dev server)
#   --docker              Use Docker-based cross-compilation via Dockerfile.cross-compile
#   --help                Show this help message

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────
WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${WORKSPACE_ROOT}/dist"

# All binary crates in the workspace
ALL_CRATES="mesoclaw-cli mesoclaw-tui mesoclaw-daemon mesoclaw-desktop"

# Target mappings: friendly name -> Rust triple
# Using a function instead of associative arrays for macOS Bash 3.2 compatibility
get_rust_target() {
    case "$1" in
        linux-x86)      echo "x86_64-unknown-linux-gnu";;
        linux-arm|linux-arm64) echo "aarch64-unknown-linux-gnu";;
        linux-armv7)    echo "armv7-unknown-linux-gnueabihf";;
        linux-musl)     echo "aarch64-unknown-linux-musl";;
        macos-x86)      echo "x86_64-apple-darwin";;
        macos-arm)      echo "aarch64-apple-darwin";;
        macos-universal) echo "universal-apple-darwin";;
        windows)        echo "x86_64-pc-windows-gnu";;
        *)              return 1;;
    esac
}

ALL_TARGETS="linux-x86 linux-arm64 linux-armv7 linux-musl macos-x86 macos-arm windows"

# ── Defaults ───────────────────────────────────────────────────────────
TARGET="native"
PROFILE="debug"
CARGO_PROFILE=""
CRATES=""
FEATURES=""
ALL_FEATURES=false
INSTALL_TOOLCHAIN=false
DEV_MODE=false
TAURI_MODE=false
BUNDLE_FORMATS=""
DOCKER_MODE=false

# ── Colors ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }

# ── Functions ──────────────────────────────────────────────────────────

show_help() {
    head -18 "$0" | tail -17 | sed 's/^# //' | sed 's/^#//'
}

list_targets() {
    echo "Available build targets:"
    echo ""
    echo "  native          Build for the current OS/architecture"
    for key in $ALL_TARGETS; do
        local rust_target
        rust_target="$(get_rust_target "$key" 2>/dev/null || echo "N/A")"
        printf "  %-16s %s\n" "$key" "$rust_target"
    done
    echo "  macos-universal Build both darwin targets and merge with lipo"
    echo "  all             Build for all targets (requires cross-compilation setup)"
    echo ""
    detect_os
    echo "Current platform: ${DETECTED_OS} (${DETECTED_ARCH})"
    echo ""
    echo "Available profiles:"
    echo "  debug           Default debug profile"
    echo "  release         Full LTO, opt-level=z, codegen-units=1"
    echo "  ci-release      Thin LTO, opt-level=s, codegen-units=16 (faster CI builds)"
    echo "  release-fast    Thin LTO with debug info (profiling)"
}

detect_os() {
    DETECTED_OS="unknown"
    DETECTED_ARCH="$(uname -m)"

    case "$(uname -s)" in
        Linux*)  DETECTED_OS="linux";;
        Darwin*) DETECTED_OS="macos";;
        MINGW*|MSYS*|CYGWIN*) DETECTED_OS="windows";;
    esac
}

get_native_target() {
    rustc -vV | grep host | awk '{print $2}'
}

check_toolchain() {
    local target="$1"

    if ! rustup target list --installed | grep -q "$target"; then
        if [ "$INSTALL_TOOLCHAIN" = true ]; then
            info "Installing toolchain for $target..."
            rustup target add "$target"
        else
            err "Rust target '$target' is not installed."
            echo "  Install it with: rustup target add $target"
            echo "  Or re-run with: $0 --install-toolchain --target ..."
            exit 1
        fi
    fi
}

check_cross_compiler() {
    local target="$1"

    case "$target" in
        "aarch64-unknown-linux-gnu"|"aarch64-unknown-linux-musl")
            if ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
                warn "Cross-compiler 'aarch64-linux-gnu-gcc' not found."
                echo "  Install with: sudo apt install gcc-aarch64-linux-gnu"
                return 1
            fi
            ;;
        "armv7-unknown-linux-gnueabihf")
            if ! command -v arm-linux-gnueabihf-gcc &> /dev/null; then
                warn "Cross-compiler 'arm-linux-gnueabihf-gcc' not found."
                echo "  Install with: sudo apt install gcc-arm-linux-gnueabihf"
                return 1
            fi
            ;;
        "x86_64-pc-windows-gnu")
            if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
                warn "Cross-compiler 'x86_64-w64-mingw32-gcc' not found."
                echo "  Install with: sudo apt install gcc-mingw-w64-x86-64"
                return 1
            fi
            ;;
    esac
    return 0
}

# Resolve the effective Cargo profile flag for the build command
get_cargo_profile_args() {
    local profile="$1"
    case "$profile" in
        debug)
            # No extra flags needed for debug
            ;;
        release)
            echo "--release"
            ;;
        ci-release|release-fast)
            echo "--profile" "$profile"
            ;;
        *)
            err "Unknown profile: $profile"
            exit 1
            ;;
    esac
}

# Get the target directory name for a profile
get_profile_dir() {
    local profile="$1"
    case "$profile" in
        debug)         echo "debug";;
        release)       echo "release";;
        ci-release)    echo "ci-release";;
        release-fast)  echo "release-fast";;
        *)             echo "$profile";;
    esac
}

build_target() {
    local rust_target="$1"
    local friendly_name="$2"

    info "Building for $friendly_name ($rust_target) [profile: $PROFILE]..."

    # Check toolchain
    if [ "$rust_target" != "$(get_native_target)" ]; then
        check_toolchain "$rust_target"
        if ! check_cross_compiler "$rust_target"; then
            warn "Skipping $friendly_name -- missing cross-compiler"
            return 1
        fi
    fi

    # Determine output directory
    local profile_dir
    profile_dir="$(get_profile_dir "$PROFILE")"
    local output_dir="${BUILD_DIR}/${friendly_name}/${profile_dir}"
    mkdir -p "$output_dir"

    # Build each crate
    local crates_to_build
    if [ -n "$CRATES" ]; then
        crates_to_build="$CRATES"
    else
        crates_to_build="$ALL_CRATES"
    fi

    for crate in $crates_to_build; do
        # Skip desktop crate for non-native cross-compilation (needs Tauri setup)
        if [ "$crate" = "mesoclaw-desktop" ] && [ "$rust_target" != "$(get_native_target)" ]; then
            warn "Skipping $crate for cross-compilation (requires Tauri platform setup)"
            continue
        fi

        info "  Building $crate..."

        local cargo_args=("-p" "$crate")

        # Profile flags
        local profile_args
        profile_args="$(get_cargo_profile_args "$PROFILE")"
        if [ -n "$profile_args" ]; then
            # shellcheck disable=SC2086
            cargo_args+=($profile_args)
        fi

        # Target (skip for native)
        if [ "$rust_target" != "$(get_native_target)" ]; then
            cargo_args+=("--target" "$rust_target")
        fi

        # Features
        if [ "$ALL_FEATURES" = true ]; then
            cargo_args+=("--all-features")
        elif [ -n "$FEATURES" ]; then
            cargo_args+=("--features" "$FEATURES")
        fi

        if cargo build "${cargo_args[@]}"; then
            # Copy binary to dist
            local bin_name
            bin_name=$(get_bin_name "$crate" "$rust_target")

            local src_path
            if [ "$rust_target" = "$(get_native_target)" ]; then
                src_path="${WORKSPACE_ROOT}/target/${profile_dir}/${bin_name}"
            else
                src_path="${WORKSPACE_ROOT}/target/${rust_target}/${profile_dir}/${bin_name}"
            fi

            if [ -f "$src_path" ]; then
                cp "$src_path" "$output_dir/"
                ok "  $crate -> dist/${friendly_name}/${profile_dir}/${bin_name}"
            fi
        else
            err "  Failed to build $crate for $friendly_name"
            return 1
        fi
    done

    ok "Build complete for $friendly_name"
}

build_macos_universal() {
    detect_os
    if [ "$DETECTED_OS" != "macos" ]; then
        err "macOS universal builds can only run on macOS (detected: $DETECTED_OS)"
        exit 1
    fi

    if ! command -v lipo &> /dev/null; then
        err "'lipo' command not found. It should be available on macOS."
        exit 1
    fi

    step "Building macOS universal binary (x86_64 + aarch64)..."

    # Build both architectures
    info "Building x86_64-apple-darwin..."
    build_target "x86_64-apple-darwin" "macos-x86"

    info "Building aarch64-apple-darwin..."
    build_target "aarch64-apple-darwin" "macos-arm"

    # Merge with lipo
    local profile_dir
    profile_dir="$(get_profile_dir "$PROFILE")"
    local universal_dir="${BUILD_DIR}/macos-universal/${profile_dir}"
    mkdir -p "$universal_dir"

    local crates_to_build
    if [ -n "$CRATES" ]; then
        crates_to_build="$CRATES"
    else
        crates_to_build="$ALL_CRATES"
    fi

    for crate in $crates_to_build; do
        # Skip desktop for cross-compilation
        if [ "$crate" = "mesoclaw-desktop" ]; then
            warn "Skipping $crate for universal build (requires Tauri platform setup)"
            continue
        fi

        local bin_name
        bin_name=$(get_bin_name "$crate" "x86_64-apple-darwin")

        local x86_bin="${BUILD_DIR}/macos-x86/${profile_dir}/${bin_name}"
        local arm_bin="${BUILD_DIR}/macos-arm/${profile_dir}/${bin_name}"
        local universal_bin="${universal_dir}/${bin_name}"

        if [ -f "$x86_bin" ] && [ -f "$arm_bin" ]; then
            info "  Merging $bin_name with lipo..."
            lipo -create -output "$universal_bin" "$x86_bin" "$arm_bin"
            ok "  $bin_name -> dist/macos-universal/${profile_dir}/${bin_name}"
        else
            warn "  Skipping $bin_name -- missing one or both architectures"
        fi
    done

    ok "macOS universal build complete"
}

build_via_docker() {
    local target="$1"
    local friendly_name="$2"

    info "Building via Docker for $friendly_name..."

    # Delegate to docker-build.sh
    local docker_script="${WORKSPACE_ROOT}/scripts/docker-build.sh"
    if [ ! -x "$docker_script" ]; then
        err "Docker build script not found or not executable: $docker_script"
        echo "  Make it executable with: chmod +x $docker_script"
        exit 1
    fi

    local docker_args=("--target" "$friendly_name")

    if [ "$PROFILE" != "debug" ]; then
        docker_args+=("--profile" "$PROFILE")
    fi

    if [ -n "$CRATES" ]; then
        docker_args+=("--crates" "$CRATES")
    fi

    if [ "$ALL_FEATURES" = true ]; then
        docker_args+=("--all-features")
    elif [ -n "$FEATURES" ]; then
        docker_args+=("--features" "$FEATURES")
    fi

    "$docker_script" "${docker_args[@]}"
}

get_bin_name() {
    local crate="$1"
    local target="$2"

    local name
    case "$crate" in
        "mesoclaw-cli")     name="mesoclaw";;
        "mesoclaw-tui")     name="mesoclaw-tui";;
        "mesoclaw-daemon")  name="mesoclaw-daemon";;
        "mesoclaw-desktop") name="mesoclaw-desktop";;
        *)                  name="$crate";;
    esac

    # Add .exe for Windows targets
    if [[ "$target" == *"windows"* ]]; then
        name="${name}.exe"
    fi

    echo "$name"
}

# ── Parse Arguments ────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --release)
            PROFILE="release"
            shift
            ;;
        --profile)
            CARGO_PROFILE="$2"
            PROFILE="$2"
            shift 2
            ;;
        --crates)
            CRATES="$2"
            shift 2
            ;;
        --features)
            FEATURES="$2"
            shift 2
            ;;
        --all-features)
            ALL_FEATURES=true
            shift
            ;;
        --tauri)
            TAURI_MODE=true
            shift
            ;;
        --bundle)
            BUNDLE_FORMATS="$2"
            shift 2
            ;;
        --dev)
            DEV_MODE=true
            shift
            ;;
        --docker)
            DOCKER_MODE=true
            shift
            ;;
        --install-toolchain)
            INSTALL_TOOLCHAIN=true
            shift
            ;;
        --list-targets)
            list_targets
            exit 0
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            err "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

run_tauri_build() {
    info "Building Tauri desktop app..."

    # Tauri cannot cross-compile — must build on the target OS
    if [ "$TARGET" != "native" ]; then
        err "Tauri builds only support --target native."
        echo "  Cross-platform Tauri builds require CI/CD (GitHub Actions)."
        echo "  Each platform (Linux/macOS/Windows) must build on its native runner."
        exit 1
    fi

    # Check cargo-tauri is installed
    if ! cargo tauri --version &> /dev/null; then
        err "cargo-tauri CLI is not installed."
        echo "  Install it with: cargo install tauri-cli"
        exit 1
    fi

    # Build frontend first
    if ! command -v bun &> /dev/null; then
        err "bun is not installed. Install it from https://bun.sh"
        exit 1
    fi

    info "Building frontend assets..."
    (cd "${WORKSPACE_ROOT}/web" && bun install && bun run build)

    # Assemble cargo tauri build args
    local tauri_args=()

    if [ "$PROFILE" = "debug" ]; then
        tauri_args+=("--debug")
    fi

    if [ "$ALL_FEATURES" = true ]; then
        tauri_args+=("--" "--all-features")
    elif [ -n "$FEATURES" ]; then
        tauri_args+=("--" "--features" "$FEATURES")
    fi

    # Bundle format selection
    if [ -n "$BUNDLE_FORMATS" ]; then
        # Convert comma-separated to space-separated for multiple --bundles flags
        IFS=',' read -ra FORMATS <<< "$BUNDLE_FORMATS"
        for fmt in "${FORMATS[@]}"; do
            tauri_args=("--bundles" "$fmt" "${tauri_args[@]}")
        done
    fi

    info "Running: cargo tauri build ${tauri_args[*]}"
    (cd "${WORKSPACE_ROOT}/crates/mesoclaw-desktop" && cargo tauri build "${tauri_args[@]}")

    if [ $? -eq 0 ]; then
        ok "Tauri build complete!"
        echo ""
        info "Bundle outputs:"
        local bundle_dir="${WORKSPACE_ROOT}/target/release/bundle"
        if [ -d "$bundle_dir" ]; then
            find "$bundle_dir" -type f \( -name "*.deb" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.app" -o -name "*.msi" -o -name "*.exe" -o -name "*.rpm" \) 2>/dev/null | sort | while read -r f; do
                local_path="${f#"$WORKSPACE_ROOT"/}"
                size=$(du -h "$f" | awk '{print $1}')
                echo "  $local_path ($size)"
            done
        fi
        if [ "$PROFILE" = "debug" ]; then
            bundle_dir="${WORKSPACE_ROOT}/target/debug/bundle"
            if [ -d "$bundle_dir" ]; then
                find "$bundle_dir" -type f \( -name "*.deb" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.app" -o -name "*.msi" -o -name "*.exe" -o -name "*.rpm" \) 2>/dev/null | sort | while read -r f; do
                    local_path="${f#"$WORKSPACE_ROOT"/}"
                    size=$(du -h "$f" | awk '{print $1}')
                    echo "  $local_path ($size)"
                done
            fi
        fi
    else
        err "Tauri build failed!"
        exit 1
    fi
}

run_dev() {
    info "Starting dev mode..."

    # Check bun is available
    if ! command -v bun &> /dev/null; then
        err "bun is not installed. Install it from https://bun.sh"
        exit 1
    fi

    # Install web dependencies if needed
    if [ ! -d "${WORKSPACE_ROOT}/web/node_modules" ]; then
        info "Installing web dependencies..."
        (cd "${WORKSPACE_ROOT}/web" && bun install)
    fi

    # Cleanup on exit/interrupt
    cleanup() {
        info "Shutting down..."
        kill "$VITE_PID" 2>/dev/null
        wait "$VITE_PID" 2>/dev/null
    }
    trap cleanup EXIT INT TERM

    # Kill any existing process on port 5173
    local existing_pid
    existing_pid=$(lsof -ti :5173 2>/dev/null || true)
    if [ -n "$existing_pid" ]; then
        warn "Port 5173 is in use (PID $existing_pid), killing it..."
        kill "$existing_pid" 2>/dev/null || true
        sleep 1
    fi

    # Start Vite dev server in background
    info "Starting Vite dev server on http://localhost:5173..."
    (cd "${WORKSPACE_ROOT}/web" && bun run dev) &
    VITE_PID=$!

    # Wait for Vite to be ready
    info "Waiting for Vite to start..."
    for i in $(seq 1 30); do
        if curl -s http://localhost:5173 > /dev/null 2>&1; then
            ok "Vite dev server is ready"
            break
        fi
        if [ "$i" -eq 30 ]; then
            err "Vite dev server failed to start within 30 seconds"
            kill "$VITE_PID" 2>/dev/null
            exit 1
        fi
        sleep 1
    done

    # Start Tauri dev without its own dev server (cleanup handled by trap)
    # --no-watch: prevent file watcher from interrupting the initial build
    info "Starting Tauri dev server..."
    (cd "${WORKSPACE_ROOT}/crates/mesoclaw-desktop" && cargo tauri dev --no-dev-server --no-watch)
    exit $?
}

# ── Main ───────────────────────────────────────────────────────────────

cd "$WORKSPACE_ROOT"
detect_os

# Handle special modes early
if [ "$DEV_MODE" = true ]; then
    run_dev
fi

if [ "$TAURI_MODE" = true ]; then
    run_tauri_build
    exit 0
fi

echo ""
echo "========================================"
echo "  MesoClaw Build"
echo "  Target:  $TARGET"
echo "  Profile: $PROFILE"
echo "  Crates:  ${CRATES:-all}"
if [ "$DOCKER_MODE" = true ]; then
echo "  Docker:  yes"
fi
echo "========================================"
echo ""

case "$TARGET" in
    "native")
        native_target="$(get_native_target)"
        if [ "$DOCKER_MODE" = true ]; then
            build_via_docker "$native_target" "native"
        else
            build_target "$native_target" "native"
        fi
        ;;
    "macos-universal")
        if [ "$DOCKER_MODE" = true ]; then
            err "macOS universal builds are not supported via Docker."
            exit 1
        fi
        build_macos_universal
        ;;
    "all")
        failed=0
        for friendly in $ALL_TARGETS; do
            if [ "$DOCKER_MODE" = true ]; then
                if ! build_via_docker "$(get_rust_target "$friendly")" "$friendly"; then
                    failed=$((failed + 1))
                fi
            else
                if ! build_target "$(get_rust_target "$friendly")" "$friendly"; then
                    failed=$((failed + 1))
                fi
            fi
            echo ""
        done
        if [ $failed -gt 0 ]; then
            warn "$failed target(s) failed or were skipped"
        fi
        ;;
    *)
        rust_target="$(get_rust_target "$TARGET")"
        if [ -n "$rust_target" ]; then
            if [ "$DOCKER_MODE" = true ]; then
                build_via_docker "$rust_target" "$TARGET"
            else
                build_target "$rust_target" "$TARGET"
            fi
        else
            err "Unknown target: $TARGET"
            echo "Use --list-targets to see available targets."
            exit 1
        fi
        ;;
esac

echo ""
info "Build artifacts are in: ${BUILD_DIR}/"
echo ""

# Show summary
if [ -d "$BUILD_DIR" ]; then
    info "Built binaries:"
    find "$BUILD_DIR" -type f -executable -o -name "*.exe" 2>/dev/null | sort | while read -r f; do
        local_path="${f#"$WORKSPACE_ROOT"/}"
        size=$(du -h "$f" | awk '{print $1}')
        echo "  $local_path ($size)"
    done
fi
