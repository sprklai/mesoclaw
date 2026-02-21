#!/bin/bash
#
# docker-build.sh - Build script for cross-platform Tauri builds using Docker
#
# This script enables building for platforms other than your host OS.
#
# Usage:
#   ./scripts/docker-build.sh --platforms windows,linux
#   ./scripts/docker-build.sh --all
#   ./scripts/docker-build.sh --windows --clean
#

set -e

# ---------------------------------------------------------------------------
# Colors
# ---------------------------------------------------------------------------
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
readonly BOLD='\033[1m'
readonly NC='\033[0m'

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCKER_IMAGE="tauri-cross-compile"
DOCKER_CONTAINER="tauri-build-$$"

# Build options
PLATFORMS=""
CLEAN_BUILD=""
BUILD_ARGS=""

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_step() {
    echo -e "${CYAN}${BOLD}==>${NC} ${BOLD}$*${NC}"
}

show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Build Tauri application for cross-platform targets using Docker.

Options:
  --platforms, -p PLATFORMS    Comma-separated platforms to build
                               Available: linux, windows, macos (via GitHub Actions)
  --all                        Build for all supported platforms
  --linux                      Build for all Linux targets
  --windows                    Build for all Windows targets
  --clean                      Clean build artifacts before building
  --verbose, -v                Enable verbose output
  --build-image                Rebuild the Docker image
  --help, -h                   Show this help message

Examples:
  # Build Windows binaries from Linux
  $0 --windows

  # Build Linux and Windows from macOS
  $0 --platforms linux,windows

  # Build all platforms (Linux + Windows)
  $0 --all

  # Clean build for Windows
  $0 --windows --clean

Notes:
  - macOS builds require macOS host or GitHub Actions
  - Windows MSVC builds require Windows host
  - Windows MinGW builds work from Linux via Docker

Docker usage:
  # Build Docker image once
  $0 --build-image

  # Then use docker run directly
  docker run --rm -v \$(pwd):/app -w /app $DOCKER_IMAGE ./scripts/cross-platform-build.sh --windows

EOF
}

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --platforms|-p)
                PLATFORMS="$2"
                shift 2
                ;;
            --all)
                PLATFORMS="linux,windows"
                shift
                ;;
            --linux)
                PLATFORMS="linux"
                shift
                ;;
            --windows)
                PLATFORMS="windows"
                shift
                ;;
            --macos)
                PLATFORMS="macos"
                shift
                ;;
            --clean)
                CLEAN_BUILD="--clean"
                shift
                ;;
            --verbose|-v)
                BUILD_ARGS="$BUILD_ARGS --verbose"
                shift
                ;;
            --build-image)
                BUILD_IMAGE=1
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# ---------------------------------------------------------------------------
# Build Docker image
# ---------------------------------------------------------------------------
build_docker_image() {
    log_step "Building Docker image: $DOCKER_IMAGE"

    docker build \
        -f "$SCRIPT_DIR/Dockerfile.cross-compile" \
        -t "$DOCKER_IMAGE" \
        "$PROJECT_ROOT"

    log_success "Docker image built successfully"
}

# ---------------------------------------------------------------------------
# Detect host platform
# ---------------------------------------------------------------------------
detect_host_platform() {
    local os="$(uname -s)"
    case "$os" in
        Linux)
            HOST_OS="linux"
            ;;
        Darwin)
            HOST_OS="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            HOST_OS="windows"
            ;;
        *)
            log_error "Unknown host OS: $os"
            exit 1
            ;;
    esac
    log_info "Host platform: $HOST_OS"
}

# ---------------------------------------------------------------------------
# Main execution
# ---------------------------------------------------------------------------
main() {
    parse_arguments "$@"
    detect_host_platform

    # Build image if requested or doesn't exist
    if [[ "$BUILD_IMAGE" == "1" ]] || ! docker image inspect "$DOCKER_IMAGE" &> /dev/null; then
        build_docker_image
        if [[ "$BUILD_IMAGE" == "1" ]]; then
            exit 0
        fi
    fi

    # Default to all platforms if none specified
    if [[ -z "$PLATFORMS" ]]; then
        log_info "No platforms specified, defaulting to: linux,windows"
        PLATFORMS="linux,windows"
    fi

    # Build the target list for the script
    local script_targets=""
    local platforms_array=()
    IFS=',' read -ra platforms_array <<< "$PLATFORMS"

    for platform in "${platforms_array[@]}"; do
        platform=$(echo "$platform" | tr -d ' ')
        case "$platform" in
            linux)
                script_targets="$script_targets --linux"
                ;;
            windows)
                script_targets="$script_targets --windows"
                ;;
            macos)
                if [[ "$HOST_OS" != "macos" ]]; then
                    log_error "macOS builds require macOS host or GitHub Actions"
                    log_info "Skipping macOS builds..."
                else
                    script_targets="$script_targets --macos"
                fi
                ;;
            *)
                log_error "Unknown platform: $platform"
                exit 1
                ;;
        esac
    done

    # Run build in Docker
    log_step "Starting cross-platform build in Docker..."
    log_info "Platforms: $PLATFORMS"

    # Ensure the script is executable in the container
    chmod +x "$SCRIPT_DIR/cross-platform-build.sh"

    # Run the build
    docker run --rm \
        -v "$PROJECT_ROOT:/app" \
        -w /app \
        -e CARGO_TERM_COLOR=always \
        -e CARGO_INCREMENTAL=0 \
        --name "$DOCKER_CONTAINER" \
        "$DOCKER_IMAGE" \
        ./scripts/cross-platform-build.sh $script_targets $CLEAN_BUILD $BUILD_ARGS

    # The cross-platform-build.sh script will display artifacts
    log_success "Docker build completed!"
}

main "$@"
