#!/usr/bin/env bash
# MesoClaw -- Cross-Platform Build Script
# Usage: ./scripts/build.sh [OPTIONS]
#
# Options:
#   --target <TARGET>     Build target (native|linux-x86|linux-arm|macos-x86|macos-arm|windows|all)
#   --release             Build in release mode (default: debug)
#   --crates <CRATES>     Space-separated list of crates to build (default: all binary crates)
#   --features <FEATURES> Comma-separated features to enable
#   --all-features        Enable all features
#   --list-targets        List available build targets and exit
#   --install-toolchain   Install the required Rust target toolchain
#   --dev                 Start dev mode (Vite + Tauri dev server)
#   --help                Show this help message

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────
WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${WORKSPACE_ROOT}/dist"

# All binary crates in the workspace
ALL_CRATES="mesoclaw-cli mesoclaw-tui mesoclaw-daemon mesoclaw-desktop"

# Target mappings: friendly name -> Rust triple
declare -A TARGET_MAP=(
    ["linux-x86"]="x86_64-unknown-linux-gnu"
    ["linux-arm"]="aarch64-unknown-linux-gnu"
    ["macos-x86"]="x86_64-apple-darwin"
    ["macos-arm"]="aarch64-apple-darwin"
    ["windows"]="x86_64-pc-windows-gnu"
)

# ── Defaults ───────────────────────────────────────────────────────────
TARGET="native"
PROFILE="debug"
CRATES=""
FEATURES=""
ALL_FEATURES=false
INSTALL_TOOLCHAIN=false
DEV_MODE=false

# ── Colors ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# ── Functions ──────────────────────────────────────────────────────────

show_help() {
    head -14 "$0" | tail -13 | sed 's/^# //' | sed 's/^#//'
}

list_targets() {
    echo "Available build targets:"
    echo ""
    echo "  native       Build for the current OS/architecture"
    for key in "${!TARGET_MAP[@]}"; do
        printf "  %-12s %s\n" "$key" "${TARGET_MAP[$key]}"
    done
    echo "  all          Build for all targets (requires cross-compilation setup)"
    echo ""
    detect_os
    echo "Current platform: ${DETECTED_OS} (${DETECTED_ARCH})"
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
        "aarch64-unknown-linux-gnu")
            if ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
                warn "Cross-compiler 'aarch64-linux-gnu-gcc' not found."
                echo "  Install with: sudo apt install gcc-aarch64-linux-gnu"
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

build_target() {
    local rust_target="$1"
    local friendly_name="$2"

    info "Building for $friendly_name ($rust_target)..."

    # Check toolchain
    if [ "$rust_target" != "$(get_native_target)" ]; then
        check_toolchain "$rust_target"
        if ! check_cross_compiler "$rust_target"; then
            warn "Skipping $friendly_name -- missing cross-compiler"
            return 1
        fi
    fi

    # Determine output directory
    local output_dir="${BUILD_DIR}/${friendly_name}/${PROFILE}"
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

        # Profile
        if [ "$PROFILE" = "release" ]; then
            cargo_args+=("--release")
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
                src_path="${WORKSPACE_ROOT}/target/${PROFILE}/${bin_name}"
            else
                src_path="${WORKSPACE_ROOT}/target/${rust_target}/${PROFILE}/${bin_name}"
            fi

            if [ -f "$src_path" ]; then
                cp "$src_path" "$output_dir/"
                ok "  $crate -> dist/${friendly_name}/${PROFILE}/${bin_name}"
            fi
        else
            err "  Failed to build $crate for $friendly_name"
            return 1
        fi
    done

    ok "Build complete for $friendly_name"
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
        --dev)
            DEV_MODE=true
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

# Handle dev mode early
if [ "$DEV_MODE" = true ]; then
    run_dev
fi

echo ""
echo "========================================"
echo "  MesoClaw Build"
echo "  Target:  $TARGET"
echo "  Profile: $PROFILE"
echo "  Crates:  ${CRATES:-all}"
echo "========================================"
echo ""

case "$TARGET" in
    "native")
        native_target="$(get_native_target)"
        build_target "$native_target" "native"
        ;;
    "all")
        failed=0
        for friendly in "${!TARGET_MAP[@]}"; do
            if ! build_target "${TARGET_MAP[$friendly]}" "$friendly"; then
                ((failed++))
            fi
            echo ""
        done
        if [ $failed -gt 0 ]; then
            warn "$failed target(s) failed or were skipped"
        fi
        ;;
    *)
        if [ -n "${TARGET_MAP[$TARGET]+x}" ]; then
            build_target "${TARGET_MAP[$TARGET]}" "$TARGET"
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
