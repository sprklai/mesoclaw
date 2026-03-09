#!/usr/bin/env bash
# MesoClaw -- Docker-Based Cross-Compilation Script
# Usage: ./scripts/docker-build.sh [OPTIONS]
#
# Options:
#   --target <TARGET>     Build target (linux-x86|linux-arm64|linux-armv7|windows)
#   --profile <PROFILE>   Cargo profile (release|ci-release|release-fast) (default: release)
#   --crates <CRATES>     Space-separated list of crates to build (default: cli tui daemon)
#   --features <FEATURES> Comma-separated features to enable
#   --all-features        Enable all features
#   --no-cache            Build Docker image without cache
#   --help                Show this help message

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────
WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOCKERFILE="${WORKSPACE_ROOT}/Dockerfile.cross-compile"
IMAGE_NAME="mesoclaw-cross"
DIST_DIR="${WORKSPACE_ROOT}/target/dist"

# ── Defaults ───────────────────────────────────────────────────────────
TARGET=""
PROFILE="release"
CRATES=""
FEATURES=""
ALL_FEATURES=false
NO_CACHE=false

# ── Colors ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }

# ── Functions ──────────────────────────────────────────────────────────

show_help() {
    head -12 "$0" | tail -11 | sed 's/^# //' | sed 's/^#//'
}

get_rust_target() {
    case "$1" in
        linux-x86)      echo "x86_64-unknown-linux-gnu";;
        linux-arm64)    echo "aarch64-unknown-linux-gnu";;
        linux-armv7)    echo "armv7-unknown-linux-gnueabihf";;
        windows)        echo "x86_64-pc-windows-gnu";;
        *)
            err "Unsupported Docker build target: $1"
            echo "  Supported: linux-x86, linux-arm64, linux-armv7, windows"
            exit 1
            ;;
    esac
}

get_docker_stage() {
    case "$1" in
        linux-x86)      echo "base";;
        linux-arm64)    echo "linux-cross";;
        linux-armv7)    echo "linux-cross";;
        windows)        echo "windows-cross";;
        *)              echo "universal";;
    esac
}

get_cargo_profile_args() {
    case "$1" in
        debug)                          ;;
        release)       echo "--release" ;;
        ci-release)    echo "--profile ci-release" ;;
        release-fast)  echo "--profile release-fast" ;;
        *)
            err "Unknown profile: $1"
            exit 1
            ;;
    esac
}

get_profile_dir() {
    case "$1" in
        debug)         echo "debug";;
        release)       echo "release";;
        ci-release)    echo "ci-release";;
        release-fast)  echo "release-fast";;
        *)             echo "$1";;
    esac
}

get_bin_name() {
    local crate="$1"
    local target="$2"

    local name
    case "$crate" in
        "mesoclaw-cli")     name="mesoclaw";;
        "mesoclaw-tui")     name="mesoclaw-tui";;
        "mesoclaw-daemon")  name="mesoclaw-daemon";;
        *)                  name="$crate";;
    esac

    if [[ "$target" == *"windows"* ]]; then
        name="${name}.exe"
    fi

    echo "$name"
}

build_docker_image() {
    local stage="$1"

    step "Building Docker image (stage: $stage)..."

    local docker_args=()
    docker_args+=("-f" "$DOCKERFILE")
    docker_args+=("--target" "$stage")
    docker_args+=("-t" "${IMAGE_NAME}:${stage}")

    if [ "$NO_CACHE" = true ]; then
        docker_args+=("--no-cache")
    fi

    docker build "${docker_args[@]}" "$WORKSPACE_ROOT"
    ok "Docker image built: ${IMAGE_NAME}:${stage}"
}

run_cargo_in_docker() {
    local stage="$1"
    local rust_target="$2"
    local friendly_name="$3"

    local profile_args
    profile_args="$(get_cargo_profile_args "$PROFILE")"

    local crates_to_build
    if [ -n "$CRATES" ]; then
        crates_to_build="$CRATES"
    else
        # Skip desktop in Docker (needs Tauri/GTK)
        crates_to_build="mesoclaw-cli mesoclaw-tui mesoclaw-daemon"
    fi

    local profile_dir
    profile_dir="$(get_profile_dir "$PROFILE")"
    local output_dir="${DIST_DIR}/${friendly_name}/${profile_dir}"
    mkdir -p "$output_dir"

    for crate in $crates_to_build; do
        if [ "$crate" = "mesoclaw-desktop" ]; then
            warn "Skipping $crate in Docker build (requires Tauri platform setup)"
            continue
        fi

        info "Building $crate for $friendly_name in Docker..."

        local cargo_cmd="cargo build -p $crate --target $rust_target"
        if [ -n "$profile_args" ]; then
            cargo_cmd="$cargo_cmd $profile_args"
        fi
        if [ "$ALL_FEATURES" = true ]; then
            cargo_cmd="$cargo_cmd --all-features"
        elif [ -n "$FEATURES" ]; then
            cargo_cmd="$cargo_cmd --features $FEATURES"
        fi

        docker run --rm \
            -v "${WORKSPACE_ROOT}:/workspace" \
            -w /workspace \
            "${IMAGE_NAME}:${stage}" \
            bash -c "$cargo_cmd"

        # Extract binary
        local bin_name
        bin_name="$(get_bin_name "$crate" "$rust_target")"
        local src_path="${WORKSPACE_ROOT}/target/${rust_target}/${profile_dir}/${bin_name}"

        if [ -f "$src_path" ]; then
            cp "$src_path" "$output_dir/"
            ok "  $crate -> target/dist/${friendly_name}/${profile_dir}/${bin_name}"
        else
            warn "  Binary not found at expected path: $src_path"
        fi
    done
}

# ── Parse Arguments ────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --profile)
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
        --no-cache)
            NO_CACHE=true
            shift
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

# ── Main ───────────────────────────────────────────────────────────────

if [ -z "$TARGET" ]; then
    err "No target specified. Use --target <TARGET>"
    echo "  Supported: linux-x86, linux-arm64, linux-armv7, windows"
    exit 1
fi

# Check Docker is available
if ! command -v docker &> /dev/null; then
    err "Docker is not installed or not in PATH."
    echo "  Install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

if ! docker info &> /dev/null 2>&1; then
    err "Docker daemon is not running."
    echo "  Start it with: sudo systemctl start docker"
    exit 1
fi

echo ""
echo "========================================"
echo "  MesoClaw Docker Build"
echo "  Target:  $TARGET"
echo "  Profile: $PROFILE"
echo "  Crates:  ${CRATES:-cli tui daemon}"
echo "========================================"
echo ""

rust_target="$(get_rust_target "$TARGET")"
docker_stage="$(get_docker_stage "$TARGET")"

build_docker_image "$docker_stage"
run_cargo_in_docker "$docker_stage" "$rust_target" "$TARGET"

echo ""
ok "Docker build complete!"
info "Binaries are in: ${DIST_DIR}/${TARGET}/"
echo ""

# Show built binaries
if [ -d "${DIST_DIR}/${TARGET}" ]; then
    info "Built binaries:"
    find "${DIST_DIR}/${TARGET}" -type f -executable -o -name "*.exe" 2>/dev/null | sort | while read -r f; do
        local_path="${f#"$WORKSPACE_ROOT"/}"
        size=$(du -h "$f" | awk '{print $1}')
        echo "  $local_path ($size)"
    done
fi
